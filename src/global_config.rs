// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Cargador de configuración global para AlcaparraLang.
//!
//! Busca `alcaparra.capercfg` en las siguientes ubicaciones (menor → mayor prioridad):
//!
//! 1. **Sistema**  — `/etc/alcaparra/alcaparra.capercfg`
//! 2. **Usuario**  — `~/.alcaparra/alcaparra.capercfg`
//! 3. **Binario**  — `<directorio_binario>/alcaparra.capercfg`
//! 4. **Variable de entorno** — ruta en `$ALCAPARRA_CONFIG`
//!
//! El `.capercfg` de proyecto y los flags de CLI son manejados por `config.rs` y `main.rs`.
//!
//! Los aliases de ruta y plugins son **aditivos** entre niveles (todos se fusionan).
//! Los valores de contexto siguen precedencia (el nivel más alto gana en conflicto).

use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Un único archivo de configuración global.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct GlobalConfig {
    /// Variables de contexto globales inyectadas en cada ejecución de script.
    #[serde(default)]
    pub context: serde_json::Value,

    /// Aliases de ruta globales (aditivos entre niveles).
    #[serde(default)]
    pub paths: HashMap<String, String>,

    /// Declaraciones de plugin: `{ "nombre_modulo": "/ruta/al/plugin.so" }`.
    /// Cargados como plugins dinámicos (requiere feature `dynamic_plugins`).
    #[serde(default)]
    pub plugins: HashMap<String, String>,
}

impl GlobalConfig {
    /// Merge `other` into `self`.
    ///
    /// - `paths` and `plugins` are additive: other overrides same-named entries.
    /// - `context` keys: `self` wins (lower-priority layers are loaded first).
    pub fn merge_from(&mut self, other: GlobalConfig) {
        // Paths and plugins: higher-priority layer (other) overrides
        self.paths.extend(other.paths);
        self.plugins.extend(other.plugins);

        // Context: merge JSON objects, other wins on conflict
        match (&mut self.context, other.context) {
            (serde_json::Value::Object(base), serde_json::Value::Object(over)) => {
                for (k, v) in over { base.entry(k).or_insert(v); }
            }
            (base, over) if !over.is_null() => *base = over,
            _ => {}
        }
    }

    /// Resolve path aliases relative to a base directory.
    pub fn resolved_aliases(&self, base: &Path) -> HashMap<String, PathBuf> {
        self.paths.iter()
            .map(|(alias, rel)| (alias.clone(), base.join(rel)))
            .collect()
    }
}

/// Carga y fusiona todos los archivos de configuración global.
///
/// Retorna un único `GlobalConfig` con todo fusionado (menor prioridad primero,
/// de modo que las capas de mayor prioridad sobreescriben en conflictos de context; aditivo para paths/plugins).
pub fn load_global_config() -> GlobalConfig {
    let mut merged = GlobalConfig::default();

    for path in candidate_paths() {
        if let Some(cfg) = load_file(&path) {
            let base = path.parent().unwrap_or(Path::new(".")).to_path_buf();
            // Resolve relative plugin paths to absolute before merging
            let cfg = resolve_plugin_paths(cfg, &base);
            merged.merge_from(cfg);
        }
    }

    merged
}

/// Retorna todas las rutas candidatas de configuración con su estado (encontrado/no encontrado).
/// Usado por `caper --cfg` para reportar qué archivos existen.
pub fn config_paths() -> Vec<(PathBuf, bool)> {
    candidate_paths().into_iter()
        .map(|p| { let found = p.exists(); (p, found) })
        .collect()
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

/// Retorna las rutas candidatas de configuración en orden ascendente de prioridad.
pub fn candidate_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. System — /etc/alcaparra/alcaparra.capercfg
    paths.push(PathBuf::from("/etc/alcaparra/alcaparra.capercfg"));

    // 2. User home — ~/.alcaparra/alcaparra.capercfg
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".alcaparra").join("alcaparra.capercfg"));
    }

    // 3. Binary directory — <exe_dir>/alcaparra.capercfg
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("alcaparra.capercfg"));
        }
    }

    // 4. Env var — $ALCAPARRA_CONFIG
    if let Ok(env_path) = std::env::var("ALCAPARRA_CONFIG") {
        paths.push(PathBuf::from(env_path));
    }

    paths
}

fn load_file(path: &Path) -> Option<GlobalConfig> {
    if !path.exists() { return None; }
    let content = std::fs::read_to_string(path).ok()?;
    match serde_json::from_str::<GlobalConfig>(&content) {
        Ok(cfg) => Some(cfg),
        Err(e) => {
            eprintln!("[alcaparra] Warning: could not parse {}: {e}", path.display());
            None
        }
    }
}

/// Resuelve rutas de plugins relativas a absolutas, usando `base` como raíz.
fn resolve_plugin_paths(mut cfg: GlobalConfig, base: &Path) -> GlobalConfig {
    cfg.plugins = cfg.plugins.into_iter().map(|(module, path_str)| {
        let p = PathBuf::from(&path_str);
        let resolved = if p.is_absolute() { p } else { base.join(&p) };
        (module, resolved.to_string_lossy().into_owned())
    }).collect();
    cfg
}
