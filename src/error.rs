// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum CaperError {
    // --- Errores del lexer ---
    #[error("Unexpected character '{ch}' at {line}:{col}")]
    UnexpectedChar { ch: char, line: usize, col: usize },

    #[error("Unterminated string literal at {line}:{col}")]
    UnterminatedString { line: usize, col: usize },

    // --- Errores del parser ---
    #[error("Syntax error at {line}:{col}: {detail}")]
    SyntaxError { line: usize, col: usize, detail: String },

    #[error("Expected '{expected}' at {line}:{col}, found '{found}'")]
    UnexpectedToken { expected: String, found: String, line: usize, col: usize },

    /// El programa anida o encadena más de lo que el motor puede recorrer sin
    /// desbordar la pila. Ver `limits` — incluye el tope que se excedió para que
    /// quien lo reciba sepa contra qué número está midiendo.
    #[error("Nesting too deep at line {line}: {depth} levels, limit is {limit}")]
    NestingTooDeep { line: usize, depth: usize, limit: usize },

    // --- Errores de runtime ---
    #[error("Undefined variable '{name}' at line {line}")]
    UndefinedVariable { name: String, line: usize },

    #[error("Type error at line {line}: {detail}")]
    TypeError { line: usize, detail: String },

    #[error("Division by zero at line {line}")]
    DivisionByZero { line: usize },

    #[error("Cannot reassign const '{name}' at line {line}")]
    ConstReassignment { name: String, line: usize },

    #[error("Cannot reassign immutable variable '{name}' at line {line}")]
    ImmutableReassignment { name: String, line: usize },

    #[error("Variable '{name}' already declared in this scope at line {line} — use 'var' if necesitas reasignar")]
    VariableRedeclaration { name: String, line: usize },

    #[error("Stack overflow: recursion limit ({limit}) exceeded")]
    StackOverflow { limit: usize },

    #[error("Loop limit exceeded: {limit} iterations")]
    LoopLimitExceeded { limit: usize },

    #[error("Script ended without emit")]
    EmitMissing,

    #[error("Import not found: '{path}'")]
    ImportNotFound { path: String },

    #[error("Circular import detected: '{path}'")]
    CircularImport { path: String },

    // --- Acceso a índice / campo ---
    #[error("Index out of bounds: index {index}, length {length}")]
    IndexOutOfBounds { index: i64, length: usize },

    #[error("Key not found: '{key}'")]
    KeyNotFound { key: String },

    // --- Errores de stdlib ---
    #[error("Argument error in '{func}': {detail}")]
    ArgumentError { func: String, detail: String },

    #[error("Invalid regex pattern '{pattern}': {detail}")]
    InvalidRegex { pattern: String, detail: String },

    #[error("JSON parse error: {detail}")]
    JsonParseError { detail: String },

    #[error("XML parse error: {detail}")]
    XmlParseError { detail: String },

    // --- Throw del usuario ---
    #[error("Uncaught exception at line {line} [{code}]: {message}")]
    UserThrow { line: usize, code: String, message: String },
}

impl CaperError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnexpectedChar { .. }        => "UNEXPECTED_CHAR",
            Self::UnterminatedString { .. }    => "UNTERMINATED_STRING",
            Self::SyntaxError { .. }           => "SYNTAX_ERROR",
            Self::UnexpectedToken { .. }       => "UNEXPECTED_TOKEN",
            Self::NestingTooDeep { .. }        => "NESTING_TOO_DEEP",
            Self::UndefinedVariable { .. }     => "UNDEFINED_VARIABLE",
            Self::TypeError { .. }             => "TYPE_ERROR",
            Self::DivisionByZero { .. }        => "DIVISION_BY_ZERO",
            Self::ConstReassignment { .. }     => "CONST_REASSIGNMENT",
            Self::ImmutableReassignment { .. }  => "IMMUTABLE_REASSIGNMENT",
            Self::VariableRedeclaration { .. }  => "VARIABLE_REDECLARATION",
            Self::StackOverflow { .. }         => "STACK_OVERFLOW",
            Self::LoopLimitExceeded { .. }     => "LOOP_LIMIT_EXCEEDED",
            Self::EmitMissing                  => "EMIT_MISSING",
            Self::ImportNotFound { .. }        => "IMPORT_NOT_FOUND",
            Self::CircularImport { .. }        => "CIRCULAR_IMPORT",
            Self::IndexOutOfBounds { .. }      => "INDEX_OUT_OF_BOUNDS",
            Self::KeyNotFound { .. }           => "KEY_NOT_FOUND",
            Self::ArgumentError { .. }         => "ARGUMENT_ERROR",
            Self::InvalidRegex { .. }          => "INVALID_REGEX",
            Self::JsonParseError { .. }        => "JSON_PARSE_ERROR",
            Self::XmlParseError { .. }         => "XML_PARSE_ERROR",
            Self::UserThrow { .. }             => "USER_THROW",
        }
    }

    /// Retorna (línea, columna) cuando el error tiene información de posición.
    /// Los errores de runtime retornan (línea, 0) — columna no disponible en tiempo de ejecución.
    pub fn position(&self) -> Option<(usize, usize)> {
        match self {
            Self::UnexpectedChar  { line, col, .. } => Some((*line, *col)),
            Self::UnterminatedString { line, col }  => Some((*line, *col)),
            Self::SyntaxError     { line, col, .. } => Some((*line, *col)),
            Self::UnexpectedToken { line, col, .. } => Some((*line, *col)),
            Self::NestingTooDeep  { line, .. }      => Some((*line, 0)),
            Self::UndefinedVariable { line, .. }    => Some((*line, 0)),
            Self::TypeError         { line, .. }    => Some((*line, 0)),
            Self::DivisionByZero    { line }        => Some((*line, 0)),
            Self::ConstReassignment { line, .. }    => Some((*line, 0)),
            Self::ImmutableReassignment { line, .. } => Some((*line, 0)),
            Self::VariableRedeclaration { line, .. } => Some((*line, 0)),
            Self::UserThrow         { line, .. }    => Some((*line, 0)),
            _ => None,
        }
    }

    /// Formatea un mensaje de diagnóstico enriquecido, opcionalmente con contexto fuente.
    ///
    /// La salida luce así:
    /// ```text
    /// error[SYNTAX_ERROR]: Syntax error at 5:12: expected ';'
    ///   --> script.caper:5:12
    ///    |
    ///  5 |     let x = foo(bar
    ///    |                 ^
    ///    |
    /// ```
    ///
    /// Si `source` es `None`, sólo se emite la primera línea (sin contexto).
    pub fn format_diagnostic(&self, filename: &str, source: Option<&str>) -> String {
        let header = format!("error[{}]: {}", self.code(), self);

        let Some((line, col)) = self.position() else {
            return header;
        };

        let loc = if col > 0 {
            format!("{}:{}:{}", filename, line, col)
        } else {
            format!("{}:{}", filename, line)
        };

        let Some(src) = source else {
            return format!("{}\n  --> {}", header, loc);
        };

        // Extraer la línea fuente (indexada desde 1)
        let src_line = src.lines().nth(line.saturating_sub(1)).unwrap_or("");
        let line_no  = line.to_string();
        let pad      = " ".repeat(line_no.len());

        let caret_line = if col > 0 {
            let caret_col = col.saturating_sub(1); // ajustar a columna visual con índice 0
            format!("{pad} | {}{}", " ".repeat(caret_col), "^")
        } else {
            format!("{pad} | ^")
        };

        format!(
            "{header}\n  --> {loc}\n{pad} |\n{line_no} | {src_line}\n{caret_line}\n{pad} |"
        )
    }
}
