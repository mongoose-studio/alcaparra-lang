// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;
use rust_decimal::MathematicalOps;

pub fn call(name: &str, args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "abs"   => abs(args, line)?,
        "ceil"  => ceil(args, line)?,
        "floor" => floor(args, line)?,
        "round" => round(args, line)?,
        "trunc" => trunc(args, line)?,
        "max"   => max(args, line)?,
        "min"   => min(args, line)?,
        "pow"   => pow(args, line)?,
        "sqrt"  => sqrt(args, line)?,
        "sign"  => sign(args, line)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// --- Helpers internos ---

fn require_number(args: &[Value], idx: usize, fn_name: &str, _line: usize) -> Result<Decimal, CaperError> {
    args.get(idx)
        .and_then(|v| v.as_decimal())
        .ok_or_else(|| CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} must be a number", idx + 1),
        })
}

fn require_usize(args: &[Value], idx: usize, fn_name: &str, line: usize) -> Result<u32, CaperError> {
    let n = require_number(args, idx, fn_name, line)?;
    n.to_u32().ok_or_else(|| CaperError::ArgumentError {
        func: fn_name.to_string(),
        detail: format!("argument {} must be a non-negative integer", idx + 1),
    })
}

// --- Funciones ---

fn abs(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "abs", line)?;
    Ok(Value::Number(n.abs()))
}

fn ceil(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "ceil", line)?;
    Ok(Value::Number(n.ceil()))
}

fn floor(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "floor", line)?;
    Ok(Value::Number(n.floor()))
}

fn round(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "round", line)?;
    let decimals = if args.len() > 1 {
        require_usize(args, 1, "round", line)?
    } else {
        0
    };
    Ok(Value::Number(n.round_dp(decimals)))
}

fn trunc(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "trunc", line)?;
    let decimals = if args.len() > 1 {
        require_usize(args, 1, "trunc", line)?
    } else {
        0
    };
    Ok(Value::Number(n.trunc_with_scale(decimals)))
}

fn max(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let a = require_number(args, 0, "max", line)?;
    let b = require_number(args, 1, "max", line)?;
    Ok(Value::Number(a.max(b)))
}

fn min(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let a = require_number(args, 0, "min", line)?;
    let b = require_number(args, 1, "min", line)?;
    Ok(Value::Number(a.min(b)))
}

fn pow(args: &[Value], line: usize) -> Result<Value, CaperError> {
    use rust_decimal::prelude::ToPrimitive;
    let base = require_number(args, 0, "pow", line)?;
    let exp  = require_number(args, 1, "pow", line)?;
    let exp_i = exp.to_i64().ok_or_else(|| CaperError::ArgumentError {
        func: "pow".to_string(),
        detail: "exponent must be an integer".to_string(),
    })?;
    Ok(Value::Number(base.powi(exp_i)))
}

fn sqrt(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "sqrt", line)?;
    if n < Decimal::ZERO {
        return Err(CaperError::ArgumentError {
            func: "sqrt".to_string(),
            detail: "cannot take square root of a negative number".to_string(),
        });
    }
    // sqrt() returns Option<Decimal>; None only for negatives (already checked above)
    Ok(Value::Number(n.sqrt().unwrap_or(Decimal::ZERO)))
}

fn sign(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "sign", line)?;
    let s = if n > Decimal::ZERO {
        Decimal::ONE
    } else if n < Decimal::ZERO {
        Decimal::NEGATIVE_ONE
    } else {
        Decimal::ZERO
    };
    Ok(Value::Number(s))
}