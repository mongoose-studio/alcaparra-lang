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
        "yaml_encode"      => yaml_encode(args)?,
        "yaml_encode_opts" => yaml_encode_opts(args)?,
        "yaml_decode"      => yaml_decode(args)?,
        "yaml_valid"       => yaml_valid(args)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// yaml_encode(v)
// ---------------------------------------------------------------------------

fn yaml_encode(args: &[Value]) -> Result<Value, CaperError> {
    let val = args.get(0).cloned().unwrap_or(Value::Null);
    let yaml_val = to_yaml_value(&val);
    let s = serde_yaml::to_string(&yaml_val)
        .map_err(|e| CaperError::ArgumentError {
            func: "yaml_encode".to_string(),
            detail: e.to_string(),
        })?;
    Ok(Value::Str(s))
}

// ---------------------------------------------------------------------------
// yaml_encode_opts(v, opts)
// opts: { indent: number (default 2), flow_style: bool }
// Nota: serde_yaml 0.9 no expone configuración de indentación; se post-procesa.
// ---------------------------------------------------------------------------

fn yaml_encode_opts(args: &[Value]) -> Result<Value, CaperError> {
    let val  = args.get(0).cloned().unwrap_or(Value::Null);
    let opts = args.get(1);

    let yaml_val = to_yaml_value(&val);
    let s = serde_yaml::to_string(&yaml_val)
        .map_err(|e| CaperError::ArgumentError {
            func: "yaml_encode_opts".to_string(),
            detail: e.to_string(),
        })?;

    // Ajustar indentación si se solicitó (serde_yaml usa 2 espacios por defecto)
    let indent = match opts {
        Some(Value::Object(m)) => match m.get("indent") {
            Some(v) => v.as_decimal()
                .and_then(|d| { use rust_decimal::prelude::ToPrimitive; d.to_usize() })
                .unwrap_or(2),
            None => 2,
        },
        _ => 2,
    };

    let result = if indent == 2 {
        s
    } else {
        reindent(&s, indent)
    };

    Ok(Value::Str(result))
}

// ---------------------------------------------------------------------------
// yaml_decode(s) → Value
// ---------------------------------------------------------------------------

fn yaml_decode(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "yaml_decode")?;
    let yaml_val: serde_yaml::Value = serde_yaml::from_str(s)
        .map_err(|e| CaperError::ArgumentError {
            func: "yaml_decode".to_string(),
            detail: e.to_string(),
        })?;
    Ok(from_yaml_value(yaml_val))
}

// ---------------------------------------------------------------------------
// yaml_valid(s) → bool
// ---------------------------------------------------------------------------

fn yaml_valid(args: &[Value]) -> Result<Value, CaperError> {
    let ok = args.get(0)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .map(|s| serde_yaml::from_str::<serde_yaml::Value>(s).is_ok())
        .unwrap_or(false);
    Ok(Value::Bool(ok))
}

// ---------------------------------------------------------------------------
// Conversión Value → serde_yaml::Value
// ---------------------------------------------------------------------------

fn to_yaml_value(v: &Value) -> serde_yaml::Value {
    match v {
        Value::Null      => serde_yaml::Value::Null,
        Value::Bool(b)   => serde_yaml::Value::Bool(*b),
        Value::Number(n) => {
            // Preferir representación entera cuando sea posible
            if n.fract().is_zero() {
                if let Ok(i) = i64::from_str(&n.to_string()) {
                    return serde_yaml::Value::Number(i.into());
                }
            }
            if let Ok(f) = f64::from_str(&n.to_string()) {
                serde_yaml::Value::Number(serde_yaml::Number::from(f))
            } else {
                serde_yaml::Value::String(n.to_string())
            }
        }
        Value::Str(s) => serde_yaml::Value::String(s.clone()),
        Value::Array(a) => serde_yaml::Value::Sequence(
            a.iter().map(to_yaml_value).collect()
        ),
        Value::Object(m) => {
            let mapping: serde_yaml::Mapping = m.iter()
                .map(|(k, v)| (serde_yaml::Value::String(k.clone()), to_yaml_value(v)))
                .collect();
            serde_yaml::Value::Mapping(mapping)
        }
        _ => serde_yaml::Value::Null,
    }
}

// ---------------------------------------------------------------------------
// Conversión serde_yaml::Value → Value
// ---------------------------------------------------------------------------

fn from_yaml_value(y: serde_yaml::Value) -> Value {
    match y {
        serde_yaml::Value::Null          => Value::Null,
        serde_yaml::Value::Bool(b)       => Value::Bool(b),
        serde_yaml::Value::Number(n)     => {
            // Intentar preservar precisión via conversión a string
            let s = n.to_string();
            Decimal::from_str(&s)
                .map(Value::Number)
                .unwrap_or(Value::Str(s))
        }
        serde_yaml::Value::String(s)     => Value::Str(s),
        serde_yaml::Value::Sequence(seq) => {
            Value::Array(seq.into_iter().map(from_yaml_value).collect())
        }
        serde_yaml::Value::Mapping(map)  => {
            let mut m = IndexMap::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s,
                    other => format!("{:?}", other),
                };
                m.insert(key, from_yaml_value(v));
            }
            Value::Object(m)
        }
        serde_yaml::Value::Tagged(t)     => from_yaml_value(t.value),
    }
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

/// Re-indent a serde_yaml output (which uses 2 spaces) to n spaces.
fn reindent(s: &str, n: usize) -> String {
    let target = " ".repeat(n);
    s.lines()
        .map(|line| {
            let trimmed = line.trim_start_matches(' ');
            let spaces  = line.len() - trimmed.len();
            let level   = spaces / 2;
            format!("{}{}", target.repeat(level), trimmed)
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

fn require_str<'a>(args: &'a [Value], idx: usize, func: &str) -> Result<&'a str, CaperError> {
    args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a string", idx + 1),
        })
}
