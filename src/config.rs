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

/// Una ruta de contexto o varias.
///
/// Existe para que el manifiesto acepte las dos escrituras sin que el campo sea
/// polimórfico hacia afuera: adentro siempre es una lista.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ContextFiles {
    One(String),
    Many(Vec<String>),
}

impl Default for ContextFiles {
    fn default() -> Self { Self::Many(vec![]) }
}

impl ContextFiles {
    pub fn as_slice(&self) -> &[String] {
        match self {
            Self::One(p)  => std::slice::from_ref(p),
            Self::Many(v) => v,
        }
    }

    pub fn is_empty(&self) -> bool { self.as_slice().is_empty() }
}

/// Por qué no se pudo armar el contexto del proyecto.
///
/// Es un error y no una advertencia a propósito: si el manifiesto apunta a un
/// archivo de contexto, correr sin él no es «correr con menos datos», es correr
/// otra cosa.
#[derive(Debug)]
pub enum ContextError {
    /// El archivo no se pudo leer.
    Unreadable { path: PathBuf, detail: String },
    /// Se leyó pero no es JSON válido.
    NotJson { path: PathBuf, detail: String },
    /// Es JSON válido pero no es un objeto, así que no tiene claves que inyectar.
    NotAnObject { path: PathBuf },
}

impl std::fmt::Display for ContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreadable { path, detail } =>
                write!(f, "cannot read context file {}: {detail}", path.display()),
            Self::NotJson { path, detail } =>
                write!(f, "context file {} is not valid JSON: {detail}", path.display()),
            Self::NotAnObject { path } =>
                write!(f, "context file {} must contain a JSON object", path.display()),
        }
    }
}

impl std::error::Error for ContextError {}

/// Contenido de un archivo `.capercfg` — el manifiesto del proyecto.
#[derive(Debug, Deserialize, Default)]
pub struct CaperConfig {
    pub name:        Option<String>,
    pub version:     Option<String>,
    pub description: Option<String>,
    pub author:      Option<String>,
    /// Punto de entrada del script. Por defecto `main.caper`.
    pub entry:       Option<String>,
    /// Variables de contexto **por defecto** inyectadas en modo proyecto.
    ///
    /// Es la base: lo que traiga `context_file` va encima. Ver `resolved_context`.
    #[serde(default)]
    pub context:     serde_json::Value,
    /// Archivos de los que sale el contexto de verdad, en orden de aplicación.
    ///
    /// Acepta una ruta sola o una lista:
    ///
    /// ```json
    /// { "context_file": "./contexto/base.json" }
    /// { "context_file": ["./contexto/base.json", "./contexto/ana.json"] }
    /// ```
    ///
    /// Las rutas se resuelven **relativas al directorio del `.capercfg`**, igual
    /// que `paths`.
    #[serde(default)]
    pub context_file: ContextFiles,
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

    /// Rutas de `context_file` resueltas contra el directorio del manifiesto.
    ///
    /// Se normalizan para que un `"./contexto/x.json"` no termine apareciendo como
    /// `/proyecto/./contexto/x.json` en un mensaje de error que alguien va a leer.
    pub fn resolved_context_files(&self, base: &Path) -> Vec<PathBuf> {
        self.context_file
            .as_slice()
            .iter()
            .map(|p| base.join(p).components().collect::<PathBuf>())
            .collect()
    }

    /// El contexto efectivo del proyecto: `context` como base y los
    /// `context_file` encima, en orden.
    ///
    /// **El archivo gana sobre el manifiesto**, y la razón vale la pena dejarla
    /// escrita: el archivo es el payload de verdad —el mismo que manda el host, con
    /// los montos como string y todo— mientras que lo que está incrustado en el
    /// manifiesto son valores por defecto, puestos a mano para poder correr el
    /// proyecto sin nada más. Si el manifiesto ganara, bastaría un `UF` olvidado
    /// ahí adentro para que la corrida mintiera sobre el caso real.
    ///
    /// La fusión es **por clave de primer nivel**: un archivo posterior reemplaza
    /// la clave entera, no la mezcla por dentro. Es predecible, que es más
    /// importante que ser astuto — un `TRABAJADOR` a medio fusionar entre dos
    /// archivos es imposible de leer en un diagnóstico.
    pub fn resolved_context(&self, base: &Path) -> Result<serde_json::Value, ContextError> {
        let mut merged = match &self.context {
            serde_json::Value::Object(m) => m.clone(),
            _ => serde_json::Map::new(),
        };

        for path in self.resolved_context_files(base) {
            let text = std::fs::read_to_string(&path).map_err(|e| ContextError::Unreadable {
                path: path.clone(),
                detail: e.to_string(),
            })?;
            let parsed: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| ContextError::NotJson { path: path.clone(), detail: e.to_string() })?;
            match parsed {
                serde_json::Value::Object(m) => {
                    for (k, v) in m { merged.insert(k, v); }
                }
                _ => return Err(ContextError::NotAnObject { path }),
            }
        }

        Ok(serde_json::Value::Object(merged))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).expect("write fixture");
    }

    fn config_from(json: &str) -> CaperConfig {
        serde_json::from_str(json).expect("parse .capercfg")
    }

    #[test]
    fn context_file_accepts_one_path_or_many() {
        let one  = config_from(r#"{ "context_file": "a.json" }"#);
        let many = config_from(r#"{ "context_file": ["a.json", "b.json"] }"#);
        assert_eq!(one.context_file.as_slice(),  ["a.json".to_string()]);
        assert_eq!(many.context_file.as_slice(), ["a.json".to_string(), "b.json".to_string()]);
        assert!(config_from("{}").context_file.is_empty());
    }

    /// El archivo es la verdad y el manifiesto el valor por defecto; entre archivos
    /// gana el último.
    #[test]
    fn files_win_over_the_manifest_and_later_files_win_over_earlier_ones() {
        let dir = std::env::temp_dir().join("alcaparra-cfg-precedence");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        write(&dir, "base.json", r#"{ "B": "from-base", "C": "from-base" }"#);
        write(&dir, "case.json", r#"{ "C": "from-case" }"#);

        let cfg = config_from(
            r#"{ "context": { "A": "from-manifest", "B": "from-manifest", "C": "from-manifest" },
                 "context_file": ["base.json", "case.json"] }"#,
        );
        let ctx = cfg.resolved_context(&dir).expect("resolved context");
        let obj = ctx.as_object().expect("object");

        assert_eq!(obj["A"], "from-manifest", "a key only in the manifest survives");
        assert_eq!(obj["B"], "from-base",     "a file beats the manifest");
        assert_eq!(obj["C"], "from-case",     "the last file wins");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_or_broken_context_file_is_an_error() {
        let dir = std::env::temp_dir().join("alcaparra-cfg-broken");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        write(&dir, "broken.json", "{ not json");
        write(&dir, "list.json",   "[1, 2, 3]");

        let missing = config_from(r#"{ "context_file": "nope.json" }"#).resolved_context(&dir);
        assert!(matches!(missing, Err(ContextError::Unreadable { .. })), "got {missing:?}");

        let broken = config_from(r#"{ "context_file": "broken.json" }"#).resolved_context(&dir);
        assert!(matches!(broken, Err(ContextError::NotJson { .. })), "got {broken:?}");

        let list = config_from(r#"{ "context_file": "list.json" }"#).resolved_context(&dir);
        assert!(matches!(list, Err(ContextError::NotAnObject { .. })), "got {list:?}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_relative_path_is_normalised_for_error_messages() {
        let cfg = config_from(r#"{ "context_file": "./ctx/a.json" }"#);
        let resolved = cfg.resolved_context_files(Path::new("/proyecto"));
        assert_eq!(resolved, vec![PathBuf::from("/proyecto/ctx/a.json")]);
    }
}
