// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // --- Literales ---
    Number(rust_decimal::Decimal),
    StringLit(String),
    Bool(bool),
    Null,

    // --- Identificadores & keywords ---
    Ident(String),

    // Keywords (palabras clave)
    Let,
    Var,
    Const,
    Fn,
    Emit,
    If,
    Else,
    Match,
    While,
    Foreach,
    In,
    Break,
    Continue,
    Try,
    Catch,
    Use,
    Header,
    Main,
    And,
    Or,
    Not,
    Throw,
    Return, // reservado, aún no usado

    // --- Operadores ---
    Plus,          // +
    Minus,         // -
    Star,          // *
    Slash,         // /
    Percent,       // %
    StarStar,      // **
    Eq,            // ==
    NotEq,         // !=
    Lt,            // <
    LtEq,          // <=
    Gt,            // >
    GtEq,          // >=
    Assign,        // =
    FatArrow,      // =>
    Arrow,         // -> (reservado para futuras anotaciones de tipo)
    Pipe,          // |  (closure args delimiter)
    DotDot,        // .. (range)
    DotDotDot,     // ... (spread)
    Question,      // ?
    QuestionQuestion, // ??  (null coalescing)

    // --- Delimitadores ---
    LParen,        // (
    RParen,        // )
    LBrace,        // {
    RBrace,        // }
    LBracket,      // [
    RBracket,      // ]
    Semicolon,     // ;
    Colon,         // :
    Comma,         // ,
    Dot,           // .

    // --- Meta / fin de stream ---
    Eof,
}

impl Token {
    pub fn keyword_or_ident(s: &str) -> Token {
        match s {
            "let"      => Token::Let,
            "var"      => Token::Var,
            "const"    => Token::Const,
            "fn"       => Token::Fn,
            "emit"     => Token::Emit,
            "if"       => Token::If,
            "else"     => Token::Else,
            "match"    => Token::Match,
            "while"    => Token::While,
            "foreach"  => Token::Foreach,
            "in"       => Token::In,
            "break"    => Token::Break,
            "continue" => Token::Continue,
            "try"      => Token::Try,
            "catch"    => Token::Catch,
            "use"      => Token::Use,
            "header"   => Token::Header,
            "main"     => Token::Main,
            "and"      => Token::And,
            "or"       => Token::Or,
            "not"      => Token::Not,
            "throw"    => Token::Throw,
            "true"     => Token::Bool(true),
            "false"    => Token::Bool(false),
            "null"     => Token::Null,
            _          => Token::Ident(s.to_string()),
        }
    }
}

/// Un token con su posición en el fuente (ambos con índice base-1).
#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub line:  usize,
    pub col:   usize,
}

/// Elemento de token que también lleva comentarios y líneas en blanco — usado por el formateador.
#[derive(Debug, Clone)]
pub enum FmtSpanned {
    Token(Spanned),
    /// Comentario `// ...` (el texto es todo lo que sigue a `//`, sin salto de línea final).
    LineComment { text: String, line: usize },
    /// Comentario `/* ... */` (el texto es lo que hay entre los delimitadores).
    BlockComment { text: String, line: usize },
    /// Dos o más saltos de línea consecutivos (una línea en blanco).
    BlankLine,
}

// ---------------------------------------------------------------------------
// Position-tracking char iterator
// ---------------------------------------------------------------------------

struct PosChars<'src> {
    inner: std::iter::Peekable<std::str::Chars<'src>>,
    pub line: usize,
    pub col:  usize,
}

impl<'src> PosChars<'src> {
    fn new(s: &'src str) -> Self {
        Self { inner: s.chars().peekable(), line: 1, col: 1 }
    }

    fn peek(&mut self) -> Option<&char> {
        self.inner.peek()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.inner.next();
        match c {
            Some('\n') => { self.line += 1; self.col = 1; }
            Some(_)    => { self.col += 1; }
            None       => {}
        }
        c
    }

    /// Posición actual — snapshot ANTES de consumir el siguiente char.
    fn pos(&self) -> (usize, usize) { (self.line, self.col) }
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

/// Tokeniza un string fuente `.caper` en una secuencia de tokens `Spanned`.
///
/// Los comentarios y líneas en blanco se descartan. Para una variante que preserva
/// comentarios (usada por el formateador), ver [`tokenize_with_comments`].
pub fn tokenize(source: &str) -> Result<Vec<Spanned>, CaperError> {
    Ok(tokenize_with_comments(source)?
        .into_iter()
        .filter_map(|ft| match ft {
            FmtSpanned::Token(s) => Some(s),
            _ => None,
        })
        .collect())
}

/// Tokeniza un string fuente `.caper` preservando comentarios y líneas en blanco.
///
/// Retorna elementos `FmtSpanned` — usado por el formateador.
/// Para ejecución normal usar [`tokenize`] en su lugar.
pub fn tokenize_with_comments(source: &str) -> Result<Vec<FmtSpanned>, CaperError> {
    let mut tokens: Vec<FmtSpanned> = Vec::new();
    let mut chars  = PosChars::new(source);

    macro_rules! push {
        ($tok:expr, $line:expr, $col:expr) => {
            tokens.push(FmtSpanned::Token(Spanned { token: $tok, line: $line, col: $col }))
        };
    }

    while let Some(&ch) = chars.peek() {
        let (line, col) = chars.pos();

        match ch {
            // --- Espacios en blanco — detectar líneas vacías ---
            ' ' | '\t' | '\r' | '\n' => {
                let mut nl_count = if ch == '\n' { 1 } else { 0 };
                chars.next();
                while let Some(&c) = chars.peek() {
                    match c {
                        '\n' => { nl_count += 1; chars.next(); }
                        ' ' | '\t' | '\r' => { chars.next(); }
                        _ => break,
                    }
                }
                if nl_count >= 2 {
                    tokens.push(FmtSpanned::BlankLine);
                }
            }

            // --- Comentarios — preservados para el formateador ---
            '/' => {
                chars.next();
                match chars.peek() {
                    Some('/') => {
                        chars.next(); // consumir el segundo '/'
                        let mut text = String::new();
                        while let Some(&c) = chars.peek() {
                            if c == '\n' { break; }
                            text.push(c);
                            chars.next();
                        }
                        tokens.push(FmtSpanned::LineComment { text, line });
                    }
                    Some('*') => {
                        chars.next(); // consumir '*'
                        let mut text = String::new();
                        loop {
                            match chars.next() {
                                None => return Err(CaperError::UnterminatedString { line, col }),
                                Some('*') if chars.peek() == Some(&'/') => {
                                    chars.next();
                                    break;
                                }
                                Some(c) => text.push(c),
                            }
                        }
                        tokens.push(FmtSpanned::BlockComment { text, line });
                    }
                    _ => push!(Token::Slash, line, col),
                }
            }

            // --- Literales de string ---
            '"' | '\'' => {
                let quote = ch;
                chars.next();
                let mut s = String::new();
                loop {
                    match chars.next() {
                        None => return Err(CaperError::UnterminatedString { line, col }),
                        Some('\\') => {
                            match chars.next() {
                                Some('n')  => s.push('\n'),
                                Some('t')  => s.push('\t'),
                                Some('r')  => s.push('\r'),
                                Some('\\') => s.push('\\'),
                                Some(c) if c == quote => s.push(c),
                                Some(c) => { s.push('\\'); s.push(c); }
                                None    => return Err(CaperError::UnterminatedString { line, col }),
                            }
                        }
                        Some(c) if c == quote => break,
                        Some(c) => s.push(c),
                    }
                }
                push!(Token::StringLit(s), line, col);
            }

            // --- Números ---
            '0'..='9' => {
                let mut num = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_digit() || c == '.' || c == '_' {
                        if c != '_' { num.push(c); }
                        chars.next();
                    } else {
                        break;
                    }
                }
                let decimal = num.parse::<rust_decimal::Decimal>().map_err(|_| {
                    CaperError::SyntaxError { line, col, detail: format!("Invalid number '{num}'") }
                })?;
                push!(Token::Number(decimal), line, col);
            }

            // --- Identificadores & keywords ---
            c if c.is_alphabetic() || c == '_' => {
                let mut ident = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        ident.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                push!(Token::keyword_or_ident(&ident), line, col);
            }

            // --- Operadores de dos caracteres (el más largo primero) ---
            '*' => {
                chars.next();
                if chars.peek() == Some(&'*') { chars.next(); push!(Token::StarStar, line, col); }
                else { push!(Token::Star, line, col); }
            }
            '=' => {
                chars.next();
                if chars.peek() == Some(&'=') { chars.next(); push!(Token::Eq,       line, col); }
                else if chars.peek() == Some(&'>') { chars.next(); push!(Token::FatArrow, line, col); }
                else { push!(Token::Assign, line, col); }
            }
            '!' => {
                chars.next();
                if chars.peek() == Some(&'=') { chars.next(); push!(Token::NotEq, line, col); }
                else { return Err(CaperError::UnexpectedChar { ch: '!', line, col }); }
            }
            '<' => {
                chars.next();
                if chars.peek() == Some(&'=') { chars.next(); push!(Token::LtEq, line, col); }
                else { push!(Token::Lt, line, col); }
            }
            '>' => {
                chars.next();
                if chars.peek() == Some(&'=') { chars.next(); push!(Token::GtEq, line, col); }
                else { push!(Token::Gt, line, col); }
            }
            '?' => {
                chars.next();
                if chars.peek() == Some(&'?') { chars.next(); push!(Token::QuestionQuestion, line, col); }
                else { push!(Token::Question, line, col); }
            }
            '.' => {
                chars.next();
                if chars.peek() == Some(&'.') {
                    chars.next();
                    if chars.peek() == Some(&'.') { chars.next(); push!(Token::DotDotDot, line, col); }
                    else { push!(Token::DotDot, line, col); }
                } else {
                    push!(Token::Dot, line, col);
                }
            }

            // --- Tokens de un solo carácter ---
            '+' => { chars.next(); push!(Token::Plus,      line, col); }
            '-' => { chars.next(); push!(Token::Minus,     line, col); }
            '%' => { chars.next(); push!(Token::Percent,   line, col); }
            '(' => { chars.next(); push!(Token::LParen,    line, col); }
            ')' => { chars.next(); push!(Token::RParen,    line, col); }
            '{' => { chars.next(); push!(Token::LBrace,    line, col); }
            '}' => { chars.next(); push!(Token::RBrace,    line, col); }
            '[' => { chars.next(); push!(Token::LBracket,  line, col); }
            ']' => { chars.next(); push!(Token::RBracket,  line, col); }
            ';' => { chars.next(); push!(Token::Semicolon, line, col); }
            ':' => { chars.next(); push!(Token::Colon,     line, col); }
            ',' => { chars.next(); push!(Token::Comma,     line, col); }
            '|' => { chars.next(); push!(Token::Pipe,      line, col); }

            // --- Paquetes con scope y aliases de ruta: @nombre, @author/pkg-name ---
            // Formatos reconocidos:
            //   @alias          → alias de ruta clásico
            //   @author/pkg     → paquete de vendor con scope (sin guiones en author)
            //   @author/pkg-name → paquete de vendor con guiones en el nombre
            '@' => {
                chars.next();
                if chars.peek().map(|c| c.is_alphabetic() || *c == '_').unwrap_or(false) {
                    let mut ident = String::from("@");
                    // Parte author: letras, dígitos, _, -
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' || c == '-' { ident.push(c); chars.next(); }
                        else { break; }
                    }
                    // Parte pkg opcional: /pkg-name (incluye guiones)
                    if chars.peek() == Some(&'/') {
                        chars.next(); // consumir /
                        ident.push('/');
                        while let Some(&c) = chars.peek() {
                            if c.is_alphanumeric() || c == '_' || c == '-' { ident.push(c); chars.next(); }
                            else { break; }
                        }
                    }
                    push!(Token::Ident(ident), line, col);
                } else {
                    return Err(CaperError::UnexpectedChar { ch: '@', line, col });
                }
            }

            c => return Err(CaperError::UnexpectedChar { ch: c, line, col }),
        }
    }

    let (line, col) = chars.pos();
    push!(Token::Eof, line, col);
    Ok(tokens)
}