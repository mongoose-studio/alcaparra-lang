// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Librería estándar de AlcaparraLang.
//!
//! Las funciones se resuelven por nombre en tiempo de llamada.
//! Cada módulo expone un `call(name, args, line) -> Option<Result<Value, CaperError>>`.
//! `None` significa "este módulo no maneja ese nombre".
//! `Some(result)` significa "manejado — retornar este resultado (ok o error)".

pub mod arrays;
pub mod crypto;
pub mod dates;
pub mod json;
pub mod math;
pub mod objects;
pub mod rand;
pub mod regex;
pub mod search;
pub mod sort;
pub mod strings;
pub mod time;
pub mod types;
pub mod xml;
pub mod yaml;

use crate::error::CaperError;
use crate::vm::value::Value;

/// Nombres de las funciones HOF que requieren soporte de callback en la VM.
/// La VM las intercepta antes de despachar a la stdlib.
pub use arrays::HOF_NAMES;

/// Metadatos de un módulo de stdlib, mostrado por `caper -m`.
pub struct ModuleInfo {
    pub name:        &'static str,
    pub path:        &'static str,
    pub description: &'static str,
    pub author:      &'static str,
    pub functions:   &'static [&'static str],
}

/// Todos los módulos de stdlib con sus metadatos.
pub const MODULES: &[ModuleInfo] = &[
    ModuleInfo { name: "math",    path: "std.math",    author: "AlcaparraLang core", description: "Mathematical functions",
        functions: &["abs","ceil","floor","round","round_half_up","trunc","sign","min","max","clamp","pow","sqrt","log","log10","exp","pi","e"] },
    ModuleInfo { name: "strings", path: "std.strings", author: "AlcaparraLang core", description: "String manipulation",
        functions: &["len","upper","lower","trim","trim_start","trim_end","starts_with","ends_with","contains","replace","replace_all","split","join","pad_left","pad_right","repeat","char_at","substring","index_of","count_occurrences","to_number","to_string"] },
    ModuleInfo { name: "arrays",  path: "std.arrays",  author: "AlcaparraLang core", description: "Array operations and HOFs",
        functions: &["count","push","pop","shift","unshift","slice","concat","flat","unique","reverse","sum","avg","min","max","first","last","includes","index_of","zip","range","map","filter","reduce","find","find_all","find_index","any","all","sort_by","group_by","partition"] },
    ModuleInfo { name: "objects", path: "std.objects", author: "AlcaparraLang core", description: "Object/map operations",
        functions: &["keys","values","entries","has","get","set","delete","merge","pick","omit","from_entries","count"] },
    ModuleInfo { name: "types",   path: "std.types",   author: "AlcaparraLang core", description: "Type checking and conversion",
        functions: &["type_of","is_null","is_number","is_string","is_bool","is_array","is_object","is_function","to_number","to_string","to_bool"] },
    ModuleInfo { name: "json",    path: "std.json",    author: "AlcaparraLang core", description: "JSON encode / decode",
        functions: &["json_encode","json_decode","json_valid","json_pretty"] },
    ModuleInfo { name: "dates",   path: "std.dates",   author: "AlcaparraLang core", description: "Date arithmetic (Chile holidays included)",
        functions: &["today","now","year","month","day","date_diff","date_add","date_sub","date_format","date_parse","is_valid_date","working_days"] },
    ModuleInfo { name: "rand",    path: "std.rand",    author: "AlcaparraLang core", description: "Random number generation",
        functions: &["rand","rand_int","rand_float","rand_item","rand_sample","shuffle","seed","uuid"] },
    ModuleInfo { name: "regex",   path: "std.regex",   author: "AlcaparraLang core", description: "Regular expressions",
        functions: &["regex_match","regex_test","regex_find","regex_find_all","regex_groups","regex_replace","regex_replace_all","regex_split"] },
    ModuleInfo { name: "crypto",  path: "std.crypto",  author: "AlcaparraLang core", description: "Cryptographic hashing and encoding",
        functions: &["md5","sha1","sha256","sha512","hmac_sha256","base64_encode","base64_decode","base64_url_encode","hex_encode","hex_decode"] },
    ModuleInfo { name: "time",    path: "std.time",    author: "AlcaparraLang core", description: "Timestamps and elapsed time",
        functions: &["timestamp","timestamp_ms","from_timestamp","from_timestamp_ms","to_timestamp","time_now","time_format","elapsed_ms"] },
    ModuleInfo { name: "sort",    path: "std.sort",    author: "AlcaparraLang core", description: "Sorting and grouping",
        functions: &["sort_asc","sort_desc","sort_by_desc","order_by","group_by_key","chunk","group_by","partition"] },
    ModuleInfo { name: "search",  path: "std.search",  author: "AlcaparraLang core", description: "Search and lookup",
        functions: &["binary_search","in_range","search","fuzzy","find_all","find_index"] },
    ModuleInfo { name: "xml",     path: "std.xml",     author: "AlcaparraLang core", description: "XML encode / decode",
        functions: &["xml_encode","xml_encode_opts","xml_decode","xml_valid","xml_get","xml_get_all"] },
    ModuleInfo { name: "yaml",    path: "std.yaml",    author: "AlcaparraLang core", description: "YAML encode / decode",
        functions: &["yaml_encode","yaml_encode_opts","yaml_decode","yaml_valid"] },
];

/// Rutas de módulos de stdlib conocidos — usados para validar sentencias `use` en runtime.
const KNOWN_MODULES: &[&str] = &[
    "std.math",
    "std.strings",
    "std.arrays",
    "std.objects",
    "std.types",
    "std.json",
    "std.dates",
    "std.rand",
    "std.regex",
    "std.crypto",
    "std.time",
    "std.sort",
    "std.search",
    "std.xml",
    "std.yaml",
];

/// Valida una ruta `use` en runtime.
/// Retorna Ok(()) para rutas de stdlib conocidas, Err para las desconocidas.
/// Los imports de scripts definidos por el usuario (ej. `use formulas.gratificacion`)
/// no están soportados aún.
pub fn validate_use(path: &[String]) -> Result<(), CaperError> {
    if path.is_empty() { return Ok(()); }
    // Solo se validan rutas "std.*"; todo lo demás es un import de usuario
    if path[0] != "std" { return Ok(()); }
    let module_path = path.join(".");
    // Aceptar estilo "std.math.round" — verificar que el prefijo del módulo coincida
    let known = KNOWN_MODULES.iter().any(|m| {
        module_path == *m || module_path.starts_with(&format!("{m}."))
    });
    if known {
        Ok(())
    } else {
        Err(CaperError::ImportNotFound { path: module_path })
    }
}

/// Despacha una llamada a la stdlib por nombre de función.
///
/// Retorna `Ok(Some(value))` si la función fue encontrada y ejecutada.
/// Retorna `Ok(None)` si ningún módulo de stdlib maneja ese nombre (el caller
/// lo buscará como función definida por el usuario).
///
/// El orden de despacho importa para nombres polimórficos (len, count, contains):
/// cada módulo verifica el tipo de argumento y retorna None si no coincide.
pub fn call(name: &str, args: Vec<Value>, line: usize) -> Result<Option<Value>, CaperError> {
    if let Some(r) = math::call(name, &args, line)?    { return Ok(Some(r)); }
    if let Some(r) = strings::call(name, &args, line)? { return Ok(Some(r)); }
    if let Some(r) = arrays::call(name, &args, line)?  { return Ok(Some(r)); }
    if let Some(r) = objects::call(name, &args, line)?  { return Ok(Some(r)); }
    if let Some(r) = types::call(name, &args, line)?   { return Ok(Some(r)); }
    if let Some(r) = json::call(name, &args, line)?    { return Ok(Some(r)); }
    if let Some(r) = dates::call(name, &args, line)?   { return Ok(Some(r)); }
    if let Some(r) = regex::call(name, &args, line)?   { return Ok(Some(r)); }
    if let Some(r) = rand::call(name, &args, line)?    { return Ok(Some(r)); }
    if let Some(r) = crypto::call(name, &args, line)?  { return Ok(Some(r)); }
    if let Some(r) = time::call(name, &args, line)?    { return Ok(Some(r)); }
    if let Some(r) = sort::call(name, &args, line)?    { return Ok(Some(r)); }
    if let Some(r) = search::call(name, &args, line)?  { return Ok(Some(r)); }
    if let Some(r) = xml::call(name, &args, line)?     { return Ok(Some(r)); }
    if let Some(r) = yaml::call(name, &args, line)?    { return Ok(Some(r)); }
    Ok(None)
}
