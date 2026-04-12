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
