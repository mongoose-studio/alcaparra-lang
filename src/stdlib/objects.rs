// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use indexmap::IndexMap;
use rust_decimal::Decimal;

pub fn call(name: &str, args: &[Value], _line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "keys"   => keys(args)?,
        "values" => obj_values(args)?,
        "entries"=> entries(args)?,
        "has"    => has(args)?,
        "get"    => obj_get(args)?,
        "merge"  => merge(args)?,
        "omit"   => omit(args)?,
        "pick"   => pick(args)?,
        // Nombres que anuncia el catalogo de `caper -m`.
        "set"          => set(args)?,
        "delete"       => delete(args)?,
        "from_entries" => from_entries(args)?,
        "count"  => {
            // count(obj) — solo para objetos; los arrays son manejados en el módulo arrays
            match args.first() {
                Some(Value::Object(_)) => count_obj(args)?,
                _ => return Ok(None),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// --- Helpers internos ---

fn require_obj(args: &[Value], idx: usize, fn_name: &str) -> Result<IndexMap<String, Value>, CaperError> {
    match args.get(idx) {
        Some(Value::Object(m)) => Ok(m.clone()),
        Some(other) => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} must be an object, got {}", idx + 1, other.type_name()),
        }),
        None => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} is required", idx + 1),
        }),
    }
}

fn require_str(args: &[Value], idx: usize, fn_name: &str) -> Result<String, CaperError> {
    match args.get(idx) {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(other) => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} must be a string, got {}", idx + 1, other.type_name()),
        }),
        None => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} is required", idx + 1),
        }),
    }
}

// --- Funciones ---

fn keys(args: &[Value]) -> Result<Value, CaperError> {
    let map = require_obj(args, 0, "keys")?;
    Ok(Value::Array(map.keys().map(|k| Value::Str(k.clone())).collect()))
}

fn obj_values(args: &[Value]) -> Result<Value, CaperError> {
    let map = require_obj(args, 0, "values")?;
    Ok(Value::Array(map.into_values().collect()))
}

fn entries(args: &[Value]) -> Result<Value, CaperError> {
    let map = require_obj(args, 0, "entries")?;
    let pairs = map.into_iter()
        .map(|(k, v)| Value::Array(vec![Value::Str(k), v]))
        .collect();
    Ok(Value::Array(pairs))
}

fn has(args: &[Value]) -> Result<Value, CaperError> {
    let map = require_obj(args, 0, "has")?;
    let key = require_str(args, 1, "has")?;
    Ok(Value::Bool(map.contains_key(&key)))
}

fn obj_get(args: &[Value]) -> Result<Value, CaperError> {
    let map      = require_obj(args, 0, "get")?;
    let key      = require_str(args, 1, "get")?;
    let default  = args.get(2).cloned().unwrap_or(Value::Null);
    Ok(map.get(&key).cloned().unwrap_or(default))
}

fn merge(args: &[Value]) -> Result<Value, CaperError> {
    let mut base  = require_obj(args, 0, "merge")?;
    let overlay   = require_obj(args, 1, "merge")?;
    for (k, v) in overlay {
        base.insert(k, v);
    }
    Ok(Value::Object(base))
}

fn omit(args: &[Value]) -> Result<Value, CaperError> {
    let map = require_obj(args, 0, "omit")?;
    let keys_to_drop = match args.get(1) {
        Some(Value::Array(arr)) => arr.iter()
            .filter_map(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
            .collect::<Vec<_>>(),
        Some(other) => return Err(CaperError::ArgumentError {
            func: "omit".to_string(),
            detail: format!("argument 2 must be an array, got {}", other.type_name()),
        }),
        None => return Err(CaperError::ArgumentError {
            func: "omit".to_string(),
            detail: "argument 2 is required".to_string(),
        }),
    };
    let result = map.into_iter()
        .filter(|(k, _)| !keys_to_drop.contains(k))
        .collect();
    Ok(Value::Object(result))
}

fn pick(args: &[Value]) -> Result<Value, CaperError> {
    let map = require_obj(args, 0, "pick")?;
    let keys_to_keep = match args.get(1) {
        Some(Value::Array(arr)) => arr.iter()
            .filter_map(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
            .collect::<Vec<_>>(),
        Some(other) => return Err(CaperError::ArgumentError {
            func: "pick".to_string(),
            detail: format!("argument 2 must be an array, got {}", other.type_name()),
        }),
        None => return Err(CaperError::ArgumentError {
            func: "pick".to_string(),
            detail: "argument 2 is required".to_string(),
        }),
    };
    let result = map.into_iter()
        .filter(|(k, _)| keys_to_keep.contains(k))
        .collect();
    Ok(Value::Object(result))
}

fn count_obj(args: &[Value]) -> Result<Value, CaperError> {
    let map = require_obj(args, 0, "count")?;
    Ok(Value::Number(Decimal::from(map.len())))
}

/// Devuelve una copia del objeto con la clave puesta en ese valor.
///
/// **No muta el original**, igual que `push` sobre arreglos: en un lenguaje
/// determinista conviene que nada cambie a espaldas de quien llama. Si la clave
/// ya existia se reemplaza su valor y conserva su posicion.
fn set(args: &[Value]) -> Result<Value, CaperError> {
    let mut obj = require_obj(args, 0, "set")?;
    let clave = match args.get(1) {
        Some(Value::Str(k)) => k.clone(),
        _ => return Err(CaperError::ArgumentError {
            func: "set".to_string(),
            detail: "argument 2 must be a string".to_string(),
        }),
    };
    let valor = args.get(2).cloned().unwrap_or(Value::Null);
    obj.insert(clave, valor);
    Ok(Value::Object(obj))
}

/// Devuelve una copia del objeto sin esa clave. Si no estaba, devuelve la copia
/// igual y no falla.
fn delete(args: &[Value]) -> Result<Value, CaperError> {
    let obj = require_obj(args, 0, "delete")?;
    let clave = match args.get(1) {
        Some(Value::Str(k)) => k.clone(),
        _ => return Err(CaperError::ArgumentError {
            func: "delete".to_string(),
            detail: "argument 2 must be a string".to_string(),
        }),
    };
    // `shift_remove` conserva el orden de las demas claves; `swap_remove` no.
    let mut salida = obj;
    salida.shift_remove(&clave);
    Ok(Value::Object(salida))
}

/// Arma un objeto desde pares `[clave, valor]`. Es la vuelta de `entries`.
fn from_entries(args: &[Value]) -> Result<Value, CaperError> {
    let pares = match args.first() {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err(CaperError::ArgumentError {
            func: "from_entries".to_string(),
            detail: "argument 1 must be an array of [key, value] pairs".to_string(),
        }),
    };

    let mut obj = IndexMap::new();
    for par in pares {
        match par {
            Value::Array(ref p) if p.len() == 2 => {
                let clave = match &p[0] {
                    Value::Str(k) => k.clone(),
                    otro => otro.to_string(),
                };
                obj.insert(clave, p[1].clone());
            }
            otro => return Err(CaperError::ArgumentError {
                func: "from_entries".to_string(),
                detail: format!("cada entrada debe ser un par [clave, valor], llego {}", otro.type_name()),
            }),
        }
    }
    Ok(Value::Object(obj))
}
