// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use chrono::{DateTime, Datelike, Local, NaiveDate, TimeZone, Timelike, Utc};
use rust_decimal::Decimal;
use std::cell::Cell;
use std::time::{SystemTime, UNIX_EPOCH};

// Inicio de script thread-local para elapsed_ms()
thread_local! {
    static SCRIPT_START_MS: Cell<u128> = Cell::new({
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    });
}

pub fn call(name: &str, args: &[Value], _line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "timestamp"        => timestamp()?,
        "timestamp_ms"     => timestamp_ms()?,
        "from_timestamp"   => from_timestamp(args)?,
        "from_timestamp_ms"=> from_timestamp_ms(args)?,
        "to_timestamp"     => to_timestamp(args)?,
        "time_now"         => time_now()?,
        "time_format"      => time_format(args)?,
        "elapsed_ms"       => elapsed_ms()?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------

fn timestamp() -> Result<Value, CaperError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(Value::Number(Decimal::from(ts)))
}

fn timestamp_ms() -> Result<Value, CaperError> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    Ok(Value::Number(Decimal::from(ts as u64)))
}

fn from_timestamp(args: &[Value]) -> Result<Value, CaperError> {
    let ts = require_i64(args, 0, "from_timestamp")?;
    let dt = DateTime::from_timestamp(ts, 0)
        .ok_or_else(|| arg_err("from_timestamp", "invalid timestamp"))?;
    Ok(Value::Str(dt.format("%Y-%m-%d").to_string()))
}

fn from_timestamp_ms(args: &[Value]) -> Result<Value, CaperError> {
    let ts_ms = require_i64(args, 0, "from_timestamp_ms")?;
    let secs = ts_ms / 1000;
    let dt = DateTime::from_timestamp(secs, 0)
        .ok_or_else(|| arg_err("from_timestamp_ms", "invalid timestamp"))?;
    Ok(Value::Str(dt.format("%Y-%m-%d").to_string()))
}

fn to_timestamp(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "to_timestamp")?;
    let d = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| arg_err("to_timestamp", &format!("cannot parse date '{s}'")))?;
    let dt = d.and_hms_opt(0, 0, 0).unwrap();
    let ts = Utc.from_utc_datetime(&dt).timestamp();
    Ok(Value::Number(Decimal::from(ts)))
}

fn time_now() -> Result<Value, CaperError> {
    let now = Local::now();
    Ok(Value::Str(format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second())))
}

fn time_format(args: &[Value]) -> Result<Value, CaperError> {
    let ts = require_i64(args, 0, "time_format")?;
    let fmt = require_str(args, 1, "time_format")?;
    let dt = DateTime::from_timestamp(ts, 0)
        .ok_or_else(|| arg_err("time_format", "invalid timestamp"))?
        .naive_local();
    let result = fmt
        .replace("YYYY", &format!("{:04}", dt.year()))
        .replace("MM",   &format!("{:02}", dt.month()))
        .replace("DD",   &format!("{:02}", dt.day()))
        .replace("HH",   &format!("{:02}", dt.hour()))
        .replace("mm",   &format!("{:02}", dt.minute()))
        .replace("ss",   &format!("{:02}", dt.second()));
    Ok(Value::Str(result))
}

fn elapsed_ms() -> Result<Value, CaperError> {
    let start = SCRIPT_START_MS.with(|c| c.get());
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let elapsed = now.saturating_sub(start);
    Ok(Value::Number(Decimal::from(elapsed as u64)))
}

// ---------------------------------------------------------------------------

fn require_str<'a>(args: &'a [Value], idx: usize, func: &str) -> Result<&'a str, CaperError> {
    args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| arg_err(func, &format!("argument {} must be a string", idx + 1)))
}

fn require_i64(args: &[Value], idx: usize, func: &str) -> Result<i64, CaperError> {
    args.get(idx)
        .and_then(|v| v.as_decimal())
        .and_then(|d| { use rust_decimal::prelude::ToPrimitive; d.to_i64() })
        .ok_or_else(|| arg_err(func, &format!("argument {} must be a number", idx + 1)))
}

fn arg_err(func: &str, detail: &str) -> CaperError {
    CaperError::ArgumentError { func: func.to_string(), detail: detail.to_string() }
}
