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
        "round_half_up" => round_half_up(args, line)?,
        "trunc" => trunc(args, line)?,
        "max"   => match max(args, line)? { Some(v) => v, None => return Ok(None) },
        "min"   => match min(args, line)? { Some(v) => v, None => return Ok(None) },
        "pow"   => pow(args, line)?,
        "sqrt"  => sqrt(args, line)?,
        "sign"  => sign(args, line)?,
        "clamp" => clamp(args, line)?,
        "log"   => log(args, line)?,
        "log10" => log10(args, line)?,
        "exp"   => exp(args, line)?,
        "pi"    => Value::Number(Decimal::PI),
        "e"     => Value::Number(Decimal::E),
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

/// El segundo argumento de `round` y `round_half_up`: cuantos decimales, y
/// **puede ser negativo** para redondear a decenas, centenas y asi. Ausente, 0.
fn require_decimales(args: &[Value], fn_name: &str, _line: usize) -> Result<i32, CaperError> {
    if args.len() < 2 {
        return Ok(0);
    }
    args.get(1)
        .and_then(|v| v.as_decimal())
        .and_then(|d| d.to_i32())
        .ok_or_else(|| CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: "argument 2 must be an integer".to_string(),
        })
}

/// Redondea con la estrategia dada, admitiendo decimales negativos.
///
/// `rust_decimal` solo redondea a una cantidad no negativa de decimales, asi que
/// para los negativos se escala: dividir por la potencia de diez, redondear al
/// entero y multiplicar de vuelta. `round(1234.567, -2)` da 1200.
fn redondear(n: Decimal, decimales: i32, estrategia: RoundingStrategy) -> Option<Decimal> {
    if decimales >= 0 {
        return Some(n.round_dp_with_strategy(decimales as u32, estrategia));
    }
    let factor = Decimal::from(10i64.checked_pow(decimales.unsigned_abs())?);
    n.checked_div(factor)?
        .round_dp_with_strategy(0, estrategia)
        .checked_mul(factor)
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
    let decimales = require_decimales(args, "round", line)?;
    redondear(n, decimales, RoundingStrategy::MidpointNearestEven)
        .map(Value::Number)
        .ok_or_else(|| no_representable("round"))
}

/// Redondeo **medio hacia arriba**: el 0,5 sube siempre.
///
/// `round` usa la estrategia por defecto de `rust_decimal`, que es medio al par
/// —0,5 → 0 · 1,5 → 2 · 2,5 → 2—. Es correcta para estadistica, pero la
/// contabilidad y las liquidaciones de sueldo en Chile redondean medio arriba,
/// y la diferencia de un peso en una liquidacion es un reclamo.
///
/// El catalogo de `std.math` ya anunciaba esta funcion; esta es su implementacion.
fn round_half_up(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "round_half_up", line)?;
    let decimales = require_decimales(args, "round_half_up", line)?;
    redondear(n, decimales, RoundingStrategy::MidpointAwayFromZero)
        .map(Value::Number)
        .ok_or_else(|| no_representable("round_half_up"))
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

/// `max(a, b)` sobre numeros. Devuelve `None` cuando el primer argumento no es
/// un numero — ver la nota de `min`.
fn max(args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    if args.first().and_then(|v| v.as_decimal()).is_none() {
        return Ok(None);
    }
    let a = require_number(args, 0, "max", line)?;
    let b = require_number(args, 1, "max", line)?;
    Ok(Some(Value::Number(a.max(b))))
}

/// `min(a, b)` sobre numeros. Devuelve `None` —no un error— cuando el primer
/// argumento no es un numero, para que el despacho siga hacia `std.arrays.min`,
/// que opera sobre un arreglo. El orden de los modulos hace que `math` vea estos
/// nombres primero, asi que fallar aca dejaba `min([1, 2])` inalcanzable.
fn min(args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    if args.first().and_then(|v| v.as_decimal()).is_none() {
        return Ok(None);
    }
    let a = require_number(args, 0, "min", line)?;
    let b = require_number(args, 1, "min", line)?;
    Ok(Some(Value::Number(a.min(b))))
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

/// Acota un valor al rango `[minimo, maximo]`.
fn clamp(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n   = require_number(args, 0, "clamp", line)?;
    let min = require_number(args, 1, "clamp", line)?;
    let max = require_number(args, 2, "clamp", line)?;

    if min > max {
        return Err(CaperError::ArgumentError {
            func: "clamp".to_string(),
            detail: format!("el minimo ({min}) no puede ser mayor que el maximo ({max})"),
        });
    }

    Ok(Value::Number(n.max(min).min(max)))
}

/// Logaritmo natural, o en la base dada como segundo argumento.
///
/// A diferencia del resto del modulo, el resultado es una **aproximacion**: no
/// hay un logaritmo exacto en decimal. Sigue siendo determinista —misma entrada,
/// misma salida— pero no se usa para plata.
fn log(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "log", line)?;
    exigir_positivo(n, "log")?;

    let natural = n.checked_ln().ok_or_else(|| no_representable("log"))?;

    if args.len() < 2 {
        return Ok(Value::Number(natural));
    }

    let base = require_number(args, 1, "log", line)?;
    exigir_positivo(base, "log")?;

    if base == Decimal::ONE {
        return Err(CaperError::ArgumentError {
            func: "log".to_string(),
            detail: "la base no puede ser 1".to_string(),
        });
    }

    let base_ln = base.checked_ln().ok_or_else(|| no_representable("log"))?;
    natural
        .checked_div(base_ln)
        .map(Value::Number)
        .ok_or_else(|| no_representable("log"))
}

fn log10(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "log10", line)?;
    exigir_positivo(n, "log10")?;
    n.checked_log10().map(Value::Number).ok_or_else(|| no_representable("log10"))
}

fn exp(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let n = require_number(args, 0, "exp", line)?;
    n.checked_exp().map(Value::Number).ok_or_else(|| no_representable("exp"))
}

fn exigir_positivo(n: Decimal, fn_name: &str) -> Result<(), CaperError> {
    if n <= Decimal::ZERO {
        return Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("el argumento debe ser positivo, llego {n}"),
        });
    }
    Ok(())
}

fn no_representable(fn_name: &str) -> CaperError {
    CaperError::ArgumentError {
        func: fn_name.to_string(),
        detail: "el resultado no es representable como decimal".to_string(),
    }
}
