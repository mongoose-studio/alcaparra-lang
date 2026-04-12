// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Sistema de plugins para AlcaparraLang.
//!
//! Dos puntos de extensión:
//!
//! ## Opción A — Plugins Rust de primera parte (trait `CaperPlugin`)
//!
//! Implementa `CaperPlugin` en Rust y regístralo con
//! `Interpreter::register_plugin(Box::new(MyPlugin))`.
//! El plugin expone un nombre de módulo y un método `call`.
//! En scripts se invoca como `std.my_module.fn_name(...)`.
//!
//! ```rust,no_run
//! use alcaparra::plugin::CaperPlugin;
//! use alcaparra::{CaperError, Value};
//!
//! struct FsPlugin;
//!
//! impl CaperPlugin for FsPlugin {
//!     fn module_name(&self) -> &str { "fs" }
//!
//!     fn call(&self, name: &str, args: Vec<Value>, line: usize) -> Result<Option<Value>, CaperError> {
//!         match name {
//!             "read_file" => {
//!                 let path = match args.into_iter().next() {
//!                     Some(Value::Str(s)) => s,
//!                     _ => return Err(CaperError::TypeError {
//!                         line, detail: "read_file: argument must be a string".into()
//!                     }),
//!                 };
//!                 let content = std::fs::read_to_string(&path)
//!                     .map_err(|e| CaperError::ArgumentError {
//!                         func: "read_file".into(), detail: e.to_string()
//!                     })?;
//!                 Ok(Some(Value::str(content)))
//!             }
//!             _ => Ok(None), // unknown function — let the executor continue searching
//!         }
//!     }
//!
//!     fn function_names(&self) -> Vec<&str> { vec!["read_file", "write_file"] }
//! }
//! ```
//!
//! ## Opción B — Plugins dinámicos (`.so` / `.dll`)
//!
//! Compilados como shared libraries que exponen el C ABI `alcaparra_plugin_call`.
//! Se registran con `Interpreter::load_plugin("ruta/al/plugin.so")`.
//! Los valores se intercambian como strings JSON a través de la frontera FFI.
//!
//! Esta funcionalidad requiere compilar con `--features dynamic_plugins`.

use crate::error::CaperError;
use crate::vm::value::Value;
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Trait CaperPlugin — implementar para plugins de primera parte
// ---------------------------------------------------------------------------

/// Trait para plugins Rust de primera parte.
///
/// Impleméntalo y regístralo con `Interpreter::register_plugin`.
/// El intérprete es single-threaded, por lo que no se requieren `Send + Sync`.
pub trait CaperPlugin {
    /// El namespace de módulo que ocupa este plugin, ej. `"fs"` → `use std.fs.read_file`.
    fn module_name(&self) -> &str;

    /// Llama a una función en este plugin.
    ///
    /// Retorna `Ok(Some(value))` si la función fue manejada,
    /// `Ok(None)` si el nombre de función no es reconocido (el executor seguirá buscando),
    /// `Err(e)` para errores en tiempo de ejecución.
    fn call(&self, name: &str, args: Vec<Value>, line: usize) -> Result<Option<Value>, CaperError>;

    /// Opcional: lista los nombres de funciones que exporta este plugin.
    /// Usado para validación y herramientas de IDE.
    fn function_names(&self) -> Vec<&str> { vec![] }
}

// ---------------------------------------------------------------------------
// PluginRegistry — almacena todos los plugins registrados
// ---------------------------------------------------------------------------

pub struct PluginRegistry {
    /// nombre_modulo → plugin
    plugins: HashMap<String, Box<dyn CaperPlugin>>,
}

impl Default for PluginRegistry {
    fn default() -> Self { Self::new() }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self { plugins: HashMap::new() }
    }

    /// Registra un plugin de primera parte.
    pub fn register(&mut self, plugin: Box<dyn CaperPlugin>) {
        self.plugins.insert(plugin.module_name().to_string(), plugin);
    }

    /// Intenta llamar a `function_name` en cualquier plugin que declare `module_name`.
    ///
    /// Si `module_name` es `None`, busca en todos los plugins en orden de registro.
    pub fn call(
        &self,
        module_name: Option<&str>,
        function_name: &str,
        args: Vec<Value>,
        line: usize,
    ) -> Result<Option<Value>, CaperError> {
        if let Some(module) = module_name {
            if let Some(plugin) = self.plugins.get(module) {
                return plugin.call(function_name, args, line);
            }
            return Ok(None);
        }
        // Sin hint de módulo — probar en todos los plugins
        for plugin in self.plugins.values() {
            if let Some(v) = plugin.call(function_name, args.clone(), line)? {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    pub fn has_module(&self, module_name: &str) -> bool {
        self.plugins.contains_key(module_name)
    }

    /// Todos los nombres de módulo registrados.
    pub fn module_names(&self) -> Vec<&str> {
        self.plugins.keys().map(|s| s.as_str()).collect()
    }
}

// ---------------------------------------------------------------------------
// Carga dinámica de plugins (protegida por feature flag)
// ---------------------------------------------------------------------------

/// Carga un plugin dinámico desde un archivo `.so` / `.dll` / `.dylib`.
///
/// La shared library debe exportar una función con C ABI:
///
/// ```c
/// // Retorna un string JSON o un error terminado en null prefijado con "ERROR:"
/// const char* alcaparra_plugin_call(
///     const char* fn_name,   // nombre de función
///     const char* args_json, // array JSON de argumentos
///     const char* out_buf,   // buffer provisto por el caller (1 MB)
///     int         buf_len
/// );
///
/// // Opcional: retorna lista separada por comas de nombres de funciones exportadas
/// const char* alcaparra_plugin_functions();
///
/// // Requerido: retorna el nombre del módulo (ej. "fs")
/// const char* alcaparra_plugin_module();
/// ```
///
/// Los valores se serializan a/desde JSON a través de la frontera FFI.
#[cfg(feature = "dynamic_plugins")]
pub mod dynamic {
    use super::*;
    use libloading::{Library, Symbol};
    use std::ffi::{CStr, CString};
    use std::path::Path;

    pub struct DynamicPlugin {
        module: String,
        _lib: Library, // keep alive
        call_fn: unsafe extern "C" fn(*const i8, *const i8, *mut i8, i32) -> *const i8,
    }

    impl DynamicPlugin {
        /// Carga una shared library de plugin desde `path`.
        pub fn load(path: &Path) -> Result<Self, CaperError> {
            let lib = unsafe { Library::new(path) }.map_err(|e| {
                CaperError::RuntimeError { message: format!("plugin load error: {e}"), line: 0 }
            })?;

            let module: String = unsafe {
                let sym: Symbol<unsafe extern "C" fn() -> *const i8> =
                    lib.get(b"alcaparra_plugin_module\0").map_err(|e| {
                        CaperError::RuntimeError {
                            message: format!("plugin missing alcaparra_plugin_module: {e}"),
                            line: 0,
                        }
                    })?;
                let ptr = sym();
                if ptr.is_null() {
                    return Err(CaperError::RuntimeError {
                        message: "alcaparra_plugin_module returned null".into(),
                        line: 0,
                    });
                }
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            };

            let call_fn: unsafe extern "C" fn(*const i8, *const i8, *mut i8, i32) -> *const i8 =
                unsafe {
                    *lib.get(b"alcaparra_plugin_call\0").map_err(|e| {
                        CaperError::RuntimeError {
                            message: format!("plugin missing alcaparra_plugin_call: {e}"),
                            line: 0,
                        }
                    })?
                };

            Ok(Self { module, _lib: lib, call_fn })
        }
    }

    impl CaperPlugin for DynamicPlugin {
        fn module_name(&self) -> &str { &self.module }

        fn call(&self, name: &str, args: Vec<Value>, line: usize) -> Result<Option<Value>, CaperError> {
            // Serializar args a JSON
            let json_args: serde_json::Value = args
                .into_iter()
                .map(serde_json::Value::from)
                .collect::<Vec<_>>()
                .into();
            let args_str = serde_json::to_string(&json_args).unwrap_or_else(|_| "[]".into());

            let fn_cstr = CString::new(name).unwrap_or_default();
            let args_cstr = CString::new(args_str).unwrap_or_default();

            // Buffer de salida (1 MB)
            let mut out_buf = vec![0i8; 1024 * 1024];

            let result_ptr = unsafe {
                (self.call_fn)(
                    fn_cstr.as_ptr(),
                    args_cstr.as_ptr(),
                    out_buf.as_mut_ptr(),
                    out_buf.len() as i32,
                )
            };

            if result_ptr.is_null() {
                return Ok(None); // función no encontrada en el plugin
            }

            let result_str = unsafe { CStr::from_ptr(result_ptr).to_string_lossy() };

            if let Some(err) = result_str.strip_prefix("ERROR:") {
                return Err(CaperError::RuntimeError { message: err.trim().to_string(), line });
            }

            let json: serde_json::Value = serde_json::from_str(&result_str).map_err(|e| {
                CaperError::RuntimeError {
                    message: format!("plugin returned invalid JSON: {e}"),
                    line,
                }
            })?;

            Ok(Some(crate::vm::value::json_to_value(json)))
        }
    }

    /// Carga un plugin dinámico y lo registra en un `PluginRegistry`.
    pub fn load_into(registry: &mut PluginRegistry, path: &Path) -> Result<String, CaperError> {
        let plugin = DynamicPlugin::load(path)?;
        let module = plugin.module_name().to_string();
        registry.register(Box::new(plugin));
        Ok(module)
    }
}
