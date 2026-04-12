// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use chrono::{Datelike, Duration, Local, NaiveDate};
use rust_decimal::Decimal;

pub fn call(name: &str, args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "today"         => today()?,
        "now"           => now()?,
        "year"          => year(args, line)?,
        "month"         => month_fn(args, line)?,
        "day"           => day_fn(args, line)?,
        "date_diff"     => date_diff(args, line)?,
        "date_add"      => date_add(args, line)?,
        "date_sub"      => date_sub(args, line)?,
        "date_format"   => date_format(args, line)?,
        "date_parse"    => date_parse_fn(args, line)?,
        "is_valid_date" => is_valid_date(args, line)?,
        "working_days"  => working_days(args, line)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// today / now
// ---------------------------------------------------------------------------

fn today() -> Result<Value, CaperError> {
    let d = Local::now().date_naive();
    Ok(Value::Str(d.format("%Y-%m-%d").to_string()))
}

fn now() -> Result<Value, CaperError> {
    let dt = Local::now();
    Ok(Value::Str(dt.format("%Y-%m-%dT%H:%M:%S").to_string()))
}

// ---------------------------------------------------------------------------
// Extractores de componentes de fecha
// ---------------------------------------------------------------------------

fn year(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let d = require_date(args, 0, "year", line)?;
    Ok(Value::Number(Decimal::from(d.year())))
}

fn month_fn(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let d = require_date(args, 0, "month", line)?;
    Ok(Value::Number(Decimal::from(d.month())))
}

fn day_fn(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let d = require_date(args, 0, "day", line)?;
    Ok(Value::Number(Decimal::from(d.day())))
}

// ---------------------------------------------------------------------------
// date_diff
// ---------------------------------------------------------------------------

fn date_diff(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let d1 = require_date(args, 0, "date_diff", line)?;
    let d2 = require_date(args, 1, "date_diff", line)?;

    let unit = args.get(2)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .unwrap_or("day");

    let result = match unit {
        "day"   => (d2 - d1).num_days(),
        "month" => {
            let months = (d2.year() - d1.year()) * 12 + (d2.month() as i32 - d1.month() as i32);
            months as i64
        }
        "year"  => (d2.year() - d1.year()) as i64,
        other => return Err(CaperError::ArgumentError {
            func: "date_diff".to_string(),
            detail: format!("unknown unit '{other}' — use \"day\", \"month\" or \"year\""),
        }),
    };

    Ok(Value::Number(Decimal::from(result)))
}

// ---------------------------------------------------------------------------
// date_add / date_sub
// ---------------------------------------------------------------------------

fn date_add(args: &[Value], line: usize) -> Result<Value, CaperError> {
    shift_date(args, line, "date_add", 1)
}

fn date_sub(args: &[Value], line: usize) -> Result<Value, CaperError> {
    shift_date(args, line, "date_sub", -1)
}

fn shift_date(args: &[Value], line: usize, func: &str, sign: i64) -> Result<Value, CaperError> {
    let d = require_date(args, 0, func, line)?;
    let n = require_int(args, 1, func)?;
    let unit = args.get(2)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .unwrap_or("day");

    let result = match unit {
        "day" => {
            d.checked_add_signed(Duration::days(sign * n))
                .ok_or_else(|| overflow_err(func))?
        }
        "month" => {
            let total_months = d.month() as i64 + sign * n;
            let year_offset = total_months.div_euclid(12);
            let new_month = ((total_months - 1).rem_euclid(12) + 1) as u32;
            let new_year = d.year() as i64 + year_offset;
            // Clamp day to last day of target month
            let last_day = days_in_month(new_year as i32, new_month);
            let new_day = d.day().min(last_day);
            NaiveDate::from_ymd_opt(new_year as i32, new_month, new_day)
                .ok_or_else(|| overflow_err(func))?
        }
        "year" => {
            let new_year = d.year() as i64 + sign * n;
            let last_day = days_in_month(new_year as i32, d.month());
            let new_day = d.day().min(last_day);
            NaiveDate::from_ymd_opt(new_year as i32, d.month(), new_day)
                .ok_or_else(|| overflow_err(func))?
        }
        other => return Err(CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("unknown unit '{other}' — use \"day\", \"month\" or \"year\""),
        }),
    };

    Ok(Value::Str(result.format("%Y-%m-%d").to_string()))
}

// ---------------------------------------------------------------------------
// date_format
// ---------------------------------------------------------------------------

fn date_format(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let d = require_date(args, 0, "date_format", line)?;
    let fmt = require_str(args, 1, "date_format")?;
    Ok(Value::Str(apply_date_format(d, fmt)))
}

fn apply_date_format(d: NaiveDate, fmt: &str) -> String {
    fmt.replace("YYYY", &format!("{:04}", d.year()))
       .replace("MM",   &format!("{:02}", d.month()))
       .replace("DD",   &format!("{:02}", d.day()))
}

// ---------------------------------------------------------------------------
// date_parse
// ---------------------------------------------------------------------------

fn date_parse_fn(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "date_parse")?;
    let fmt = require_str(args, 1, "date_parse")?;

    let d = parse_with_format(s, fmt).ok_or_else(|| CaperError::ArgumentError {
        func: "date_parse".to_string(),
        detail: format!("cannot parse '{s}' with format '{fmt}'"),
    })?;
    Ok(Value::Str(d.format("%Y-%m-%d").to_string()))
}

/// Parse a date string using a format with tokens YYYY, MM, DD.
/// Non-token characters in fmt must match literally in the input.
fn parse_with_format(input: &str, fmt: &str) -> Option<NaiveDate> {
    let tokens: &[(&str, usize)] = &[("YYYY", 4), ("MM", 2), ("DD", 2)];
    let mut year = None::<i32>;
    let mut month = None::<u32>;
    let mut day = None::<u32>;

    let mut fi = 0usize;
    let mut ii = 0usize;
    let fb = fmt.as_bytes();
    let ib = input.as_bytes();

    while fi < fb.len() {
        let mut matched = false;
        for (tok, len) in tokens {
            if fb.get(fi..fi + tok.len()) == Some(tok.as_bytes()) {
                let slice = std::str::from_utf8(ib.get(ii..ii + len)?).ok()?;
                let val: i32 = slice.parse().ok()?;
                match *tok {
                    "YYYY" => year = Some(val),
                    "MM"   => month = Some(val as u32),
                    "DD"   => day = Some(val as u32),
                    _ => {}
                }
                fi += tok.len();
                ii += len;
                matched = true;
                break;
            }
        }
        if !matched {
            // Literal — both cursors advance one byte
            if ib.get(ii) != fb.get(fi) { return None; }
            fi += 1;
            ii += 1;
        }
    }

    NaiveDate::from_ymd_opt(year?, month?, day?)
}

// ---------------------------------------------------------------------------
// is_valid_date
// ---------------------------------------------------------------------------

fn is_valid_date(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let ok = args.get(0)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .is_some();
    Ok(Value::Bool(ok))
}

// ---------------------------------------------------------------------------
// working_days(year, month) → días hábiles del mes (lunes-vier, sin festivos CL)
// ---------------------------------------------------------------------------

fn working_days(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let year = require_int(args, 0, "working_days")? as i32;
    let month = require_int(args, 1, "working_days")? as u32;

    if !(1..=12).contains(&month) {
        return Err(CaperError::ArgumentError {
            func: "working_days".to_string(),
            detail: format!("month must be 1-12, got {month}"),
        });
    }

    let holidays = cl_holidays(year);
    let last = days_in_month(year, month);
    let mut count = 0u32;

    for day_n in 1..=last {
        let d = NaiveDate::from_ymd_opt(year, month, day_n).unwrap();
        let wd = d.weekday().number_from_monday(); // 1=Mon … 7=Sun
        if wd <= 5 && !holidays.contains(&d) {
            count += 1;
        }
    }

    Ok(Value::Number(Decimal::from(count)))
}

/// Returns the set of Chilean public holidays for the given year.
fn cl_holidays(year: i32) -> Vec<NaiveDate> {
    let mut h = vec![
        // Fixed
        date(year, 1,  1),  // Año Nuevo
        date(year, 5,  1),  // Día del Trabajo
        date(year, 5,  21), // Glorias Navales
        date(year, 6,  29), // San Pedro y San Pablo
        date(year, 7,  16), // Virgen del Carmen
        date(year, 8,  15), // Asunción de la Virgen
        date(year, 9,  18), // Fiestas Patrias
        date(year, 9,  19), // Glorias del Ejército
        date(year, 10, 12), // Encuentro de Dos Mundos
        date(year, 10, 31), // Día de las Iglesias Evangélicas
        date(year, 11, 1),  // Todos los Santos
        date(year, 12, 8),  // Inmaculada Concepción
        date(year, 12, 25), // Navidad
    ];

    // Variable — Semana Santa
    let easter = easter_date(year);
    h.push(Some(easter - Duration::days(2))); // Viernes Santo
    h.push(Some(easter - Duration::days(1))); // Sábado Santo

    h.into_iter().flatten().collect()
}

fn date(year: i32, month: u32, day: u32) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(year, month, day)
}

/// Easter date via Meeus/Jones/Butcher algorithm.
fn easter_date(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = ((h + l - 7 * m + 114) / 31) as u32;
    let day   = ((h + l - 7 * m + 114) % 31 + 1) as u32;
    NaiveDate::from_ymd_opt(year, month, day).unwrap()
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

fn require_date(args: &[Value], idx: usize, func: &str, _line: usize) -> Result<NaiveDate, CaperError> {
    let s = args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a date string \"YYYY-MM-DD\"", idx + 1),
        })?;

    NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| CaperError::ArgumentError {
        func: func.to_string(),
        detail: format!("cannot parse date '{s}' — expected \"YYYY-MM-DD\""),
    })
}

fn require_str<'a>(args: &'a [Value], idx: usize, func: &str) -> Result<&'a str, CaperError> {
    args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a string", idx + 1),
        })
}

fn require_int(args: &[Value], idx: usize, func: &str) -> Result<i64, CaperError> {
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

fn days_in_month(year: i32, month: u32) -> u32 {
    let next_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    (next_month.unwrap() - NaiveDate::from_ymd_opt(year, month, 1).unwrap()).num_days() as u32
}

fn overflow_err(func: &str) -> CaperError {
    CaperError::ArgumentError {
        func: func.to_string(),
        detail: "date arithmetic overflow".to_string(),
    }
}
