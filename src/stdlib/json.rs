// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use indexmap::IndexMap;
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn call(name: &str, args: &[Value], _line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "json_encode" => json_encode(args)?,
        "json_decode" => json_decode(args)?,
        "json_valid"  => json_valid(args)?,
        "json_pretty" => json_pretty(args)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

fn require_arg<'a>(args: &'a [Value], fn_name: &str) -> Result<&'a Value, CaperError> {
    args.first().ok_or_else(|| CaperError::ArgumentError {
        func: fn_name.to_string(),
        detail: "argument 1 is required".to_string(),
    })
}

fn require_str(args: &[Value], fn_name: &str) -> Result<String, CaperError> {
    match args.first() {
        Some(Value::Str(s)) => Ok(s.clone()),
        Some(other) => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument 1 must be a string, got {}", other.type_name()),
        }),
        None => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: "argument 1 is required".to_string(),
        }),
    }
}

fn json_encode(args: &[Value]) -> Result<Value, CaperError> {
    let v = require_arg(args, "json_encode")?;
    let json_val: serde_json::Value = v.clone().into();
    serde_json::to_string(&json_val)
        .map(Value::Str)
        .map_err(|e| CaperError::JsonParseError { detail: e.to_string() })
}

fn json_pretty(args: &[Value]) -> Result<Value, CaperError> {
    let v = require_arg(args, "json_pretty")?;
    let json_val: serde_json::Value = v.clone().into();
    serde_json::to_string_pretty(&json_val)
        .map(Value::Str)
        .map_err(|e| CaperError::JsonParseError { detail: e.to_string() })
}

fn json_decode(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, "json_decode")?;
    let json_val: serde_json::Value = serde_json::from_str(&s)
        .map_err(|e| CaperError::JsonParseError { detail: e.to_string() })?;
    Ok(json_to_value(json_val))
}

fn json_valid(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, "json_valid")?;
    Ok(Value::Bool(serde_json::from_str::<serde_json::Value>(&s).is_ok()))
}

/// Convierte serde_json::Value → Value de caper.
/// Los números se parsean como Decimal para preservar precisión.
fn json_to_value(j: serde_json::Value) -> Value {
    match j {
        serde_json::Value::Null       => Value::Null,
        serde_json::Value::Bool(b)    => Value::Bool(b),
        serde_json::Value::String(s)  => Value::Str(s),
        serde_json::Value::Number(n)  => {
            // Usar representación de string para parseo de Decimal sin pérdida
            Decimal::from_str(&n.to_string())
                .map(Value::Number)
                .unwrap_or(Value::Null)
        }
        serde_json::Value::Array(arr) => {
            Value::Array(arr.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => {
            let imap: IndexMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, json_to_value(v)))
                .collect();
            Value::Object(imap)
        }
    }
}
