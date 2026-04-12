// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use rand::prelude::*;
use rust_decimal::Decimal;
use std::cell::Cell;
use std::str::FromStr;
use uuid::Uuid;

// Semilla RNG thread-local para que `seed(n)` afecte las llamadas posteriores en el mismo thread.
thread_local! {
    static RNG_SEED: Cell<Option<u64>> = Cell::new(None);
}

fn make_rng() -> SmallRng {
    RNG_SEED.with(|cell| {
        if let Some(s) = cell.get() {
            SmallRng::seed_from_u64(s)
        } else {
            SmallRng::from_entropy()
        }
    })
}

pub fn call(name: &str, args: &[Value], _line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "rand"        => rand_fn(args)?,
        "rand_int"    => rand_int(args)?,
        "rand_float"  => rand_float(args)?,
        "rand_item"   => rand_item(args)?,
        "rand_sample" => rand_sample(args)?,
        "shuffle"     => shuffle(args)?,
        "seed"        => seed_fn(args)?,
        "uuid"        => uuid_fn()?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// rand() → 0.0 .. 1.0
// ---------------------------------------------------------------------------

fn rand_fn(_args: &[Value]) -> Result<Value, CaperError> {
    let v: f64 = make_rng().r#gen();
    Ok(Value::Number(Decimal::from_str(&format!("{v:.10}")).unwrap_or(Decimal::ZERO)))
}

// ---------------------------------------------------------------------------
// rand_int(min, max) → entero en [min, max]
// ---------------------------------------------------------------------------

fn rand_int(args: &[Value]) -> Result<Value, CaperError> {
    let min = require_i64(args, 0, "rand_int")?;
    let max = require_i64(args, 1, "rand_int")?;
    if min > max {
        return Err(CaperError::ArgumentError {
            func: "rand_int".to_string(),
            detail: format!("min ({min}) must be ≤ max ({max})"),
        });
    }
    let v = make_rng().gen_range(min..=max);
    Ok(Value::Number(Decimal::from(v)))
}

// ---------------------------------------------------------------------------
// rand_float(min, max) → decimal en [min, max]
// ---------------------------------------------------------------------------

fn rand_float(args: &[Value]) -> Result<Value, CaperError> {
    let min = require_f64(args, 0, "rand_float")?;
    let max = require_f64(args, 1, "rand_float")?;
    if min > max {
        return Err(CaperError::ArgumentError {
            func: "rand_float".to_string(),
            detail: format!("min must be ≤ max"),
        });
    }
    let v: f64 = make_rng().gen_range(min..=max);
    Ok(Value::Number(Decimal::from_str(&format!("{v:.10}")).unwrap_or(Decimal::ZERO)))
}

// ---------------------------------------------------------------------------
// rand_item(arr) → elemento aleatorio o null
// ---------------------------------------------------------------------------

fn rand_item(args: &[Value]) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "rand_item")?;
    if arr.is_empty() { return Ok(Value::Null); }
    let idx = make_rng().gen_range(0..arr.len());
    Ok(arr[idx].clone())
}

// ---------------------------------------------------------------------------
// rand_sample(arr, n) → n elementos aleatorios distintos
// ---------------------------------------------------------------------------

fn rand_sample(args: &[Value]) -> Result<Value, CaperError> {
    let arr = require_arr(args, 0, "rand_sample")?;
    let n = require_i64(args, 1, "rand_sample")? as usize;
    if n > arr.len() {
        return Err(CaperError::ArgumentError {
            func: "rand_sample".to_string(),
            detail: format!("n ({n}) exceeds array length ({})", arr.len()),
        });
    }
    let mut rng = make_rng();
    let sample: Vec<Value> = arr.choose_multiple(&mut rng, n).cloned().collect();
    Ok(Value::Array(sample))
}

// ---------------------------------------------------------------------------
// shuffle(arr) → copia mezclada aleatoriamente
// ---------------------------------------------------------------------------

fn shuffle(args: &[Value]) -> Result<Value, CaperError> {
    let mut arr = require_arr(args, 0, "shuffle")?;
    arr.shuffle(&mut make_rng());
    Ok(Value::Array(arr))
}

// ---------------------------------------------------------------------------
// seed(n) — fijar semilla RNG para reproducibilidad
// ---------------------------------------------------------------------------

fn seed_fn(args: &[Value]) -> Result<Value, CaperError> {
    let n = require_i64(args, 0, "seed")? as u64;
    RNG_SEED.with(|cell| cell.set(Some(n)));
    Ok(Value::Null)
}

// ---------------------------------------------------------------------------
// uuid() → string UUID v4
// ---------------------------------------------------------------------------

fn uuid_fn() -> Result<Value, CaperError> {
    Ok(Value::Str(Uuid::new_v4().to_string()))
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

fn require_i64(args: &[Value], idx: usize, func: &str) -> Result<i64, CaperError> {
    args.get(idx)
        .and_then(|v| v.as_decimal())
        .and_then(|d| {
            use rust_decimal::prelude::ToPrimitive;
            d.to_i64()
        })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be an integer", idx + 1),
        })
}

fn require_f64(args: &[Value], idx: usize, func: &str) -> Result<f64, CaperError> {
    args.get(idx)
        .and_then(|v| v.as_decimal())
        .and_then(|d| {
            use rust_decimal::prelude::ToPrimitive;
            d.to_f64()
        })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a number", idx + 1),
        })
}
