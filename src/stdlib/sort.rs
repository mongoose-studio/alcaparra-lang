// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use indexmap::IndexMap;

pub fn call(name: &str, args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "sort_asc"      => sort_asc(args, line)?,
        "sort_desc"     => sort_desc(args, line)?,
        "sort_by_desc"  => sort_by_desc(args, line)?,
        "order_by"      => order_by(args, line)?,
        "group_by_key"  => group_by_key(args, line)?,
        "chunk"         => chunk(args, line)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// sort_asc / sort_desc
// ---------------------------------------------------------------------------

fn sort_asc(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "sort_asc", line)?;
    arr.sort_by(|a, b| cmp_values(a, b));
    Ok(Value::Array(arr))
}

fn sort_desc(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "sort_desc", line)?;
    arr.sort_by(|a, b| cmp_values(b, a));
    Ok(Value::Array(arr))
}

// ---------------------------------------------------------------------------
// sort_by_desc(arr, key) — descendente por campo de objeto
// ---------------------------------------------------------------------------

fn sort_by_desc(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "sort_by_desc", line)?;
    let key = require_str(args, 1, "sort_by_desc")?;
    arr.sort_by(|a, b| {
        let va = field_val(b, key); // reversed for desc
        let vb = field_val(a, key);
        cmp_values(&va, &vb)
    });
    Ok(Value::Array(arr))
}

// ---------------------------------------------------------------------------
// order_by(arr, [ ["field", "asc"|"desc"], ... ])
// ---------------------------------------------------------------------------

fn order_by(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "order_by", line)?;

    let criteria = match args.get(1) {
        Some(Value::Array(a)) => a.clone(),
        _ => return Err(CaperError::ArgumentError {
            func: "order_by".to_string(),
            detail: "argument 2 must be an array of [field, direction] pairs".to_string(),
        }),
    };

    // Parse criteria: [ ["campo", "asc"], ["otro", "desc"] ]
    let mut parsed: Vec<(String, bool)> = vec![]; // (field, asc)
    for criterion in &criteria {
        match criterion {
            Value::Array(pair) if pair.len() == 2 => {
                let field = match &pair[0] {
                    Value::Str(s) => s.clone(),
                    _ => return Err(CaperError::ArgumentError {
                        func: "order_by".to_string(),
                        detail: "criterion field must be a string".to_string(),
                    }),
                };
                let asc = match &pair[1] {
                    Value::Str(d) => d != "desc",
                    _ => true,
                };
                parsed.push((field, asc));
            }
            _ => return Err(CaperError::ArgumentError {
                func: "order_by".to_string(),
                detail: "each criterion must be [\"field\", \"asc\"|\"desc\"]".to_string(),
            }),
        }
    }

    arr.sort_by(|a, b| {
        for (field, asc) in &parsed {
            let va = field_val(a, field);
            let vb = field_val(b, field);
            let ord = if *asc { cmp_values(&va, &vb) } else { cmp_values(&vb, &va) };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });

    Ok(Value::Array(arr))
}

// ---------------------------------------------------------------------------
// group_by_key(arr, key) — agrupar array de objetos por un campo string
// ---------------------------------------------------------------------------

fn group_by_key(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "group_by_key", line)?;
    let key = require_str(args, 1, "group_by_key")?;

    let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();
    for item in arr {
        let k = match field_val(&item, key) {
            Value::Str(s) => s,
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => "null".to_string(),
        };
        groups.entry(k).or_default().push(item);
    }

    let result: IndexMap<String, Value> = groups
        .into_iter()
        .map(|(k, v)| (k, Value::Array(v)))
        .collect();
    Ok(Value::Object(result))
}

// ---------------------------------------------------------------------------
// chunk(arr, n) — dividir en sub-arrays de tamaño n
// ---------------------------------------------------------------------------

fn chunk(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "chunk", line)?;
    let n = require_usize(args, 1, "chunk")?;
    if n == 0 {
        return Err(CaperError::ArgumentError {
            func: "chunk".to_string(),
            detail: "chunk size must be > 0".to_string(),
        });
    }
    let chunks: Vec<Value> = arr.chunks(n)
        .map(|c| Value::Array(c.to_vec()))
        .collect();
    Ok(Value::Array(chunks))
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

fn require_arr(args: &[Value], idx: usize, func: &str, _line: usize) -> Result<Vec<Value>, CaperError> {
    match args.get(idx) {
        Some(Value::Array(a)) => Ok(a.clone()),
        _ => Err(CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be an array", idx + 1),
        }),
    }
}

fn require_str<'a>(args: &'a [Value], idx: usize, func: &str) -> Result<&'a str, CaperError> {
    args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a string", idx + 1),
        })
}

fn require_usize(args: &[Value], idx: usize, func: &str) -> Result<usize, CaperError> {
    args.get(idx)
        .and_then(|v| v.as_decimal())
        .and_then(|d| { use rust_decimal::prelude::ToPrimitive; d.to_usize() })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a positive integer", idx + 1),
        })
}

fn field_val(v: &Value, key: &str) -> Value {
    match v {
        Value::Object(m) => m.get(key).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn cmp_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => x.cmp(y),
        (Value::Str(x), Value::Str(y)) => x.cmp(y),
        (Value::Bool(x), Value::Bool(y)) => x.cmp(y),
        (Value::Null, Value::Null) => std::cmp::Ordering::Equal,
        (Value::Null, _) => std::cmp::Ordering::Less,
        (_, Value::Null) => std::cmp::Ordering::Greater,
        _ => std::cmp::Ordering::Equal,
    }
}
