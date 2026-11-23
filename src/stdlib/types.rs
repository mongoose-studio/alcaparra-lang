// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;

pub fn call(name: &str, args: &[Value], _line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "type_of"   => type_of(args)?,
        "is_null"   => is_null(args)?,
        "is_number" => is_number(args)?,
        "is_string" => is_string(args)?,
        "is_bool"   => is_bool(args)?,
        "is_array"  => is_array(args)?,
        "is_object" => is_object(args)?,
        "number"      => to_number(args)?,
        // Nombres que anuncia el catalogo de `caper -m`. `to_string` lo resuelve
        // `std.strings.str`, que el despacho ve antes que este modulo.
        "to_number"   => to_number(args)?,
        "to_bool"     => to_bool(args)?,
        "is_function" => is_function(args)?,
        "int"       => to_int(args)?,
        "bool"      => to_bool(args)?,
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

fn type_of(args: &[Value]) -> Result<Value, CaperError> {
    let v = require_arg(args, "type_of")?;
    Ok(Value::Str(v.type_name().to_string()))
}

fn is_null(args: &[Value]) -> Result<Value, CaperError> {
    Ok(Value::Bool(matches!(require_arg(args, "is_null")?, Value::Null)))
}

fn is_number(args: &[Value]) -> Result<Value, CaperError> {
    Ok(Value::Bool(matches!(require_arg(args, "is_number")?, Value::Number(_))))
}

fn is_string(args: &[Value]) -> Result<Value, CaperError> {
    Ok(Value::Bool(matches!(require_arg(args, "is_string")?, Value::Str(_))))
}

fn is_bool(args: &[Value]) -> Result<Value, CaperError> {
    Ok(Value::Bool(matches!(require_arg(args, "is_bool")?, Value::Bool(_))))
}

fn is_array(args: &[Value]) -> Result<Value, CaperError> {
    Ok(Value::Bool(matches!(require_arg(args, "is_array")?, Value::Array(_))))
}

fn is_object(args: &[Value]) -> Result<Value, CaperError> {
    Ok(Value::Bool(matches!(require_arg(args, "is_object")?, Value::Object(_))))
}

fn to_number(args: &[Value]) -> Result<Value, CaperError> {
    let v = require_arg(args, "number")?;
    match v.as_decimal() {
        Some(n) => Ok(Value::Number(n)),
        None => Err(CaperError::ArgumentError {
            func: "number".to_string(),
            detail: format!("cannot coerce {} to number", v.type_name()),
        }),
    }
}

fn to_int(args: &[Value]) -> Result<Value, CaperError> {
    let v = require_arg(args, "int")?;
    match v.as_decimal() {
        Some(n) => Ok(Value::Number(n.trunc_with_scale(0))),
        None => Err(CaperError::ArgumentError {
            func: "int".to_string(),
            detail: format!("cannot coerce {} to integer", v.type_name()),
        }),
    }
}

fn to_bool(args: &[Value]) -> Result<Value, CaperError> {
    let v = require_arg(args, "bool")?;
    Ok(Value::Bool(v.is_truthy()))
}

fn is_function(args: &[Value]) -> Result<Value, CaperError> {
    let v = require_arg(args, "is_function")?;
    Ok(Value::Bool(matches!(v, Value::Function(_) | Value::NativeFunction(_))))
}
