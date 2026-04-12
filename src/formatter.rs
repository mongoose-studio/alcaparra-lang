// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Formateador de código basado en tokens para archivos `.caper`.
//!
//! Preserva comentarios y líneas en blanco (hasta `max_blank_lines`).
//! Aplica indentación consistente, espaciado alrededor de operadores y
//! comillas canónicas en strings.

use crate::config::FmtConfigRaw;
use crate::error::CaperError;
use crate::lexer::{FmtSpanned, Token};

// ---------------------------------------------------------------------------
// Configuración pública
// ---------------------------------------------------------------------------

/// Configuración del formateador.
#[derive(Debug, Clone)]
pub struct FmtConfig {
    /// Espacios por nivel de indentación. Por defecto: 4.
    pub indent_size: usize,
    /// Carácter de comillas para literales de string. Por defecto: `'"'`.
    pub quotes: char,
    /// Máximo de líneas en blanco consecutivas permitidas. Por defecto: 1.
    pub max_blank_lines: usize,
}

impl Default for FmtConfig {
    fn default() -> Self {
        Self { indent_size: 4, quotes: '"', max_blank_lines: 1 }
    }
}

impl FmtConfig {
    /// Construye la config a partir de los valores raw del `.capercfg`, completando los defaults.
    pub fn from_raw(raw: &FmtConfigRaw) -> Self {
        let quotes = match raw.quotes.as_deref() {
            Some("single") => '\'',
            _              => '"',
        };
        Self {
            indent_size:     raw.indent_size.unwrap_or(4),
            quotes,
            max_blank_lines: raw.max_blank_lines.unwrap_or(1),
        }
    }
}

// ---------------------------------------------------------------------------
// Punto de entrada público
// ---------------------------------------------------------------------------

/// Formatea un string fuente `.caper`.
///
/// Retorna el fuente reformateado, siempre terminando con un único salto de línea.
pub fn format(source: &str, cfg: &FmtConfig) -> Result<String, CaperError> {
    let tokens = crate::lexer::tokenize_with_comments(source)?;
    Ok(format_tokens(&tokens, cfg))
}

// ---------------------------------------------------------------------------
// Formateador interno
// ---------------------------------------------------------------------------

fn format_tokens(tokens: &[FmtSpanned], cfg: &FmtConfig) -> String {
    let mut out     = String::new();
    let mut indent  = 0usize;
    let mut blanks  = 0usize; // líneas en blanco consecutivas emitidas
    let mut prev: Option<Token> = None;   // último token real emitido
    let mut at_start = true;              // al inicio de una nueva línea
    // Stack de contexto: '{' = bloque/objeto, '(' = paréntesis, '[' = array
    let mut ctx: Vec<char> = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            FmtSpanned::BlankLine => {
                blanks += 1;
                if blanks <= cfg.max_blank_lines {
                    trim_trailing(&mut out);
                    if !out.ends_with('\n') {
                        out.push('\n'); // end the current line
                    }
                    out.push('\n'); // the blank line itself
                    at_start = true;
                }
            }

            FmtSpanned::LineComment { text, .. } => {
                blanks = 0; // reiniciar contador de líneas en blanco
                if !at_start {
                    // Comentario inline en la misma línea — un espacio antes
                    out.push_str("  //");
                } else {
                    push_indent(&mut out, indent, cfg.indent_size);
                    out.push_str("//");
                }
                out.push_str(text);
                out.push('\n');
                at_start = true;
                prev = None;
            }

            FmtSpanned::BlockComment { text, .. } => {
                blanks = 0; // reiniciar contador de líneas en blanco
                if !at_start { out.push('\n'); }
                push_indent(&mut out, indent, cfg.indent_size);
                out.push_str("/*");
                out.push_str(text);
                out.push_str("*/\n");
                at_start = true;
                prev = None;
            }

            FmtSpanned::Token(sp) => {
                blanks = 0;
                let tok = &sp.token;

                if matches!(tok, Token::Eof) {
                    break;
                }

                match tok {
                    // --------------------------------------------------------
                    // Llave de apertura — inicia un nuevo bloque indentado
                    // --------------------------------------------------------
                    Token::LBrace => {
                        if at_start {
                            push_indent(&mut out, indent, cfg.indent_size);
                        } else if !matches!(prev.as_ref(), Some(Token::Dot)) {
                            // Sin espacio cuando va precedido de `.` (ej. `use std.math.{`)
                            out.push(' ');
                        }
                        out.push('{');
                        indent += 1;
                        ctx.push('{');
                        out.push('\n');
                        at_start = true;
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Llave de cierre — termina el bloque indentado
                    // --------------------------------------------------------
                    Token::RBrace => {
                        if indent > 0 { indent -= 1; }
                        ctx.pop();
                        trim_trailing(&mut out);
                        // Asegurar exactamente un salto de línea antes de `}` (evitar doble en blanco)
                        if !out.ends_with('\n') {
                            out.push('\n');
                        }
                        push_indent(&mut out, indent, cfg.indent_size);
                        out.push('}');
                        at_start = false;
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Punto y coma — fin de sentencia, salto de línea
                    // --------------------------------------------------------
                    Token::Semicolon => {
                        out.push(';');
                        out.push('\n');
                        at_start = true;
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Coma — separador; salto de línea dentro de {}, espacio antes
                    // del siguiente token (via needs_space_before) en ()[]
                    // --------------------------------------------------------
                    Token::Comma => {
                        out.push(',');
                        if ctx.last() == Some(&'{') {
                            out.push('\n');
                            at_start = true;
                        }
                        // En contextos ()[] el espacio lo agrega needs_space_before
                        // para el siguiente token, así que no se empuja aquí.
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Punto — sin espacio a ningún lado
                    // --------------------------------------------------------
                    Token::Dot => {
                        if at_start {
                            push_indent(&mut out, indent, cfg.indent_size);
                            at_start = false;
                        }
                        out.push('.');
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Pipe — delimitador de closure, sin espacio
                    // --------------------------------------------------------
                    Token::Pipe => {
                        if at_start {
                            push_indent(&mut out, indent, cfg.indent_size);
                            at_start = false;
                        }
                        out.push('|');
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Rango / spread — sin espacio antes
                    // --------------------------------------------------------
                    Token::DotDot => {
                        if at_start { push_indent(&mut out, indent, cfg.indent_size); at_start = false; }
                        out.push_str("..");
                        prev = Some(tok.clone());
                    }
                    Token::DotDotDot => {
                        if at_start { push_indent(&mut out, indent, cfg.indent_size); at_start = false; }
                        else if needs_space_before(tok, prev.as_ref()) { out.push(' '); }
                        out.push_str("...");
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Paréntesis / corchete de apertura — tracking de contexto y reglas de espacio
                    // --------------------------------------------------------
                    Token::LParen => {
                        if at_start {
                            push_indent(&mut out, indent, cfg.indent_size);
                            at_start = false;
                        } else if needs_space_before(tok, prev.as_ref()) {
                            out.push(' ');
                        }
                        out.push('(');
                        ctx.push('(');
                        prev = Some(tok.clone());
                    }
                    Token::RParen => {
                        ctx.pop();
                        if at_start { push_indent(&mut out, indent, cfg.indent_size); at_start = false; }
                        out.push(')');
                        prev = Some(tok.clone());
                    }
                    Token::LBracket => {
                        if at_start {
                            push_indent(&mut out, indent, cfg.indent_size);
                            at_start = false;
                        } else if needs_space_before(tok, prev.as_ref()) {
                            out.push(' ');
                        }
                        out.push('[');
                        ctx.push('[');
                        prev = Some(tok.clone());
                    }
                    Token::RBracket => {
                        ctx.pop();
                        if at_start { push_indent(&mut out, indent, cfg.indent_size); at_start = false; }
                        out.push(']');
                        prev = Some(tok.clone());
                    }

                    // --------------------------------------------------------
                    // Todos los demás tokens — regla general de espaciado
                    // --------------------------------------------------------
                    _ => {
                        let s = token_to_str(tok, cfg);
                        if at_start {
                            push_indent(&mut out, indent, cfg.indent_size);
                            at_start = false;
                        } else if needs_space_before(tok, prev.as_ref()) {
                            out.push(' ');
                        }
                        out.push_str(&s);
                        prev = Some(tok.clone());
                    }
                }
            }
        }
        i += 1;
    }

    // Un único salto de línea al final
    let s = out.trim_end_matches(|c: char| c == ' ' || c == '\t');
    let s = s.trim_end_matches('\n');
    format!("{s}\n")
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

/// Agrega `indent_size * level` espacios a `out`.
fn push_indent(out: &mut String, level: usize, size: usize) {
    for _ in 0..level * size {
        out.push(' ');
    }
}

/// Elimina espacios/tabulaciones al final de la última línea de `out`.
fn trim_trailing(out: &mut String) {
    let trim_len = out.trim_end_matches(|c: char| c == ' ' || c == '\t').len();
    out.truncate(trim_len);
}

/// Determina si se necesita un espacio antes de `tok` dado `prev`.
fn needs_space_before(tok: &Token, prev: Option<&Token>) -> bool {
    match tok {
        // Nunca espacio antes de delimitadores / tokens postfijos
        Token::Semicolon | Token::Comma | Token::Dot | Token::RParen |
        Token::RBracket  | Token::Pipe  | Token::DotDot | Token::DotDotDot |
        Token::Colon => false,

        // LParen: espacio antes de keywords, no antes de identificadores (llamadas a función)
        Token::LParen => !matches!(prev, Some(
            Token::Ident(_) | Token::RParen | Token::RBracket
        )),

        // LBracket: sin espacio después de identificador/paréntesis/corchete (indexación)
        Token::LBracket => !matches!(prev, Some(
            Token::Ident(_) | Token::RParen | Token::RBracket
        )),

        // Sin espacio después de estos (ellos "absorben" el token siguiente)
        _ => !matches!(prev, Some(
            Token::LParen | Token::LBracket | Token::Dot | Token::Pipe | Token::Not
        )),
    }
}

/// Convierte un token a su representación canónica en código fuente.
fn token_to_str(tok: &Token, cfg: &FmtConfig) -> String {
    match tok {
        Token::Number(d)    => d.to_string(),
        Token::StringLit(s) => {
            let q = cfg.quotes;
            let mut escaped = String::with_capacity(s.len() + 2);
            for c in s.chars() {
                if c == '\\' { escaped.push_str("\\\\"); }
                else if c == q { escaped.push('\\'); escaped.push(q); }
                else if c == '\n' { escaped.push_str("\\n"); }
                else if c == '\t' { escaped.push_str("\\t"); }
                else if c == '\r' { escaped.push_str("\\r"); }
                else { escaped.push(c); }
            }
            format!("{q}{escaped}{q}")
        }
        Token::Bool(b)      => b.to_string(),
        Token::Null         => "null".to_string(),
        Token::Ident(s)     => s.clone(),
        Token::Let          => "let".to_string(),
        Token::Var          => "var".to_string(),
        Token::Const        => "const".to_string(),
        Token::Fn           => "fn".to_string(),
        Token::Emit         => "emit".to_string(),
        Token::If           => "if".to_string(),
        Token::Else         => "else".to_string(),
        Token::Match        => "match".to_string(),
        Token::While        => "while".to_string(),
        Token::Foreach      => "foreach".to_string(),
        Token::In           => "in".to_string(),
        Token::Break        => "break".to_string(),
        Token::Continue     => "continue".to_string(),
        Token::Try          => "try".to_string(),
        Token::Catch        => "catch".to_string(),
        Token::Use          => "use".to_string(),
        Token::Header       => "header".to_string(),
        Token::Main         => "main".to_string(),
        Token::And          => "and".to_string(),
        Token::Or           => "or".to_string(),
        Token::Not          => "not".to_string(),
        Token::Throw        => "throw".to_string(),
        Token::Return       => "return".to_string(),
        Token::Plus         => "+".to_string(),
        Token::Minus        => "-".to_string(),
        Token::Star         => "*".to_string(),
        Token::Slash        => "/".to_string(),
        Token::Percent      => "%".to_string(),
        Token::StarStar     => "**".to_string(),
        Token::Eq           => "==".to_string(),
        Token::NotEq        => "!=".to_string(),
        Token::Lt           => "<".to_string(),
        Token::LtEq         => "<=".to_string(),
        Token::Gt           => ">".to_string(),
        Token::GtEq         => ">=".to_string(),
        Token::Assign       => "=".to_string(),
        Token::FatArrow     => "=>".to_string(),
        Token::Arrow        => "->".to_string(),
        Token::Pipe         => "|".to_string(),
        Token::DotDot       => "..".to_string(),
        Token::DotDotDot    => "...".to_string(),
        Token::Question     => "?".to_string(),
        Token::QuestionQuestion => "??".to_string(),
        Token::LParen       => "(".to_string(),
        Token::RParen       => ")".to_string(),
        Token::LBrace       => "{".to_string(),
        Token::RBrace       => "}".to_string(),
        Token::LBracket     => "[".to_string(),
        Token::RBracket     => "]".to_string(),
        Token::Semicolon    => ";".to_string(),
        Token::Colon        => ":".to_string(),
        Token::Comma        => ",".to_string(),
        Token::Dot          => ".".to_string(),
        Token::Eof          => String::new(),
    }
}
