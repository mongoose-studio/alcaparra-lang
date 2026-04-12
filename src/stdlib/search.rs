// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use rust_decimal::Decimal;

/// Nombres de HOF que aporta este módulo — manejados por la VM, no aquí.
/// Declarados aquí para documentación; el registro está en HOF_NAMES de arrays.rs.
/// HOFs: find_all, find_index, group_by, partition

pub fn call(name: &str, args: &[Value], _line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "binary_search" => binary_search(args)?,
        "in_range"      => in_range(args)?,
        "search"        => linear_search(args)?,
        "fuzzy"         => fuzzy(args)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// binary_search(arr, val) — asume array ordenado, retorna índice o -1
// ---------------------------------------------------------------------------

fn binary_search(args: &[Value]) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "binary_search")?;
    let target = args.get(1).cloned().unwrap_or(Value::Null);

    let mut lo = 0i64;
    let mut hi = arr.len() as i64 - 1;

    while lo <= hi {
        let mid = (lo + hi) / 2;
        match cmp_values(&arr[mid as usize], &target) {
            std::cmp::Ordering::Equal   => return Ok(Value::Number(Decimal::from(mid))),
            std::cmp::Ordering::Less    => lo = mid + 1,
            std::cmp::Ordering::Greater => hi = mid - 1,
        }
    }
    Ok(Value::Number(Decimal::from(-1i64)))
}

// ---------------------------------------------------------------------------
// in_range(val, min, max) — inclusivo en ambos extremos
// ---------------------------------------------------------------------------

fn in_range(args: &[Value]) -> Result<Value, CaperError> {
    let val = args.get(0).cloned().unwrap_or(Value::Null);
    let min = args.get(1).cloned().unwrap_or(Value::Null);
    let max = args.get(2).cloned().unwrap_or(Value::Null);
    let ok = cmp_values(&val, &min) != std::cmp::Ordering::Less
          && cmp_values(&val, &max) != std::cmp::Ordering::Greater;
    Ok(Value::Bool(ok))
}

// ---------------------------------------------------------------------------
// search(arr, val) — búsqueda lineal, retorna índice o -1
// ---------------------------------------------------------------------------

fn linear_search(args: &[Value]) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "search")?;
    let target = args.get(1).cloned().unwrap_or(Value::Null);
    for (i, v) in arr.iter().enumerate() {
        if *v == target {
            return Ok(Value::Number(Decimal::from(i as i64)));
        }
    }
    Ok(Value::Number(Decimal::from(-1i64)))
}

// ---------------------------------------------------------------------------
// fuzzy(s, pattern, threshold) — similitud normalizada [0.0, 1.0]
// Usa distancia de Levenshtein normalizada por max(len(s), len(patron)).
// Retorna true si similitud >= threshold (por defecto 0.8).
// ---------------------------------------------------------------------------

fn fuzzy(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "fuzzy")?;
    let pattern = require_str(args, 1, "fuzzy")?;
    let threshold = args.get(2)
        .and_then(|v| v.as_decimal())
        .and_then(|d| { use rust_decimal::prelude::ToPrimitive; d.to_f64() })
        .unwrap_or(0.8);

    let sim = similarity(s, pattern);
    Ok(Value::Bool(sim >= threshold))
}

/// Normalized Levenshtein similarity: 1 - (edit_distance / max_len)
fn similarity(a: &str, b: &str) -> f64 {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let la = a_chars.len();
    let lb = b_chars.len();
    if la == 0 && lb == 0 { return 1.0; }
    let max_len = la.max(lb);
    let dist = levenshtein(&a_chars, &b_chars);
    1.0 - (dist as f64 / max_len as f64)
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    let la = a.len();
    let lb = b.len();
    let mut dp = vec![0usize; lb + 1];
    for j in 0..=lb { dp[j] = j; }
    for i in 1..=la {
        let mut prev = dp[0];
        dp[0] = i;
        for j in 1..=lb {
            let temp = dp[j];
            dp[j] = if a[i - 1] == b[j - 1] {
                prev
            } else {
                1 + prev.min(dp[j]).min(dp[j - 1])
            };
            prev = temp;
        }
    }
    dp[lb]
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

fn require_arr(args: &[Value], idx: usize, func: &str) -> Result<Vec<Value>, CaperError> {
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
