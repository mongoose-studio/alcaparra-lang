// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Gestión de paquetes — manifiesto `pkg.capercfg`, lockfile `caper.lock` e instalación.
//!
//! Comandos asociados:
//!   `caper install` — instala dependencias declaradas en `.capercfg` al directorio `vendors/`
//!   `caper new`     — scaffolding de proyecto o paquete nuevo
//!
//! Fuentes de paquetes soportadas:
//!   - **path:**    ruta local relativa al proyecto (operativo)
//!   - **git:**     repositorio git con tag, branch o commit (operativo)
//!   - **registry** (stub): preparado para `registry.alcaparra.dev` — no operativo aún
//!
//! ## Estructura de directorios
//!
//! ```text
//! mi-proyecto/
//!   .capercfg          ← declara dependencias
//!   caper.lock         ← versiones exactas bloqueadas (commitear junto al código)
//!   main.caper
//!   vendors/
//!     @autor/
//!       mi-paquete/    ← paquete con scope
//!         pkg.capercfg
//!         lib.caper
//!     autor/
//!       otro-pkg/      ← paquete sin scope
//!         pkg.capercfg
//! ```
//!
//! ## Imports desde scripts
//!
//! ```caper
//! use @autor/mi-paquete.lib.{ mi_funcion };
//! use autor/otro-pkg.modulo.{ helper };
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{CaperConfig, DepSpec};

// ---------------------------------------------------------------------------
// Manifiesto del paquete (pkg.capercfg)
// ---------------------------------------------------------------------------

/// Manifiesto de un paquete AlcaparraLang (`pkg.capercfg`).
///
/// Describe los metadatos y módulos exportados del paquete.
/// El nombre debe incluir el author: `@author/nombre` o `author/nombre`.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct PkgManifest {
    /// Nombre del paquete, ej. `@mongoose-studio/formulas-cl` o `validaciones`.
    pub name: String,
    /// Versión semver del paquete, ej. `"1.0.0"`.
    pub version: String,
    /// Descripción breve del paquete.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Autores del paquete.
    #[serde(default)]
    pub authors: Vec<String>,
    /// Licencia SPDX, ej. `"MIT"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    /// Módulos exportados por el paquete (nombres de archivo `.caper` sin extensión).
    #[serde(default)]
    pub exports: Vec<String>,
    /// Dependencias transitivas del paquete.
    #[serde(default)]
    pub dependencies: HashMap<String, DepSpec>,
}

impl PkgManifest {
    /// Lee un `pkg.capercfg` desde el directorio indicado.
    pub fn read_from(dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path = dir.join("pkg.capercfg");
        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("no se pudo leer {}: {e}", path.display()))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("error al parsear {}: {e}", path.display()).into())
    }

    /// Escribe el manifiesto al directorio indicado como `pkg.capercfg`.
    pub fn write_to(&self, dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let path = dir.join("pkg.capercfg");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json + "\n")?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Lockfile (caper.lock)
// ---------------------------------------------------------------------------

/// Lockfile determinista del proyecto (`caper.lock`).
///
/// Registra la versión y fuente exacta de cada paquete instalado para garantizar
/// builds reproducibles. Se recomienda commitearlo junto al código fuente.
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct LockFile {
    /// Versión del formato del lockfile (actualmente 1).
    pub version: u32,
    /// Paquetes instalados y bloqueados.
    #[serde(default)]
    pub packages: Vec<LockedPackage>,
}

/// Entrada de un paquete bloqueado en `caper.lock`.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockedPackage {
    /// Nombre del paquete, ej. `@mongoose-studio/formulas-cl`.
    pub name: String,
    /// Versión instalada.
    pub version: String,
    /// Origen: `"registry+https://..."`, `"git+url#commit"` o `"path:/ruta/local"`.
    pub source: String,
    /// Checksum SHA-256 del contenido del paquete (registry y git solamente).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
}

impl LockFile {
    const FILENAME: &'static str = "caper.lock";

    /// Lee el lockfile desde el directorio de proyecto. Retorna vacío si no existe.
    pub fn read_from(project_dir: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path = project_dir.join(Self::FILENAME);
        if !path.exists() {
            return Ok(Self { version: 1, packages: vec![] });
        }
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str(&content)
            .map_err(|e| format!("error al parsear caper.lock: {e}").into())
    }

    /// Escribe el lockfile en el directorio de proyecto.
    pub fn write_to(&self, project_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let path = project_dir.join(Self::FILENAME);
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json + "\n")?;
        Ok(())
    }

    /// Actualiza la entrada de un paquete o la agrega si no existe.
    pub fn upsert(&mut self, pkg: LockedPackage) {
        if let Some(existing) = self.packages.iter_mut().find(|p| p.name == pkg.name) {
            *existing = pkg;
        } else {
            self.packages.push(pkg);
        }
    }
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

/// Resultado de la instalación de una dependencia.
#[derive(Debug)]
pub enum InstallResult {
    /// Paquete instalado correctamente.
    Installed { name: String, version: String, source: String },
    /// Paquete ya presente — sin cambios.
    AlreadyInstalled { name: String },
    /// Fuente no disponible aún (registry pendiente de implementación).
    Stub { name: String, reason: String },
    /// Error durante la instalación.
    Failed { name: String, error: String },
}

/// Instala todas las dependencias declaradas en el `.capercfg` del directorio dado.
///
/// - `path:`    → copia el paquete local a `vendors/`
/// - `git:`     → clona el repositorio y extrae los archivos `.caper`
/// - versión    → informa que el registry aún no está disponible
///
/// Al finalizar, escribe o actualiza `caper.lock`.
pub fn install(project_dir: &Path) -> Result<Vec<InstallResult>, Box<dyn std::error::Error>> {
    let cfg_path = project_dir.join(".capercfg");
    if !cfg_path.exists() {
        return Err(format!(
            "no se encontró .capercfg en {}",
            project_dir.display()
        ).into());
    }

    let content = std::fs::read_to_string(&cfg_path)?;
    let cfg: CaperConfig = serde_json::from_str(&content)
        .map_err(|e| format!("error al parsear .capercfg: {e}"))?;

    if cfg.dependencies.is_empty() {
        println!("No hay dependencias declaradas en .capercfg.");
        return Ok(vec![]);
    }

    let vendors_dir = project_dir.join("vendors");
    std::fs::create_dir_all(&vendors_dir)?;

    let mut lock = LockFile::read_from(project_dir)?;
    lock.version = 1;

    let mut results = vec![];

    for (name, dep) in &cfg.dependencies {
        let result = install_dep(name, dep, &vendors_dir, project_dir);
        match &result {
            InstallResult::Installed { version, source, .. } => {
                lock.upsert(LockedPackage {
                    name:     name.clone(),
                    version:  version.clone(),
                    source:   source.clone(),
                    checksum: None,
                });
            }
            // Si ya estaba instalado, el lock existente se mantiene intacto
            _ => {}
        }
        results.push(result);
    }

    lock.write_to(project_dir)?;
    Ok(results)
}

/// Instala una única dependencia en el directorio vendors.
fn install_dep(
    name: &str,
    dep: &DepSpec,
    vendors_dir: &Path,
    project_dir: &Path,
) -> InstallResult {
    match dep {
        // Versión semver → requiere registry (stub)
        DepSpec::Version(v) => InstallResult::Stub {
            name: name.to_string(),
            reason: format!(
                "paquete '{name}' v{v} requiere el registry (registry.alcaparra.dev) — \
                 aún no disponible. Usa 'git' o 'path' como fuente por ahora."
            ),
        },

        DepSpec::Detailed(detail) => {
            if let Some(local_path) = &detail.path {
                install_path_dep(name, local_path, vendors_dir, project_dir)
            } else if let Some(git_url) = &detail.git {
                let ref_spec = detail.tag.as_deref()
                    .or(detail.branch.as_deref())
                    .unwrap_or("HEAD");
                install_git_dep(name, git_url, ref_spec, vendors_dir)
            } else if let Some(v) = &detail.version {
                InstallResult::Stub {
                    name: name.to_string(),
                    reason: format!(
                        "paquete '{name}' v{v} requiere el registry — aún no disponible."
                    ),
                }
            } else {
                InstallResult::Failed {
                    name: name.to_string(),
                    error: "la dependencia no especifica 'version', 'git' ni 'path'".to_string(),
                }
            }
        }
    }
}

/// Instala una dependencia desde una ruta local al directorio vendors.
fn install_path_dep(
    name: &str,
    rel_path: &str,
    vendors_dir: &Path,
    project_dir: &Path,
) -> InstallResult {
    let src_dir = project_dir.join(rel_path);
    if !src_dir.exists() {
        return InstallResult::Failed {
            name:  name.to_string(),
            error: format!("ruta local '{}' no encontrada", src_dir.display()),
        };
    }

    let dest = vendor_pkg_dir(vendors_dir, name);

    // Si ya existe, no reinstalar
    if dest.exists() {
        return InstallResult::AlreadyInstalled { name: name.to_string() };
    }

    if let Err(e) = copy_pkg_files(&src_dir, &dest) {
        return InstallResult::Failed { name: name.to_string(), error: e.to_string() };
    }

    // Leer la versión del pkg.capercfg del paquete fuente
    let version = PkgManifest::read_from(&src_dir)
        .map(|m| m.version)
        .unwrap_or_else(|_| "0.0.0".to_string());

    InstallResult::Installed {
        name:    name.to_string(),
        version,
        source:  format!("path:{}", src_dir.canonicalize()
            .unwrap_or(src_dir.to_path_buf()).display()),
    }
}

/// Clona un repositorio git e instala los archivos del paquete en vendors.
fn install_git_dep(
    name: &str,
    url: &str,
    ref_spec: &str,
    vendors_dir: &Path,
) -> InstallResult {
    let dest = vendor_pkg_dir(vendors_dir, name);

    if dest.exists() {
        return InstallResult::AlreadyInstalled { name: name.to_string() };
    }

    // Directorio temporal para el clone
    let tmp = vendors_dir.join(format!(".tmp_{}", sanitize_for_dirname(name)));
    let _ = std::fs::remove_dir_all(&tmp);

    // git clone --depth 1 --branch <ref> <url> <tmp>
    let clone_status = Command::new("git")
        .args([
            "clone", "--depth", "1",
            "--branch", ref_spec,
            url,
            &tmp.to_string_lossy(),
        ])
        .status();

    match clone_status {
        Err(e) => {
            let _ = std::fs::remove_dir_all(&tmp);
            return InstallResult::Failed {
                name:  name.to_string(),
                error: format!("no se pudo ejecutar git: {e}"),
            };
        }
        Ok(s) if !s.success() => {
            let _ = std::fs::remove_dir_all(&tmp);
            return InstallResult::Failed {
                name:  name.to_string(),
                error: format!("git clone falló (branch/tag '{ref_spec}' no encontrado en {url})"),
            };
        }
        Ok(_) => {}
    }

    // Copiar .caper y pkg.capercfg al destino
    if let Err(e) = copy_pkg_files(&tmp, &dest) {
        let _ = std::fs::remove_dir_all(&tmp);
        return InstallResult::Failed { name: name.to_string(), error: e.to_string() };
    }

    // Obtener commit SHA para el lockfile
    let commit = git_head_commit(&dest).unwrap_or_else(|| ref_spec.to_string());
    let _ = std::fs::remove_dir_all(&tmp);

    let version = PkgManifest::read_from(&dest)
        .map(|m| m.version)
        .unwrap_or_else(|_| ref_spec.to_string());

    InstallResult::Installed {
        name:    name.to_string(),
        version,
        source:  format!("git+{url}#{commit}"),
    }
}

// ---------------------------------------------------------------------------
// Scaffolding (`caper new`)
// ---------------------------------------------------------------------------

/// Crea el scaffold de un nuevo **proyecto** en `./<name>/`.
///
/// Genera `.capercfg` y `main.caper` con estructura mínima lista para ejecutar.
pub fn new_project(name: &str, base_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let dir = base_dir.join(name);
    if dir.exists() {
        return Err(format!("el directorio '{}' ya existe", dir.display()).into());
    }
    std::fs::create_dir_all(&dir)?;

    // .capercfg del proyecto
    let capercfg = serde_json::json!({
        "name":         name,
        "version":      "0.1.0",
        "entry":        "main.caper",
        "context":      {},
        "paths":        {},
        "dependencies": {}
    });
    std::fs::write(
        dir.join(".capercfg"),
        serde_json::to_string_pretty(&capercfg)? + "\n",
    )?;

    // main.caper con estructura base
    std::fs::write(
        dir.join("main.caper"),
        format!(
            "header {{\n    name: \"{name}\";\n    version: \"0.1.0\";\n}}\n\nmain {{\n    emit {{ mensaje: \"Hola desde {name}!\" }};\n}}\n"
        ),
    )?;

    // .gitignore — vendors es opcional committear (ver comentario)
    std::fs::write(
        dir.join(".gitignore"),
        "# Los vendors instalados vía git/registry pueden omitirse del repo.\n\
         # Los vendors path: deben commitearse o estar disponibles localmente.\n\
         # vendors/\n",
    )?;

    println!("Proyecto '{}' creado en '{}'.", name, dir.display());
    println!("  cd {name} && caper run");
    Ok(())
}

/// Crea el scaffold de un nuevo **paquete** publicable en `./<author>/<pkg>/`.
///
/// El nombre debe incluir el author: `@author/nombre` o `author/nombre`.
/// Genera `pkg.capercfg` y `lib.caper` con estructura mínima.
pub fn new_package(name: &str, base_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    if !name.contains('/') {
        return Err(format!(
            "el nombre de paquete debe incluir author: '@author/{name}' o 'author/{name}'"
        ).into());
    }

    // Construir el directorio: @author/pkg → ./@author/pkg/
    let dir = {
        let mut p = base_dir.to_path_buf();
        for part in name.split('/') { p = p.join(part); }
        p
    };

    if dir.exists() {
        return Err(format!("el directorio '{}' ya existe", dir.display()).into());
    }
    std::fs::create_dir_all(&dir)?;

    // pkg.capercfg
    let pkg_short = name.rsplit('/').next().unwrap_or(name);
    let manifest = PkgManifest {
        name:         name.to_string(),
        version:      "0.1.0".to_string(),
        description:  Some(format!("Paquete AlcaparraLang: {pkg_short}")),
        authors:      vec![],
        license:      Some("MIT".to_string()),
        exports:      vec!["lib".to_string()],
        dependencies: HashMap::new(),
    };
    manifest.write_to(&dir)?;

    // lib.caper con función de ejemplo
    std::fs::write(
        dir.join("lib.caper"),
        format!(
            "header {{\n    name: \"{name}\";\n    version: \"0.1.0\";\n}}\n\n\
             // Exporta tus funciones aquí.\n\
             fn hola(quien) {{\n    emit \"Hola desde {pkg_short}, \" + quien + \"!\";\n}}\n"
        ),
    )?;

    // .gitignore
    std::fs::write(
        dir.join(".gitignore"),
        "# vendors/\n",
    )?;

    println!("Paquete '{}' creado en '{}'.", name, dir.display());
    println!("  Edita pkg.capercfg para agregar descripción y autores.");
    println!("  Importa con:  use {name}.lib.{{ hola }};");
    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

/// Retorna la ruta del directorio del paquete dentro de `vendors/`.
///
/// `@author/pkg` → `vendors/@author/pkg/`
/// `author/pkg`  → `vendors/author/pkg/`
pub fn vendor_pkg_dir(vendors_dir: &Path, pkg_name: &str) -> PathBuf {
    let mut path = vendors_dir.to_path_buf();
    for part in pkg_name.split('/') {
        path = path.join(part);
    }
    path
}

/// Copia los archivos relevantes de un paquete (`*.caper` y `pkg.capercfg`) de `src` a `dst`.
/// Solo copia el nivel raíz — los paquetes son flat por diseño.
fn copy_pkg_files(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry    = entry?;
        let path     = entry.path();
        let name_os  = entry.file_name();
        let name_str = name_os.to_string_lossy();
        if path.is_file() {
            let relevant = name_str.ends_with(".caper") || name_str == "pkg.capercfg";
            if relevant {
                std::fs::copy(&path, dst.join(&name_os))?;
            }
        }
        // No se copian subdirectorios — los paquetes son flat
    }
    Ok(())
}

/// Obtiene el commit SHA del HEAD en un directorio de repositorio git.
fn git_head_commit(dir: &Path) -> Option<String> {
    Command::new("git")
        .args(["-C", &dir.to_string_lossy(), "rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Sanitiza el nombre de un paquete para usarlo como sufijo de directorio temporal.
fn sanitize_for_dirname(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}
