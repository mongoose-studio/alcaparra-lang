// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use rust_decimal::Decimal;

/// Un script completo — ya sea una lista plana de sentencias o un programa estructurado header+main.
#[derive(Debug, Clone)]
pub struct Program {
    pub header: Option<Header>,
    pub body: Vec<Stmt>,
}

/// Metadatos y constantes del bloque `header { }`.
#[derive(Debug, Clone)]
pub struct Header {
    pub name:        Option<String>,
    pub version:     Option<String>,
    pub description: Option<String>,
    pub author:      Option<String>,
    pub constants:   Vec<ConstDecl>,
}

/// Un `const NOMBRE = valor;` dentro del header.
#[derive(Debug, Clone)]
pub struct ConstDecl {
    pub name:  String,
    pub value: Expr,
    pub line:  usize,
}

// ---------------------------------------------------------------------------
// Sentencias
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Stmt {
    /// `let nombre = expr;`
    Let { name: String, value: Expr, line: usize },

    /// `var nombre = expr;`
    Var { name: String, value: Expr, line: usize },

    /// `nombre = expr;`  o  `obj.campo = expr;`  o  `arr[idx] = expr;`
    Assign { target: AssignTarget, value: Expr, line: usize },

    /// `emit expr;`
    Emit { value: Expr, line: usize },

    /// `if cond { } else if cond { } else { }`
    If {
        condition: Expr,
        then_block: Vec<Stmt>,
        else_if_clauses: Vec<(Expr, Vec<Stmt>)>,
        else_block: Option<Vec<Stmt>>,
        line: usize,
    },

    /// `while (cond) { }`
    While { condition: Expr, body: Vec<Stmt>, line: usize },

    /// `foreach val in expr { }` o `foreach key, val in expr { }`
    Foreach {
        key:   Option<String>,
        value: String,
        iter:  Expr,
        body:  Vec<Stmt>,
        line:  usize,
    },

    /// `break;`
    Break { line: usize },

    /// `continue;`
    Continue { line: usize },

    /// `fn nombre(params) { }`
    FnDecl { name: String, params: Vec<String>, body: Vec<Stmt>, line: usize },

    /// `try { } catch e { }`
    TryCatch { try_block: Vec<Stmt>, error_var: String, catch_block: Vec<Stmt>, line: usize },

    /// `use std.math.round;`
    Use { path: Vec<String>, items: UseItems, line: usize },

    /// `throw expr;`
    Throw { value: Expr, line: usize },

    /// Una expresión usada como sentencia: `fn_call();`
    ExprStmt(Expr),
}

#[derive(Debug, Clone)]
pub enum UseItems {
    All,                    // `use std.math`
    Named(Vec<String>),     // `use std.math.{ abs, round }`
    Single,                 // `use std.math.round`  (el último segmento es el nombre de la fn)
}

#[derive(Debug, Clone)]
pub enum AssignTarget {
    Ident(String),
    Field { object: Box<Expr>, field: String },
    Index { object: Box<Expr>, index: Box<Expr> },
}

// ---------------------------------------------------------------------------
// Expresiones
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Expr {
    // Literales
    Number(Decimal),
    StringLit(String),
    Bool(bool),
    Null,

    // Variables
    Ident(String),

    // Aritmética & comparación
    BinOp { op: BinOp, left: Box<Expr>, right: Box<Expr>, line: usize },
    UnaryOp { op: UnaryOp, operand: Box<Expr>, line: usize },

    // Null-coalescing: `expr ?? fallback`
    NullCoalesce { left: Box<Expr>, right: Box<Expr>, line: usize },

    // Colecciones
    Array(Vec<ArrayElement>),
    Object(Vec<ObjectField>),

    // Acceso
    Index { object: Box<Expr>, index: Box<Expr>, line: usize },
    Field { object: Box<Expr>, field: String, line: usize },

    // Rango: `1..5`
    Range { start: Box<Expr>, end: Box<Expr>, line: usize },

    // Llamada a función
    Call { callee: Box<Expr>, args: Vec<Expr>, line: usize },

    // Closure: `|x, y| => expr`  o  `|x| => { stmts }`
    Closure { params: Vec<String>, body: ClosureBody, line: usize },

    // `if expr { } else { }` como expresión
    IfExpr {
        condition: Box<Expr>,
        then_block: Vec<Stmt>,
        else_block: Vec<Stmt>,
        line: usize,
    },

    // `match expr { patron => expr, ... }`
    Match { subject: Box<Expr>, arms: Vec<MatchArm>, line: usize },

    // `try { } catch e { }` como expresión
    TryCatchExpr {
        try_block:   Vec<Stmt>,
        error_var:   String,
        catch_block: Vec<Stmt>,
        line:        usize,
    },
}

#[derive(Debug, Clone)]
pub enum ArrayElement {
    Expr(Expr),
    Spread(Expr),   // `...otro_array`
}

#[derive(Debug, Clone)]
pub enum ObjectField {
    Named(String, Expr),  // `clave: valor`
    Spread(Expr),         // `...otro_obj`
}

#[derive(Debug, Clone)]
pub enum ClosureBody {
    Expr(Box<Expr>),
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: MatchPattern,
    pub guard:   Option<Box<Expr>>,   // `if condición`
    pub body:    Box<Expr>,
    pub line:    usize,
}

#[derive(Debug, Clone)]
pub enum MatchPattern {
    Wildcard,               // `_`
    Literal(Expr),          // `"string"`, `42`, `true`
    Binding(String),        // `n`  (captura el valor en una variable)
}

// ---------------------------------------------------------------------------
// Operadores
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod, Pow,
    Eq, NotEq, Lt, LtEq, Gt, GtEq,
    And, Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum UnaryOp {
    Neg,  // `-expr`
    Not,  // `not expr` (negación lógica)
}