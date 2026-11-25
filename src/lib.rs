// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! # AlcaparraLang
//!
//! Lenguaje de scripting interpretado con foco en cálculos numéricos.
//! Los archivos usan la extensión `.caper`.
//!
//! ## Inicio rápido
//!
//! ```rust
//! use alcaparra::{Interpreter, Value};
//! use std::collections::HashMap;
//! use rust_decimal_macros::dec;
//!
//! let mut interpreter = Interpreter::new();
//!
//! let source = r#"
//!     let base = SUELDO_BASE ?? 0;
//!     emit { liquido: base * 0.883 };
//! "#;
//!
//! let mut context = HashMap::new();
//! context.insert("SUELDO_BASE".to_string(), Value::number(dec!(850000)));
//!
//! let result = interpreter.run("ejemplo", source, context).unwrap();
//! println!("{:?}", result);
//! ```

pub mod compiler;
pub mod config;
pub mod error;
pub mod formatter;
pub mod global_config;
pub mod instruction;
pub mod lexer;
pub mod limits;
pub mod loader;
pub mod package;
pub mod parser;
pub mod plugin;
pub mod vm;
pub mod stdlib;

pub use error::CaperError;
pub use loader::ImportLoader;
pub use plugin::{CaperPlugin, PluginRegistry};
pub use vm::value::{NativeFn, Value};
// Re-exportados después de su definición más abajo
// LintDiagnostic y Severity se definen en este archivo

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// El punto de entrada principal para ejecutar scripts de AlcaparraLang.
pub struct Interpreter {
    loader:  Rc<RefCell<ImportLoader>>,
    plugins: Rc<RefCell<PluginRegistry>>,
}

impl Interpreter {
    /// Crea un nuevo intérprete usando el directorio de trabajo actual como
    /// base para imports de usuario. Sin aliases de ruta configurados.
    pub fn new() -> Self {
        Self {
            loader:  ImportLoader::default_loader(),
            plugins: Rc::new(RefCell::new(PluginRegistry::new())),
        }
    }

    /// Crea un intérprete con un directorio base específico y aliases de ruta
    /// opcionales (de la sección `paths` del `.capercfg`).
    pub fn with_loader(base_dir: PathBuf, aliases: HashMap<String, PathBuf>) -> Self {
        let loader = Rc::new(RefCell::new(ImportLoader::new(base_dir, aliases)));
        Self { loader, plugins: Rc::new(RefCell::new(PluginRegistry::new())) }
    }

    /// Registra un plugin Rust de primera parte.
    ///
    /// El `module_name()` del plugin se convierte en el namespace. Las funciones se
    /// resuelven en scripts después de la stdlib — es decir, los plugins pueden ser
    /// llamados por nombre si ninguna función de stdlib los sombrea.
    pub fn register_plugin(&mut self, plugin: Box<dyn CaperPlugin>) {
        self.plugins.borrow_mut().register(plugin);
    }

    /// Register a single native Rust function callable by `name` from scripts.
    ///
    /// ```rust,no_run
    /// use alcaparra::{Interpreter, Value};
    /// use rust_decimal_macros::dec;
    ///
    /// let mut interp = Interpreter::new();
    /// interp.register_fn("triple", |args, _line| {
    ///     if let Some(Value::Number(n)) = args.into_iter().next() {
    ///         Ok(Value::number(n * dec!(3)))
    ///     } else {
    ///         Ok(Value::Null)
    ///     }
    /// });
    /// ```
    pub fn register_fn<F>(&mut self, name: impl Into<String>, func: F)
    where
        F: Fn(Vec<Value>, usize) -> Result<Value, CaperError> + 'static,
    {
        let name = name.into();
        // Inyectar como valor NativeFunction en el scope base de cada ejecución futura
        // envolviéndolo en un plugin de una función.
        let nf = NativeFn { name: name.clone(), func: Rc::new(func) };
        self.plugins.borrow_mut().register(Box::new(NativeFnPlugin { name, nf }));
    }

    /// Carga un plugin dinámico desde una biblioteca compartida (`.so` / `.dll` / `.dylib`).
    ///
    /// Requiere el feature flag `dynamic_plugins`.
    #[cfg(feature = "dynamic_plugins")]
    pub fn load_plugin(&mut self, path: &std::path::Path) -> Result<String, CaperError> {
        plugin::dynamic::load_into(&mut self.plugins.borrow_mut(), path)
    }

    /// Ejecuta un script `.caper` con el contexto dado.
    ///
    /// Los valores de `context` se inyectan en el scope global (nivel 0).
    /// Las declaraciones `const` en `header {}` tienen precedencia sobre el contexto.
    pub fn run(
        &mut self,
        script_id: &str,
        source: &str,
        context: HashMap<String, Value>,
    ) -> Result<Value, CaperError> {
        let _ = script_id; // reservado para clave de caché
        let tokens  = lexer::tokenize(source)?;
        let program = parser::parse(tokens)?;
        let compiled = Rc::new(compiler::compile(program)?);
        let mut executor = vm::executor::Executor::with_plugins(
            Rc::clone(&compiled),
            context,
            Rc::clone(&self.loader),
            Rc::clone(&self.plugins),
        );
        executor.run()
    }

    /// Formatea un `CaperError` con contexto fuente enriquecido (salida estilo compilador).
    ///
    /// `filename` se usa en la línea de ubicación `-->`.
    /// `source` es el texto del script usado para el puntero de caret.
    pub fn format_diagnostic(err: &CaperError, filename: &str, source: &str) -> String {
        err.format_diagnostic(filename, Some(source))
    }

    /// Valida la sintaxis del script sin ejecutarlo. No requiere contexto.
    pub fn validate(&self, source: &str) -> Result<Vec<Warning>, Vec<CaperError>> {
        let tokens = match lexer::tokenize(source) {
            Ok(t) => t,
            Err(e) => return Err(vec![e]),
        };
        match parser::parse(tokens) {
            Ok(_) => Ok(vec![]),
            Err(e) => Err(vec![e]),
        }
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

/// Diagnóstico no fatal de validación.
#[derive(Debug, Clone)]
pub struct Warning {
    pub code: &'static str,
    pub detail: String,
    pub line: usize,
}

// ---------------------------------------------------------------------------
// Lint (análisis estático)
// ---------------------------------------------------------------------------

/// Severidad de un diagnóstico de lint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Error   => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
        }
    }
}

/// Diagnóstico de lint — error o advertencia con posición en el fuente.
#[derive(Debug, Clone)]
pub struct LintDiagnostic {
    pub severity: Severity,
    pub code:     &'static str,
    pub message:  String,
    pub line:     usize,
    pub col:      usize,
}

impl LintDiagnostic {
    fn error(code: &'static str, message: impl Into<String>, line: usize, col: usize) -> Self {
        Self { severity: Severity::Error, code, message: message.into(), line, col }
    }
    fn warning(code: &'static str, message: impl Into<String>, line: usize, col: usize) -> Self {
        Self { severity: Severity::Warning, code, message: message.into(), line, col }
    }

    /// Formatea como un diagnóstico estilo compilador, con contexto fuente opcional.
    pub fn format(&self, filename: &str, source: Option<&str>) -> String {
        let header = format!("{}[{}]: {}", self.severity, self.code, self.message);

        let loc = if self.col > 0 {
            format!("{}:{}:{}", filename, self.line, self.col)
        } else {
            format!("{}:{}", filename, self.line)
        };

        if self.line == 0 {
            return format!("{header}\n  --> {filename}");
        }

        let Some(src) = source else {
            return format!("{header}\n  --> {loc}");
        };

        let src_line = src.lines().nth(self.line.saturating_sub(1)).unwrap_or("");
        let line_no  = self.line.to_string();
        let pad      = " ".repeat(line_no.len());
        let caret    = if self.col > 0 {
            format!("{pad} | {}{}", " ".repeat(self.col.saturating_sub(1)), "^")
        } else {
            format!("{pad} | ^")
        };

        format!("{header}\n  --> {loc}\n{pad} |\n{line_no} | {src_line}\n{caret}\n{pad} |")
    }
}

impl Interpreter {
    /// Analiza (lint) un script `.caper`: lex → parse → compile → análisis estático.
    ///
    /// Retorna una lista de [`LintDiagnostic`]s (errores y advertencias).
    /// No ejecuta el script.
    ///
    /// Convención de salida para CLI:
    /// - Cualquier `Severity::Error` → exit 1
    /// - Solo advertencias → exit 0
    pub fn lint(source: &str) -> Vec<LintDiagnostic> {
        let mut diags: Vec<LintDiagnostic> = vec![];

        // Etapa 1 — Lexer
        let tokens = match lexer::tokenize(source) {
            Ok(t) => t,
            Err(e) => {
                diags.push(err_to_lint(e));
                return diags;
            }
        };

        // Etapa 2 — Parser
        let ast = match parser::parse(tokens) {
            Ok(a) => a,
            Err(e) => {
                diags.push(err_to_lint(e));
                return diags;
            }
        };

        // Etapa 3 — Compilación
        let compiled = match compiler::compile(ast) {
            Ok(c) => c,
            Err(e) => {
                diags.push(err_to_lint(e));
                return diags;
            }
        };

        // Etapa 4 — Análisis estático sobre el bytecode compilado
        diags.extend(static_analysis(&compiled, source));
        diags
    }
}

// ---------------------------------------------------------------------------
// Pases de análisis estático sobre CompiledProgram
// ---------------------------------------------------------------------------

fn static_analysis(
    compiled: &instruction::CompiledProgram,
    source: &str,
) -> Vec<LintDiagnostic> {
    use instruction::Instruction;

    let mut diags = vec![];
    let chunk0 = &compiled.chunks[0].code;

    // El compilador siempre añade un `Const(Null) + Emit(0)` centinela al
    // final del chunk 0. Los emits centinela llevan line=0; los emits explícitos llevan line>0.
    // Recolectar sólo los emits explícitos (line > 0).
    let explicit_emits: Vec<usize> = chunk0.iter().enumerate()
        .filter_map(|(i, instr)| match instr {
            Instruction::Emit(line) if *line > 0 => Some(i),
            _ => None,
        })
        .collect();

    // --- Pase 1: emit ausente ---
    if explicit_emits.is_empty() {
        diags.push(LintDiagnostic::error(
            "MISSING_EMIT",
            "script has no `emit` statement — it will always return null",
            0, 0,
        ));
    } else if explicit_emits.len() > 1 {
        // Múltiples emits explícitos — el ejecutor se detiene en el primero alcanzado.
        // Los emits anteriores pueden ser inalcanzables si están en una ruta lineal.
        for &pos in &explicit_emits[..explicit_emits.len() - 1] {
            let line = match &chunk0[pos] {
                Instruction::Emit(l) => *l,
                _ => 0,
            };
            diags.push(LintDiagnostic::warning(
                "DEAD_EMIT",
                "this `emit` may be shadowed — execution stops at the first `emit` reached",
                line, 0,
            ));
        }
    }

    // --- Pase 2: cuerpos de función sin ruta de retorno ---
    for (idx, chunk) in compiled.chunks.iter().enumerate().skip(1) {
        let has_ret  = chunk.code.iter().any(|i| matches!(i, Instruction::Ret));
        let has_emit = chunk.code.iter().any(|i| matches!(i, Instruction::Emit(_)));
        if !has_ret && !has_emit {
            let name = chunk.name.as_deref().unwrap_or("<closure>");
            diags.push(LintDiagnostic::warning(
                "NO_RETURN",
                format!("function '{name}' (chunk {idx}) has no `emit` — will implicitly return null"),
                0, 0,
            ));
        }
    }

    let _ = source; // reservado para futuros pases a nivel de fuente
    diags
}

/// Convierte un `CaperError` en un `LintDiagnostic`.
fn err_to_lint(e: CaperError) -> LintDiagnostic {
    let (line, col) = e.position().unwrap_or((0, 0));
    LintDiagnostic::error(e.code(), e.to_string(), line, col)
}


// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

/// Envuelve un único `NativeFn` como `CaperPlugin` para que pueda vivir en el registry.
struct NativeFnPlugin {
    name: String,
    nf:   NativeFn,
}

impl CaperPlugin for NativeFnPlugin {
    fn module_name(&self) -> &str { &self.name }

    fn call(&self, name: &str, args: Vec<Value>, line: usize) -> Result<Option<Value>, CaperError> {
        if name == self.name {
            let result = (self.nf.func)(args, line)?;
            Ok(Some(result))
        } else {
            Ok(None)
        }
    }

    fn function_names(&self) -> Vec<&str> { vec![&self.name] }
}