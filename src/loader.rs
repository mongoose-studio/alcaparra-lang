// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::compiler;
use crate::error::CaperError;
use crate::instruction::CompiledProgram;
use crate::lexer;
use crate::parser;
use crate::vm::value::Value;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

/// Resuelve y cachea imports de bibliotecas `.caper` de usuario.
///
/// Compartido entre la VM principal y cualquier sub-VM en modo biblioteca via
/// `Rc<RefCell<ImportLoader>>` para que la caché y el stack de detección circular
/// sean consistentes durante toda la ejecución.
pub struct ImportLoader {
    /// Directorio raíz — las rutas `use` del usuario se resuelven relativas a este.
    pub base_dir: PathBuf,
    /// Aliases de ruta de la sección `paths` del `.capercfg`.
    pub aliases: HashMap<String, PathBuf>,
    /// Caché: ruta canónica → (funciones exportadas, programa compilado).
    cache: HashMap<PathBuf, (HashMap<String, Value>, CompiledProgram)>,
    /// Archivos actualmente en carga — usado para detectar imports circulares.
    import_stack: Vec<PathBuf>,
}

impl ImportLoader {
    pub fn new(base_dir: PathBuf, aliases: HashMap<String, PathBuf>) -> Self {
        Self { base_dir, aliases, cache: HashMap::new(), import_stack: Vec::new() }
    }

    pub fn default_loader() -> Rc<RefCell<Self>> {
        let base = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        Rc::new(RefCell::new(Self::new(base, HashMap::new())))
    }

    /// Resuelve un import a una ruta absoluta de archivo `.caper`.
    ///
    /// Formatos soportados:
    /// - `["@alias", "modulo"]`         → alias de ruta del `.capercfg`
    /// - `["@author/pkg", "modulo"]`    → paquete de vendor con scope
    /// - `["author/pkg", "modulo"]`     → paquete de vendor sin scope
    /// - `["modulo", "submodulo"]`      → ruta relativa al `base_dir`
    pub fn resolve(&self, path: &[String]) -> Result<PathBuf, CaperError> {
        if path.is_empty() {
            return Err(CaperError::ImportNotFound { path: String::new() });
        }

        let first = &path[0];

        let mut file_path = if first.starts_with('@') && first.contains('/') {
            // @author/pkg — paquete de vendor con scope
            self.resolve_vendor(first, &path[1..])?
        } else if first.starts_with('@') {
            // @alias — alias de ruta configurado en .capercfg o global
            let alias_base = self.aliases.get(first).ok_or_else(|| {
                CaperError::ImportNotFound {
                    path: format!("alias desconocido '{first}'"),
                }
            })?;
            let mut p = alias_base.clone();
            for seg in &path[1..] { p = p.join(seg); }
            p
        } else if first.contains('/') {
            // author/pkg — paquete de vendor sin scope
            self.resolve_vendor(first, &path[1..])?
        } else {
            // Ruta relativa al base_dir del proyecto
            let mut p = self.base_dir.clone();
            for seg in path { p = p.join(seg); }
            p
        };

        file_path.set_extension("caper");
        file_path.canonicalize().map_err(|_| CaperError::ImportNotFound {
            path: file_path.to_string_lossy().into_owned(),
        })
    }

    /// Resuelve un paquete de vendor a su ruta en el directorio `vendors/`.
    ///
    /// Busca `vendors/` subiendo desde `base_dir` hasta la raíz del sistema de archivos.
    /// `@author/pkg` → `vendors/@author/pkg/<sub_path>`
    /// `author/pkg`  → `vendors/author/pkg/<sub_path>`
    fn resolve_vendor(&self, pkg_ref: &str, sub_path: &[String]) -> Result<PathBuf, CaperError> {
        let vendors_dir = find_vendors_dir(&self.base_dir).ok_or_else(|| {
            CaperError::ImportNotFound {
                path: format!(
                    "directorio vendors/ no encontrado buscando desde '{}' \
                     (¿ejecutaste `caper install`?)",
                    self.base_dir.display()
                ),
            }
        })?;

        // Construir: vendors/<@author>/<pkg>/<sub_path[0]>
        let mut path = vendors_dir;
        for part in pkg_ref.split('/') {
            path = path.join(part);
        }
        for seg in sub_path {
            path = path.join(seg);
        }
        Ok(path)
    }

    /// Carga un archivo de biblioteca y retorna sus funciones exportadas + programa compilado.
    ///
    /// Los resultados se cachean — un archivo se parsea y compila como máximo una vez
    /// por ejecución del intérprete, sin importar cuántos scripts lo importen.
    pub fn load(
        loader_rc: &Rc<RefCell<ImportLoader>>,
        path: &[String],
    ) -> Result<(HashMap<String, Value>, CompiledProgram), CaperError> {
        // Resolver ruta (borrow corto)
        let canonical = loader_rc.borrow().resolve(path)?;

        // Retornar resultado cacheado inmediatamente
        if let Some(cached) = loader_rc.borrow().cache.get(&canonical) {
            return Ok(cached.clone());
        }

        // Detección de imports circulares
        {
            let loader = loader_rc.borrow();
            if loader.import_stack.contains(&canonical) {
                return Err(CaperError::CircularImport {
                    path: canonical.to_string_lossy().into_owned(),
                });
            }
        }

        // Leer y parsear
        let source = std::fs::read_to_string(&canonical).map_err(|e| {
            CaperError::ImportNotFound {
                path: format!("{}: {e}", canonical.display()),
            }
        })?;
        let tokens  = lexer::tokenize(&source)?;
        let ast     = parser::parse(tokens)?;

        // Empujar al stack antes de compilar (protege contra circular via cargas anidadas)
        loader_rc.borrow_mut().import_stack.push(canonical.clone());

        // Compilar la biblioteca a bytecode
        let compiled = compiler::compile(ast)?;

        // Extraer stubs de funciones exportadas (chunk_id presente, el programa será
        // adjuntado por el ejecutor cuando las vincule al scope)
        let functions = extract_library_fns(&compiled);

        // Extraer del stack y cachear
        loader_rc.borrow_mut().import_stack.pop();
        loader_rc.borrow_mut().cache.insert(canonical, (functions.clone(), compiled.clone()));

        Ok((functions, compiled))
    }
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

/// Busca el directorio `vendors/` subiendo desde `start` hasta la raíz del filesystem.
/// Retorna la primera ruta `vendors/` que sea un directorio existente.
fn find_vendors_dir(start: &std::path::Path) -> Option<PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let vendors = dir.join("vendors");
        if vendors.is_dir() { return Some(vendors); }
        if !dir.pop() { break; }
    }
    None
}

// ---------------------------------------------------------------------------
// Recorre el chunk 0 de una biblioteca compilada y recolecta stubs Value::Function
// para cada FnDecl (par Closure + LetBind).
// ---------------------------------------------------------------------------

fn extract_library_fns(compiled: &CompiledProgram) -> HashMap<String, Value> {
    use crate::instruction::Instruction;
    use crate::vm::value::FunctionDef;
    use std::rc::Rc;

    let mut fns = HashMap::new();
    let code = &compiled.chunks[0].code;
    let mut i = 0;
    while i < code.len() {
        if let Instruction::Closure { chunk_id, ref name } = code[i] {
            // The next instruction should be LetBind (the fn name binding)
            if let Some(Instruction::LetBind(bind_name, _)) = code.get(i + 1) {
                let chunk = &compiled.chunks[chunk_id];
                let fd = FunctionDef {
                    name:     name.clone().or_else(|| Some(bind_name.clone())),
                    params:   chunk.params.clone(),
                    body:     vec![],
                    chunk_id: Some(chunk_id),
                    program:  None, // adjuntado más tarde por el ejecutor
                    captured: HashMap::new(),
                };
                fns.insert(bind_name.clone(), Value::Function(Rc::new(fd)));
                i += 2;
                continue;
            }
        }
        i += 1;
    }
    fns
}
