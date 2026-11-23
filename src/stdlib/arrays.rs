// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

/// Nombres de las funciones HOF que requieren soporte de callback en la VM.
/// Son interceptadas por la VM *antes* de llegar a este módulo.
pub const HOF_NAMES: &[&str] = &[
    "map", "filter", "reduce", "find", "any", "all", "sort_by",
    // std.search HOFs
    "find_all", "find_index",
    // std.sort HOFs
    "group_by", "partition",
];

pub fn call(name: &str, args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    // Los HOF son manejados por la VM — no debería llegar aquí, pero se guarda por precaución
    if HOF_NAMES.contains(&name) {
        return Ok(None);
    }

    let result = match name {
        "count" => {
            // count(arr) — solo para arrays; los objetos se manejan por separado
            match args.first() {
                Some(Value::Array(_)) => count(args)?,
                _ => return Ok(None),
            }
        }
        "len" => {
            match args.first() {
                Some(Value::Array(_)) => count(args)?,
                _ => return Ok(None),
            }
        }
        "contains" => {
            match args.first() {
                Some(Value::Array(_)) => arr_contains(args, line)?,
                _ => return Ok(None),
            }
        }
        "push"         => push(args, line)?,
        "pop"          => pop(args, line)?,
        "unshift"      => unshift(args, line)?,
        "first"        => first(args, line)?,
        "last"         => last(args, line)?,
        "slice"        => slice(args, line)?,
        "reverse"      => reverse(args, line)?,
        "sort"         => sort(args, line)?,
        "unique"       => unique(args, line)?,
        "flatten"      => flatten(args, line)?,
        "flatten_deep" => flatten_deep(args, line)?,
        "index_of" => {
            match args.first() {
                Some(Value::Array(_)) => index_of(args, line)?,
                _ => return Ok(None),
            }
        }
        "sum"          => sum(args, line)?,
        "avg"          => avg(args, line)?,
        // Nombres que el catalogo de `caper -m` anuncia. `min` y `max` llegan aca
        // porque `std.math` cede cuando el primer argumento no es un numero.
        "includes" => {
            match args.first() {
                Some(Value::Array(_)) => arr_contains(args, line)?,
                _ => return Ok(None),
            }
        }
        "flat"         => flatten(args, line)?,
        "min"          => min_of(args, line)?,
        "max"          => max_of(args, line)?,
        "shift"        => shift(args, line)?,
        "concat"       => concat(args, line)?,
        "min_of"       => min_of(args, line)?,
        "max_of"       => max_of(args, line)?,
        "zip"          => zip(args, line)?,
        "range"        => range(args, line)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// --- Helpers internos ---

fn require_arr(args: &[Value], idx: usize, fn_name: &str) -> Result<Vec<Value>, CaperError> {
    match args.get(idx) {
        Some(Value::Array(a)) => Ok(a.clone()),
        Some(other) => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} must be an array, got {}", idx + 1, other.type_name()),
        }),
        None => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} is required", idx + 1),
        }),
    }
}

fn require_decimal(args: &[Value], idx: usize, fn_name: &str) -> Result<Decimal, CaperError> {
    args.get(idx)
        .and_then(|v| v.as_decimal())
        .ok_or_else(|| CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} must be a number", idx + 1),
        })
}

// --- Funciones ---

fn count(args: &[Value]) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "count")?;
    Ok(Value::Number(Decimal::from(arr.len())))
}

fn push(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "push")?;
    let item = args.get(1).cloned().ok_or_else(|| CaperError::ArgumentError {
        func: "push".to_string(),
        detail: "argument 2 is required".to_string(),
    })?;
    arr.push(item);
    Ok(Value::Array(arr))
}

fn pop(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "pop")?;
    if arr.is_empty() {
        return Err(CaperError::ArgumentError {
            func: "pop".to_string(),
            detail: "cannot pop from empty array".to_string(),
        });
    }
    let last = arr.pop().unwrap();
    Ok(Value::Array(vec![last, Value::Array(arr)]))
}

fn unshift(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "unshift")?;
    let item = args.get(1).cloned().ok_or_else(|| CaperError::ArgumentError {
        func: "unshift".to_string(),
        detail: "argument 2 is required".to_string(),
    })?;
    let mut result = vec![item];
    result.extend(arr);
    Ok(Value::Array(result))
}

fn first(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "first")?;
    Ok(arr.into_iter().next().unwrap_or(Value::Null))
}

fn last(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "last")?;
    Ok(arr.into_iter().last().unwrap_or(Value::Null))
}

fn slice(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr   = require_arr(args, 0, "slice")?;
    let start = require_decimal(args, 1, "slice")?
        .to_i64().unwrap_or(0).max(0) as usize;
    let end = if args.len() > 2 {
        require_decimal(args, 2, "slice")?
            .to_i64().unwrap_or(arr.len() as i64).max(0) as usize
    } else {
        arr.len()
    };
    let end = end.min(arr.len());
    let start = start.min(end);
    Ok(Value::Array(arr[start..end].to_vec()))
}

fn reverse(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "reverse")?;
    arr.reverse();
    Ok(Value::Array(arr))
}

fn sort(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "sort")?;
    let mut err: Option<CaperError> = None;
    arr.sort_by(|a, b| {
        if err.is_some() { return std::cmp::Ordering::Equal; }
        match cmp_values(a, b, line) {
            Ok(ord) => ord,
            Err(e)  => { err = Some(e); std::cmp::Ordering::Equal }
        }
    });
    if let Some(e) = err { return Err(e); }
    Ok(Value::Array(arr))
}

fn unique(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "unique")?;
    let mut seen: Vec<Value> = vec![];
    for v in arr {
        if !seen.contains(&v) {
            seen.push(v);
        }
    }
    Ok(Value::Array(seen))
}

fn flatten(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "flatten")?;
    let mut result = vec![];
    for v in arr {
        match v {
            Value::Array(inner) => result.extend(inner),
            other               => result.push(other),
        }
    }
    Ok(Value::Array(result))
}

fn flatten_deep(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "flatten_deep")?;
    Ok(Value::Array(flatten_recursive(arr)))
}

fn flatten_recursive(arr: Vec<Value>) -> Vec<Value> {
    let mut result = vec![];
    for v in arr {
        match v {
            Value::Array(inner) => result.extend(flatten_recursive(inner)),
            other               => result.push(other),
        }
    }
    result
}

fn arr_contains(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "contains")?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(Value::Bool(arr.contains(&val)))
}

fn index_of(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "index_of")?;
    let val = args.get(1).cloned().unwrap_or(Value::Null);
    let idx = arr.iter().position(|v| v == &val)
        .map(|i| i as i64)
        .unwrap_or(-1);
    Ok(Value::Number(Decimal::from(idx)))
}

fn sum(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "sum")?;
    let mut total = Decimal::ZERO;
    for v in &arr {
        total += v.as_decimal().ok_or_else(|| CaperError::TypeError {
            line,
            detail: format!("sum: expected number, got {}", v.type_name()),
        })?;
    }
    Ok(Value::Number(total))
}

fn avg(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "avg")?;
    if arr.is_empty() {
        return Ok(Value::Null);
    }
    let mut total = Decimal::ZERO;
    for v in &arr {
        total += v.as_decimal().ok_or_else(|| CaperError::TypeError {
            line,
            detail: format!("avg: expected number, got {}", v.type_name()),
        })?;
    }
    Ok(Value::Number(total / Decimal::from(arr.len())))
}

fn min_of(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "min_of")?;
    if arr.is_empty() { return Ok(Value::Null); }
    let mut min = arr[0].as_decimal().ok_or_else(|| CaperError::TypeError {
        line, detail: format!("min_of: expected number, got {}", arr[0].type_name()),
    })?;
    for v in &arr[1..] {
        let n = v.as_decimal().ok_or_else(|| CaperError::TypeError {
            line, detail: format!("min_of: expected number, got {}", v.type_name()),
        })?;
        if n < min { min = n; }
    }
    Ok(Value::Number(min))
}

fn max_of(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "max_of")?;
    if arr.is_empty() { return Ok(Value::Null); }
    let mut max = arr[0].as_decimal().ok_or_else(|| CaperError::TypeError {
        line, detail: format!("max_of: expected number, got {}", arr[0].type_name()),
    })?;
    for v in &arr[1..] {
        let n = v.as_decimal().ok_or_else(|| CaperError::TypeError {
            line, detail: format!("max_of: expected number, got {}", v.type_name()),
        })?;
        if n > max { max = n; }
    }
    Ok(Value::Number(max))
}

fn zip(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let a = require_arr(args, 0, "zip")?;
    let b = require_arr(args, 1, "zip")?;
    let result = a.into_iter().zip(b)
        .map(|(x, y)| Value::Array(vec![x, y]))
        .collect();
    Ok(Value::Array(result))
}

fn range(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let start = require_decimal(args, 0, "range")?.to_i64().unwrap_or(0);
    let end   = require_decimal(args, 1, "range")?.to_i64().unwrap_or(0);
    let step  = if args.len() > 2 {
        require_decimal(args, 2, "range")?.to_i64().unwrap_or(1)
    } else {
        1
    };
    if step == 0 {
        return Err(CaperError::ArgumentError {
            func: "range".to_string(),
            detail: "step cannot be zero".to_string(),
        });
    }
    let mut arr = vec![];
    let mut i = start;
    while (step > 0 && i < end) || (step < 0 && i > end) {
        arr.push(Value::Number(Decimal::from(i)));
        i += step;
    }
    Ok(Value::Array(arr))
}

// --- Helpers de ordenamiento ---

pub fn cmp_values(a: &Value, b: &Value, line: usize) -> Result<std::cmp::Ordering, CaperError> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Ok(x.cmp(y)),
        (Value::Str(x), Value::Str(y))       => Ok(x.cmp(y)),
        (Value::Bool(x), Value::Bool(y))     => Ok(x.cmp(y)),
        _ => Err(CaperError::TypeError {
            line,
            detail: format!("cannot compare {} and {}", a.type_name(), b.type_name()),
        }),
    }
}

/// Saca el primer elemento y devuelve el resto. Es el espejo de `pop`, que saca
/// el ultimo: ninguna de las dos muta el arreglo original.
fn shift(args: &[Value], fn_line: usize) -> Result<Value, CaperError> {
    let _ = fn_line;
    let arr = require_arr(args, 0, "shift")?;
    if arr.is_empty() {
        return Ok(Value::Array(vec![]));
    }
    Ok(Value::Array(arr[1..].to_vec()))
}

/// Une dos o mas arreglos en uno. Sin argumentos devuelve el arreglo vacio.
fn concat(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let mut salida = vec![];
    for (i, _) in args.iter().enumerate() {
        salida.extend(require_arr(args, i, "concat")?);
    }
    Ok(Value::Array(salida))
}
