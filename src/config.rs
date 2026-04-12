// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Especificación de dependencias
// ---------------------------------------------------------------------------

/// Especificación de una dependencia en `.capercfg`.
///
/// Puede ser una versión semver simple o una tabla con opciones avanzadas.
///
/// ```json
/// { "dependencies": {
///     "@autor/paquete":  "^1.2.0",
///     "autor/paquete2":  { "git": "https://github.com/.../repo.git", "tag": "v1.0" },
///     "mi-lib-local":    { "path": "../mi-lib" }
/// }}
/// ```
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum DepSpec {
    /// Versión semver: `"^1.2.0"`, `"1.0"`, `"*"`.
    Version(String),
    /// Dependencia con opciones avanzadas (git, path local o versión explícita).
    Detailed(DetailedDep),
}

/// Opciones avanzadas de una dependencia.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct DetailedDep {
    /// Versión semver para el registry, ej. `"1.2.0"`.
    pub version: Option<String>,
    /// URL de repositorio git, ej. `"https://github.com/user/repo.git"`.
    pub git:     Option<String>,
    /// Tag o commit git (usado junto a `git`), ej. `"v1.0.0"`.
    pub tag:     Option<String>,
    /// Branch git (alternativo a `tag`), ej. `"main"`.
    pub branch:  Option<String>,
    /// Ruta local relativa al directorio del proyecto, ej. `"../mi-lib"`.
    pub path:    Option<String>,
}

// ---------------------------------------------------------------------------
// Configuración del formateador
// ---------------------------------------------------------------------------

/// Configuración del formateador — leída desde la sección `fmt` del `.capercfg`.
#[derive(Debug, Deserialize, Default, Clone)]
pub struct FmtConfigRaw {
    /// Espacios por nivel de indentación (por defecto: 4).
    pub indent_size:     Option<usize>,
    /// Carácter de comillas: `"double"` o `"single"` (por defecto: `"double"`).
    pub quotes:          Option<String>,
    /// Máximo de líneas en blanco consecutivas permitidas (por defecto: 1).
    pub max_blank_lines: Option<usize>,
}

/// Contenido de un archivo `.capercfg` — el manifiesto del proyecto.
#[derive(Debug, Deserialize, Default)]
pub struct CaperConfig {
    pub name:        Option<String>,
    pub version:     Option<String>,
    pub description: Option<String>,
    pub author:      Option<String>,
    /// Punto de entrada del script. Por defecto `main.caper`.
    pub entry:       Option<String>,
    /// Variables de contexto por defecto inyectadas en modo proyecto.
    #[serde(default)]
    pub context:     serde_json::Value,
    /// Aliases de ruta, similar a `paths` de tsconfig.
    /// `"@formulas": "./formulas"` → `use @formulas.gratificacion`
    #[serde(default)]
    pub paths:       HashMap<String, String>,
    /// Declaraciones de plugin: `{ "nombre_modulo": "/ruta/al/plugin.so" }`.
    /// Cargados como plugins dinámicos (requiere feature `dynamic_plugins`).
    #[serde(default)]
    pub plugins:     HashMap<String, String>,
    /// Configuración del formateador.
    #[serde(default)]
    pub fmt:          FmtConfigRaw,
    /// Dependencias del proyecto o paquete.
    /// Instaladas en `vendors/<author>/<pkg>/` con `caper install`.
    #[serde(default)]
    pub dependencies: HashMap<String, DepSpec>,
}

impl CaperConfig {
    pub fn entry_file(&self) -> &str {
        self.entry.as_deref().unwrap_or("main.caper")
    }

    /// Resuelve los aliases de ruta relativos al directorio del archivo de configuración.
    pub fn resolved_aliases(&self, base: &Path) -> HashMap<String, PathBuf> {
        self.paths.iter()
            .map(|(alias, rel)| (alias.clone(), base.join(rel)))
            .collect()
    }
}

/// Busca un archivo `.capercfg` a partir de `start_dir` subiendo por los directorios.
/// Retorna la configuración y el directorio que la contiene.
pub fn find_config(start_dir: &Path) -> Option<(CaperConfig, PathBuf)> {
    let mut dir = start_dir.to_path_buf();
    loop {
        let cfg_path = dir.join(".capercfg");
        if cfg_path.exists() {
            let content = std::fs::read_to_string(&cfg_path).ok()?;
            let cfg: CaperConfig = serde_json::from_str(&content)
                .unwrap_or_else(|e| {
                    eprintln!("[alcaparra] Warning: .capercfg parse error: {e}");
                    CaperConfig::default()
                });
            return Some((cfg, dir));
        }
        if !dir.pop() { break; }
    }
    None
}
