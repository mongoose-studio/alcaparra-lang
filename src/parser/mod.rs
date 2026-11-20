// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Parser recursivo descendente.
//!
//! Entrada:  `Vec<Spanned>` del lexer
//! Salida:   AST `Program`
//!
//! Gramática (informal):
//!
//! ```text
//! program     = use* header? stmt* main_block?
//! header      = "header" "{" header_field* "}"
//! header_field = ident ":" literal ";" | "const" ident "=" expr ";"
//! main_block  = "main" "{" stmt* "}"
//! stmt        = let_stmt | var_stmt | assign_stmt | emit_stmt
//!             | if_stmt | while_stmt | foreach_stmt
//!             | break_stmt | continue_stmt
//!             | fn_decl | try_catch_stmt | use_stmt
//!             | expr ";"
//! expr        = ... (ver parse_expr)
//! ```

pub mod ast;

use crate::error::CaperError;
use crate::lexer::{Spanned, Token};
use ast::*;

pub fn parse(tokens: Vec<Spanned>) -> Result<Program, CaperError> {
    let mut parser = Parser::new(tokens);
    parser.parse_program()
}

struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0 }
    }

    // --- Helpers de tokens ---

    fn peek(&self) -> &Token {
        &self.tokens[self.pos].token
    }

    /// Mira el token `offset` posiciones adelante sin avanzar.
    fn peek_ahead(&self, offset: usize) -> &Token {
        let idx = (self.pos + offset).min(self.tokens.len() - 1);
        &self.tokens[idx].token
    }

    fn peek_line(&self) -> usize { self.tokens[self.pos].line }
    fn peek_col(&self)  -> usize { self.tokens[self.pos].col  }

    fn advance(&mut self) -> &Token {
        let tok = &self.tokens[self.pos].token;
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn at(&self, tok: &Token) -> bool {
        self.peek() == tok
    }

    fn eat(&mut self, tok: &Token) -> Result<(), CaperError> {
        if self.peek() == tok {
            self.advance();
            Ok(())
        } else {
            Err(CaperError::UnexpectedToken {
                expected: format!("{tok:?}"),
                found:    format!("{:?}", self.peek()),
                line:     self.peek_line(),
                col:      self.peek_col(),
            })
        }
    }

    fn eat_ident(&mut self) -> Result<String, CaperError> {
        let (line, col) = (self.peek_line(), self.peek_col());
        match self.peek().clone() {
            Token::Ident(name) => { self.advance(); Ok(name) }
            other => Err(CaperError::UnexpectedToken {
                expected: "identifier".to_string(),
                found:    format!("{other:?}"),
                line,
                col,
            }),
        }
    }

    // --- Nivel superior ---

    fn parse_program(&mut self) -> Result<Program, CaperError> {
        let mut uses  = vec![];
        let mut header = None;

        // Recolectar sentencias `use` al inicio
        while self.at(&Token::Use) {
            uses.push(self.parse_use()?);
        }

        // `header { }` opcional
        if self.at(&Token::Header) {
            header = Some(self.parse_header()?);
        }

        // Sentencias previas a `main` — tipicamente declaraciones `fn` auxiliares.
        // Antes el programa era `main_block | stmt*`, excluyentes: declarar una
        // funcion arriba y despues abrir `main` fallaba con un error que apuntaba
        // a `main`, que es justo donde no estaba el problema.
        let mut body = self.parse_stmts_until(&Token::Main)?;

        // `main { }` opcional. Lo que venga antes se ejecuta antes de su cuerpo.
        if self.at(&Token::Main) {
            self.advance(); // consumir `main`
            self.eat(&Token::LBrace)?;
            body.extend(self.parse_stmts_until(&Token::RBrace)?);
            self.eat(&Token::RBrace)?;
        }

        // Anteponer las sentencias use como Stmt::Use
        let mut all_stmts = uses;
        all_stmts.extend(body);

        Ok(Program { header, body: all_stmts })
    }

    fn parse_header(&mut self) -> Result<Header, CaperError> {
        self.advance(); // consume `header`
        self.eat(&Token::LBrace)?;

        let mut name = None;
        let mut version = None;
        let mut description = None;
        let mut author = None;
        let mut constants = vec![];

        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            let line = self.peek_line();

            if self.at(&Token::Const) {
                self.advance();
                let const_name = self.eat_ident()?;
                self.eat(&Token::Assign)?;
                let value = self.parse_expr()?;
                self.eat(&Token::Semicolon)?;
                constants.push(ConstDecl { name: const_name, value, line });
                continue;
            }

            // `clave: valor;`
            let key = self.eat_ident()?;
            self.eat(&Token::Colon)?;
            let value_expr = self.parse_expr()?;
            self.eat(&Token::Semicolon)?;

            if let Expr::StringLit(s) = value_expr {
                match key.as_str() {
                    "name"        => name = Some(s),
                    "version"     => version = Some(s),
                    "description" => description = Some(s),
                    "author"      => author = Some(s),
                    _ => {} // campos desconocidos se ignoran silenciosamente
                }
            }
        }

        self.eat(&Token::RBrace)?;

        Ok(Header { name, version, description, author, constants })
    }

    fn parse_stmts_until(&mut self, stop: &Token) -> Result<Vec<Stmt>, CaperError> {
        let mut stmts = vec![];
        while self.peek() != stop && !self.at(&Token::Eof) {
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    // --- Sentencias ---

    fn parse_stmt(&mut self) -> Result<Stmt, CaperError> {
        self.parse_stmt_inner(false)
    }

    /// `allow_tail` habilita la **expresión final** de un bloque con valor: la
    /// última expresión puede venir sin `;` cuando lo que sigue es `}`. En
    /// cualquier otra posición el `;` sigue siendo obligatorio.
    fn parse_stmt_inner(&mut self, allow_tail: bool) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        let col  = self.peek_col();

        match self.peek().clone() {
            Token::Let      => self.parse_let(false),
            Token::Var      => self.parse_let(true),
            Token::Emit     => self.parse_emit(),
            // Un `if` o un `try` en la posición final de un bloque con valor son
            // expresiones, pero no se distinguen de su versión sentencia por el
            // primer token: se intenta como expresión y se retrocede si no calza.
            Token::If       => self.parse_if_or_tail(allow_tail),
            Token::While    => self.parse_while(),
            Token::Foreach  => self.parse_foreach(),
            Token::Break    => { self.advance(); self.eat(&Token::Semicolon)?; Ok(Stmt::Break { line }) }
            Token::Continue => { self.advance(); self.eat(&Token::Semicolon)?; Ok(Stmt::Continue { line }) }
            Token::Fn       => self.parse_fn_decl(),
            Token::Try      => self.parse_try_or_tail(allow_tail),
            Token::Use      => self.parse_use(),
            Token::Throw    => self.parse_throw(),
            _ => {
                // Sentencia de expresión o asignación
                let expr = self.parse_expr()?;

                // Verificar si es asignación: `destino = valor;`
                if self.at(&Token::Assign) {
                    self.advance();
                    let value = self.parse_expr()?;
                    self.eat(&Token::Semicolon)?;

                    let target = expr_to_assign_target(expr, line, col)?;
                    return Ok(Stmt::Assign { target, value, line });
                }

                // Expresión final de un bloque con valor: sin `;` y con `}` detrás.
                if allow_tail && self.at(&Token::RBrace) {
                    return Ok(Stmt::ExprStmt(expr));
                }

                self.eat(&Token::Semicolon)?;
                Ok(Stmt::ExprStmt(expr))
            }
        }
    }

    fn parse_if_or_tail(&mut self, allow_tail: bool) -> Result<Stmt, CaperError> {
        if allow_tail {
            let marca = self.pos;
            if let Ok(expr) = self.parse_if_expr() {
                if self.at(&Token::RBrace) {
                    return Ok(Stmt::ExprStmt(expr));
                }
            }
            self.pos = marca;
        }
        self.parse_if()
    }

    fn parse_try_or_tail(&mut self, allow_tail: bool) -> Result<Stmt, CaperError> {
        if allow_tail {
            let marca = self.pos;
            if let Ok(expr) = self.parse_try_catch_expr() {
                if self.at(&Token::RBrace) {
                    return Ok(Stmt::ExprStmt(expr));
                }
            }
            self.pos = marca;
        }
        self.parse_try_catch_stmt()
    }

    /// Cuerpo de un bloque que **tiene valor**. El valor es el de su expresión
    /// final; si no termina en una expresión, el bloque vale `null`.
    fn parse_block_expr(&mut self) -> Result<Vec<Stmt>, CaperError> {
        let mut stmts = vec![];
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            stmts.push(self.parse_stmt_inner(true)?);
        }
        Ok(stmts)
    }

    fn parse_let(&mut self, mutable: bool) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `let` o `var`
        let name = self.eat_ident()?;
        self.eat(&Token::Assign)?;
        let value = self.parse_expr()?;
        self.eat(&Token::Semicolon)?;
        if mutable {
            Ok(Stmt::Var { name, value, line })
        } else {
            Ok(Stmt::Let { name, value, line })
        }
    }

    fn parse_emit(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `emit`
        let value = self.parse_expr()?;
        self.eat(&Token::Semicolon)?;
        Ok(Stmt::Emit { value, line })
    }

    fn parse_if(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `if`
        let condition = self.parse_expr()?;
        self.eat(&Token::LBrace)?;
        let then_block = self.parse_stmts_until(&Token::RBrace)?;
        self.eat(&Token::RBrace)?;

        let mut else_if_clauses = vec![];
        let mut else_block = None;

        while self.at(&Token::Else) {
            self.advance();
            if self.at(&Token::If) {
                self.advance();
                let cond = self.parse_expr()?;
                self.eat(&Token::LBrace)?;
                let body = self.parse_stmts_until(&Token::RBrace)?;
                self.eat(&Token::RBrace)?;
                else_if_clauses.push((cond, body));
            } else {
                self.eat(&Token::LBrace)?;
                let body = self.parse_stmts_until(&Token::RBrace)?;
                self.eat(&Token::RBrace)?;
                else_block = Some(body);
                break;
            }
        }

        Ok(Stmt::If { condition, then_block, else_if_clauses, else_block, line })
    }

    fn parse_while(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance();
        self.eat(&Token::LParen)?;
        let condition = self.parse_expr()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_stmts_until(&Token::RBrace)?;
        self.eat(&Token::RBrace)?;
        Ok(Stmt::While { condition, body, line })
    }

    fn parse_foreach(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `foreach`
        let first = self.eat_ident()?;

        // `foreach clave, valor in expr` o `foreach valor in expr`
        let (key, value) = if self.at(&Token::Comma) {
            self.advance();
            let second = self.eat_ident()?;
            (Some(first), second)
        } else {
            (None, first)
        };

        self.eat(&Token::In)?;
        let iter = self.parse_expr()?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_stmts_until(&Token::RBrace)?;
        self.eat(&Token::RBrace)?;
        Ok(Stmt::Foreach { key, value, iter, body, line })
    }

    fn parse_fn_decl(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `fn`
        let name = self.eat_ident()?;
        self.eat(&Token::LParen)?;
        let params = self.parse_param_list()?;
        self.eat(&Token::RParen)?;
        self.eat(&Token::LBrace)?;
        let body = self.parse_stmts_until(&Token::RBrace)?;
        self.eat(&Token::RBrace)?;
        Ok(Stmt::FnDecl { name, params, body, line })
    }

    fn parse_param_list(&mut self) -> Result<Vec<String>, CaperError> {
        let mut params = vec![];
        while !self.at(&Token::RParen) && !self.at(&Token::Eof) {
            params.push(self.eat_ident()?);
            if self.at(&Token::Comma) { self.advance(); }
        }
        Ok(params)
    }

    fn parse_try_catch_stmt(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance();
        self.eat(&Token::LBrace)?;
        let try_block = self.parse_block_expr()?;
        self.eat(&Token::RBrace)?;
        self.eat(&Token::Catch)?;
        let error_var = self.eat_ident()?;
        self.eat(&Token::LBrace)?;
        let catch_block = self.parse_block_expr()?;
        self.eat(&Token::RBrace)?;
        Ok(Stmt::TryCatch { try_block, error_var, catch_block, line })
    }

    fn parse_throw(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `throw`
        let value = self.parse_expr()?;
        self.eat(&Token::Semicolon)?;
        Ok(Stmt::Throw { value, line })
    }

    fn parse_use(&mut self) -> Result<Stmt, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `use`

        // --- Primer segmento: puede ser uno de:
        //   std           → stdlib
        //   @alias        → alias de ruta (el lexer ya produce Ident("@alias"))
        //   @author/pkg   → paquete con scope (el lexer produce Ident("@author/pkg"))
        //   author/pkg    → paquete sin scope (lexer: Ident, Slash, Ident)
        //   ./relativo    → ruta relativa
        // Para `author/pkg` con guiones, el lexer produce Ident-Slash-Ident-Minus-Ident,
        // por lo que parse_pkg_segment() reconstruye el nombre con guiones.
        let first = self.eat_ident()?;
        let first_segment = if self.at(&Token::Slash) {
            // Formato author/pkg-name — consumir / y reconstruir el nombre del paquete
            self.advance(); // consumir /
            let pkg = self.parse_pkg_segment()?;
            format!("{first}/{pkg}")
        } else {
            first
        };

        let mut path = vec![first_segment];
        while self.at(&Token::Dot) {
            self.advance();
            // Verificar si hay destructuring `{ a, b }`
            if self.at(&Token::LBrace) {
                self.advance();
                let mut items = vec![];
                while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
                    items.push(self.eat_ident()?);
                    if self.at(&Token::Comma) { self.advance(); }
                }
                self.eat(&Token::RBrace)?;
                self.eat(&Token::Semicolon)?;
                return Ok(Stmt::Use { path, items: UseItems::Named(items), line });
            }
            path.push(self.eat_ident()?);
        }
        self.eat(&Token::Semicolon)?;
        Ok(Stmt::Use { path, items: UseItems::Single, line })
    }

    /// Lee un identificador de segmento de paquete que puede contener guiones.
    ///
    /// Reconstruye `formulas-cl` a partir de `Ident("formulas") Minus Ident("cl")`.
    /// Solo se usa en el contexto de nombres de paquete en sentencias `use`.
    fn parse_pkg_segment(&mut self) -> Result<String, CaperError> {
        let mut name = self.eat_ident()?;
        // Acumular partes `- ident` mientras el patrón sea Minus seguido de Ident
        while self.at(&Token::Minus) && matches!(self.peek_ahead(1), Token::Ident(_)) {
            self.advance(); // consumir -
            name.push('-');
            name.push_str(&self.eat_ident()?);
        }
        Ok(name)
    }

    // --- Expresiones (Pratt parser por precedencia) ---

    fn parse_expr(&mut self) -> Result<Expr, CaperError> {
        self.parse_null_coalesce()
    }

    fn parse_null_coalesce(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let mut left = self.parse_or()?;
        while self.at(&Token::QuestionQuestion) {
            self.advance();
            let right = self.parse_or()?;
            left = Expr::NullCoalesce { left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_or(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let mut left = self.parse_and()?;
        while self.at(&Token::Or) {
            self.advance();
            let right = self.parse_and()?;
            left = Expr::BinOp { op: BinOp::Or, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let mut left = self.parse_equality()?;
        while self.at(&Token::And) {
            self.advance();
            let right = self.parse_equality()?;
            left = Expr::BinOp { op: BinOp::And, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let mut left = self.parse_comparison()?;
        loop {
            let op = match self.peek() {
                Token::Eq    => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_comparison()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let mut left = self.parse_additive()?;
        loop {
            let op = match self.peek() {
                Token::Lt   => BinOp::Lt,
                Token::LtEq => BinOp::LtEq,
                Token::Gt   => BinOp::Gt,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            self.advance();
            let right = self.parse_additive()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_additive(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = match self.peek() {
                Token::Plus  => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            self.advance();
            let right = self.parse_multiplicative()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let mut left = self.parse_power()?;
        loop {
            let op = match self.peek() {
                Token::Star    => BinOp::Mul,
                Token::Slash   => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            self.advance();
            let right = self.parse_power()?;
            left = Expr::BinOp { op, left: Box::new(left), right: Box::new(right), line };
        }
        Ok(left)
    }

    fn parse_power(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let base = self.parse_unary()?;
        if self.at(&Token::StarStar) {
            self.advance();
            let exp = self.parse_power()?; // asociativo por la derecha
            return Ok(Expr::BinOp { op: BinOp::Pow, left: Box::new(base), right: Box::new(exp), line });
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        if self.at(&Token::Not) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp { op: UnaryOp::Not, operand: Box::new(operand), line });
        }
        if self.at(&Token::Minus) {
            self.advance();
            let operand = self.parse_unary()?;
            return Ok(Expr::UnaryOp { op: UnaryOp::Neg, operand: Box::new(operand), line });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, CaperError> {
        let mut expr = self.parse_primary()?;
        loop {
            let line = self.peek_line();
            match self.peek().clone() {
                Token::Dot => {
                    self.advance();
                    let field = self.eat_ident()?;
                    expr = Expr::Field { object: Box::new(expr), field, line };
                }
                Token::LBracket => {
                    self.advance();
                    let index = self.parse_expr()?;
                    self.eat(&Token::RBracket)?;
                    expr = Expr::Index { object: Box::new(expr), index: Box::new(index), line };
                }
                Token::LParen => {
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.eat(&Token::RParen)?;
                    expr = Expr::Call { callee: Box::new(expr), args, line };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, CaperError> {
        let mut args = vec![];
        while !self.at(&Token::RParen) && !self.at(&Token::Eof) {
            args.push(self.parse_expr()?);
            if self.at(&Token::Comma) { self.advance(); }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        let col  = self.peek_col();
        match self.peek().clone() {
            Token::Number(n)    => { self.advance(); Ok(Expr::Number(n)) }
            Token::StringLit(s) => { self.advance(); Ok(Expr::StringLit(s)) }
            Token::Bool(b)      => { self.advance(); Ok(Expr::Bool(b)) }
            Token::Null         => { self.advance(); Ok(Expr::Null) }
            Token::Ident(name)  => { self.advance(); Ok(Expr::Ident(name)) }

            Token::LParen => {
                self.advance();
                let expr = self.parse_expr()?;
                self.eat(&Token::RParen)?;
                Ok(expr)
            }

            Token::LBracket => self.parse_array(),

            Token::LBrace => self.parse_object(),

            Token::Pipe => self.parse_closure(),

            Token::If    => self.parse_if_expr(),
            Token::Match => self.parse_match_expr(),
            Token::Try   => self.parse_try_catch_expr(),

            other => Err(CaperError::UnexpectedToken {
                expected: "expression".to_string(),
                found:    format!("{other:?}"),
                line,
                col,
            }),
        }
    }

    fn parse_array(&mut self) -> Result<Expr, CaperError> {
        self.eat(&Token::LBracket)?;
        let mut elements = vec![];
        while !self.at(&Token::RBracket) && !self.at(&Token::Eof) {
            if self.at(&Token::DotDotDot) {
                self.advance();
                let expr = self.parse_expr()?;
                elements.push(ArrayElement::Spread(expr));
            } else {
                elements.push(ArrayElement::Expr(self.parse_expr()?));
            }
            if self.at(&Token::Comma) { self.advance(); }
        }
        self.eat(&Token::RBracket)?;
        Ok(Expr::Array(elements))
    }

    fn parse_object(&mut self) -> Result<Expr, CaperError> {
        self.eat(&Token::LBrace)?;
        let mut fields = vec![];
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            if self.at(&Token::DotDotDot) {
                self.advance();
                let expr = self.parse_expr()?;
                fields.push(ObjectField::Spread(expr));
            } else {
                let key = self.eat_ident()?;
                self.eat(&Token::Colon)?;
                let value = self.parse_expr()?;
                fields.push(ObjectField::Named(key, value));
            }
            if self.at(&Token::Comma) { self.advance(); }
        }
        self.eat(&Token::RBrace)?;
        Ok(Expr::Object(fields))
    }

    fn parse_closure(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        self.eat(&Token::Pipe)?;
        let mut params = vec![];
        while !self.at(&Token::Pipe) && !self.at(&Token::Eof) {
            params.push(self.eat_ident()?);
            if self.at(&Token::Comma) { self.advance(); }
        }
        self.eat(&Token::Pipe)?;
        self.eat(&Token::FatArrow)?;

        let body = if self.at(&Token::LBrace) {
            self.advance();
            let stmts = self.parse_stmts_until(&Token::RBrace)?;
            self.eat(&Token::RBrace)?;
            ClosureBody::Block(stmts)
        } else {
            ClosureBody::Expr(Box::new(self.parse_expr()?))
        };

        Ok(Expr::Closure { params, body, line })
    }

    fn parse_if_expr(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `if`
        let condition = self.parse_expr()?;
        self.eat(&Token::LBrace)?;
        let then_block = self.parse_block_expr()?;
        self.eat(&Token::RBrace)?;
        self.eat(&Token::Else)?;

        // `else if` encadena otra expresión `if` como rama else, para no
        // obligar a anidar llaves por cada tramo.
        let else_block = if self.at(&Token::If) {
            vec![Stmt::ExprStmt(self.parse_if_expr()?)]
        } else {
            self.eat(&Token::LBrace)?;
            let bloque = self.parse_block_expr()?;
            self.eat(&Token::RBrace)?;
            bloque
        };

        Ok(Expr::IfExpr {
            condition: Box::new(condition),
            then_block,
            else_block,
            line,
        })
    }

    fn parse_match_expr(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        self.advance(); // consumir `match`
        let subject = self.parse_expr()?;
        self.eat(&Token::LBrace)?;

        let mut arms = vec![];
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            let arm_line = self.peek_line();
            let pattern = self.parse_match_pattern()?;
            let guard = if self.at(&Token::If) {
                self.advance();
                Some(Box::new(self.parse_expr()?))
            } else {
                None
            };
            self.eat(&Token::FatArrow)?;
            let body = Box::new(self.parse_expr()?);
            arms.push(MatchArm { pattern, guard, body, line: arm_line });
            if self.at(&Token::Comma) { self.advance(); }
        }

        self.eat(&Token::RBrace)?;
        Ok(Expr::Match { subject: Box::new(subject), arms, line })
    }

    fn parse_match_pattern(&mut self) -> Result<MatchPattern, CaperError> {
        match self.peek().clone() {
            Token::Ident(name) if name == "_" => { self.advance(); Ok(MatchPattern::Wildcard) }
            Token::Ident(name) => { self.advance(); Ok(MatchPattern::Binding(name)) }
            Token::StringLit(_) | Token::Number(_) | Token::Bool(_) | Token::Null => {
                Ok(MatchPattern::Literal(self.parse_primary()?))
            }
            other => Err(CaperError::UnexpectedToken {
                expected: "match pattern".to_string(),
                found:    format!("{other:?}"),
                line:     self.peek_line(),
                col:      self.peek_col(),
            }),
        }
    }

    fn parse_try_catch_expr(&mut self) -> Result<Expr, CaperError> {
        let line = self.peek_line();
        self.advance();
        self.eat(&Token::LBrace)?;
        let try_block = self.parse_block_expr()?;
        self.eat(&Token::RBrace)?;
        self.eat(&Token::Catch)?;
        let error_var = self.eat_ident()?;
        self.eat(&Token::LBrace)?;
        let catch_block = self.parse_block_expr()?;
        self.eat(&Token::RBrace)?;
        Ok(Expr::TryCatchExpr { try_block, error_var, catch_block, line })
    }
}

// --- Helpers internos ---

fn expr_to_assign_target(expr: Expr, line: usize, col: usize) -> Result<AssignTarget, CaperError> {
    match expr {
        Expr::Ident(name) => Ok(AssignTarget::Ident(name)),
        Expr::Field { object, field, .. } => Ok(AssignTarget::Field { object, field }),
        Expr::Index { object, index, .. } => Ok(AssignTarget::Index { object, index }),
        _ => Err(CaperError::SyntaxError {
            line,
            col,
            detail: "Invalid assignment target".to_string(),
        }),
    }
}