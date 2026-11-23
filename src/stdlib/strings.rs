// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;

pub fn call(name: &str, args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "len" => {
            // len(s: string) — polimórfico, delega a arrays para no-strings
            match args.first() {
                Some(Value::Str(_)) => len_str(args, line)?,
                _ => return Ok(None),
            }
        }
        "upper"        => upper(args, line)?,
        "lower"        => lower(args, line)?,
        "trim"         => trim(args, line)?,
        "ltrim"        => ltrim(args, line)?,
        "rtrim"        => rtrim(args, line)?,
        "contains" => {
            // contains(s, sub) — solo para strings; los arrays se manejan por separado
            match args.first() {
                Some(Value::Str(_)) => str_contains(args, line)?,
                _ => return Ok(None),
            }
        }
        "starts_with"  => starts_with(args, line)?,
        "ends_with"    => ends_with(args, line)?,
        "replace"      => replace(args, line)?,
        "replace_all"  => replace_all(args, line)?,
        "split"        => split(args, line)?,
        "join"         => join(args, line)?,
        "substr"       => substr(args, line)?,
        "pad_left"     => pad_left(args, line)?,
        "pad_right"    => pad_right(args, line)?,
        "str"          => to_str(args, line)?,
        // Nombres que anuncia el catalogo de `caper -m`.
        "trim_start"   => ltrim(args, line)?,
        "trim_end"     => rtrim(args, line)?,
        "to_string"    => to_str(args, line)?,
        "to_number"    => to_number(args, line)?,
        "repeat"       => repeat(args, line)?,
        "char_at"      => char_at(args, line)?,
        "substring"    => substring(args, line)?,
        "count_occurrences" => count_occurrences(args, line)?,
        "index_of" => {
            // Tambien lo reclama `std.arrays`: aca solo los strings.
            match args.first() {
                Some(Value::Str(_)) => index_of(args, line)?,
                _ => return Ok(None),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// --- Helpers internos ---

fn require_str<'a>(args: &'a [Value], idx: usize, fn_name: &str) -> Result<&'a str, CaperError> {
    match args.get(idx) {
        Some(Value::Str(s)) => Ok(s.as_str()),
        Some(other) => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} must be a string, got {}", idx + 1, other.type_name()),
        }),
        None => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} is required", idx + 1),
        }),
    }
}

fn require_usize(args: &[Value], idx: usize, fn_name: &str) -> Result<usize, CaperError> {
    match args.get(idx) {
        Some(v) => v.as_decimal()
            .and_then(|d| d.to_usize())
            .ok_or_else(|| CaperError::ArgumentError {
                func: fn_name.to_string(),
                detail: format!("argument {} must be a non-negative integer", idx + 1),
            }),
        None => Err(CaperError::ArgumentError {
            func: fn_name.to_string(),
            detail: format!("argument {} is required", idx + 1),
        }),
    }
}

// --- Funciones ---

fn len_str(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "len")?;
    Ok(Value::Number(Decimal::from(s.chars().count())))
}

fn upper(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "upper")?;
    Ok(Value::Str(s.to_uppercase()))
}

fn lower(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "lower")?;
    Ok(Value::Str(s.to_lowercase()))
}

fn trim(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "trim")?;
    Ok(Value::Str(s.trim().to_string()))
}

fn ltrim(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "ltrim")?;
    Ok(Value::Str(s.trim_start().to_string()))
}

fn rtrim(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "rtrim")?;
    Ok(Value::Str(s.trim_end().to_string()))
}

fn str_contains(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s   = require_str(args, 0, "contains")?;
    let sub = require_str(args, 1, "contains")?;
    Ok(Value::Bool(s.contains(sub)))
}

fn starts_with(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s      = require_str(args, 0, "starts_with")?;
    let prefix = require_str(args, 1, "starts_with")?;
    Ok(Value::Bool(s.starts_with(prefix)))
}

fn ends_with(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s      = require_str(args, 0, "ends_with")?;
    let suffix = require_str(args, 1, "ends_with")?;
    Ok(Value::Bool(s.ends_with(suffix)))
}

fn replace(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s    = require_str(args, 0, "replace")?;
    let from = require_str(args, 1, "replace")?;
    let to   = require_str(args, 2, "replace")?;
    Ok(Value::Str(s.replacen(from, to, 1)))
}

fn replace_all(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s    = require_str(args, 0, "replace_all")?;
    let from = require_str(args, 1, "replace_all")?;
    let to   = require_str(args, 2, "replace_all")?;
    Ok(Value::Str(s.replace(from, to)))
}

fn split(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s   = require_str(args, 0, "split")?;
    let sep = require_str(args, 1, "split")?;
    let parts = s.split(sep)
        .map(|p| Value::Str(p.to_string()))
        .collect();
    Ok(Value::Array(parts))
}

fn join(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    match args.get(0) {
        Some(Value::Array(arr)) => {
            let sep = match args.get(1) {
                Some(Value::Str(s)) => s.as_str(),
                Some(other) => return Err(CaperError::ArgumentError {
                    func: "join".to_string(),
                    detail: format!("separator must be a string, got {}", other.type_name()),
                }),
                None => "",
            };
            let parts: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
            Ok(Value::Str(parts.join(sep)))
        }
        Some(other) => Err(CaperError::ArgumentError {
            func: "join".to_string(),
            detail: format!("argument 1 must be an array, got {}", other.type_name()),
        }),
        None => Err(CaperError::ArgumentError {
            func: "join".to_string(),
            detail: "argument 1 is required".to_string(),
        }),
    }
}

fn substr(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s     = require_str(args, 0, "substr")?;
    let start = require_usize(args, 1, "substr")?;
    let chars: Vec<char> = s.chars().collect();
    let result = if args.len() > 2 {
        let length = require_usize(args, 2, "substr")?;
        chars.iter().skip(start).take(length).collect()
    } else {
        chars.iter().skip(start).collect()
    };
    Ok(Value::Str(result))
}

fn pad_left(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s    = require_str(args, 0, "pad_left")?;
    let n    = require_usize(args, 1, "pad_left")?;
    let pad  = if args.len() > 2 { require_str(args, 2, "pad_left")? } else { " " };
    let pad_char = pad.chars().next().unwrap_or(' ');
    let chars: Vec<char> = s.chars().collect();
    let current = chars.len();
    if current >= n {
        Ok(Value::Str(s.to_string()))
    } else {
        let padding: String = std::iter::repeat(pad_char).take(n - current).collect();
        Ok(Value::Str(format!("{padding}{s}")))
    }
}

fn pad_right(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s    = require_str(args, 0, "pad_right")?;
    let n    = require_usize(args, 1, "pad_right")?;
    let pad  = if args.len() > 2 { require_str(args, 2, "pad_right")? } else { " " };
    let pad_char = pad.chars().next().unwrap_or(' ');
    let chars: Vec<char> = s.chars().collect();
    let current = chars.len();
    if current >= n {
        Ok(Value::Str(s.to_string()))
    } else {
        let padding: String = std::iter::repeat(pad_char).take(n - current).collect();
        Ok(Value::Str(format!("{s}{padding}")))
    }
}

fn to_str(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    match args.first() {
        Some(v) => Ok(Value::Str(v.to_string())),
        None => Err(CaperError::ArgumentError {
            func: "str".to_string(),
            detail: "argument 1 is required".to_string(),
        }),
    }
}

/// Convierte el texto a numero. Es la misma conversion de `std.types.number`,
/// con el nombre que anuncia el catalogo de `std.strings`.
fn to_number(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    match args.first().and_then(|v| v.as_decimal()) {
        Some(n) => Ok(Value::Number(n)),
        None => Err(CaperError::ArgumentError {
            func: "to_number".to_string(),
            detail: "el argumento no se puede convertir a numero".to_string(),
        }),
    }
}

/// Repite el texto `n` veces. `n = 0` da el texto vacio.
fn repeat(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "repeat")?;
    let n = require_usize(args, 1, "repeat")?;
    Ok(Value::Str(s.repeat(n)))
}

/// El caracter en la posicion dada, contando caracteres y no bytes. Fuera de
/// rango devuelve el texto vacio y no un error, igual que `first`/`last` en
/// arreglos devuelven `null`.
fn char_at(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "char_at")?;
    let i = require_usize(args, 1, "char_at")?;
    Ok(Value::Str(s.chars().nth(i).map(|c| c.to_string()).unwrap_or_default()))
}

/// Porcion del texto entre dos posiciones: `inicio` incluido, `fin` excluido.
///
/// **No es un alias de `substr`**, que recibe un **largo** como tercer argumento.
/// Los dos nombres existen con la semantica que cada uno tiene por convencion:
/// `substr("abcde", 1, 3)` es `"bcd"` (tres caracteres) y
/// `substring("abcde", 1, 3)` es `"bc"` (hasta la posicion 3).
fn substring(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s      = require_str(args, 0, "substring")?;
    let inicio = require_usize(args, 1, "substring")?;
    let chars: Vec<char> = s.chars().collect();

    let fin = if args.len() > 2 {
        require_usize(args, 2, "substring")?.min(chars.len())
    } else {
        chars.len()
    };

    if inicio >= fin {
        return Ok(Value::Str(String::new()));
    }
    Ok(Value::Str(chars[inicio..fin].iter().collect()))
}

/// Posicion de la primera aparicion, en caracteres, o `-1` si no esta.
fn index_of(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s   = require_str(args, 0, "index_of")?;
    let sub = require_str(args, 1, "index_of")?;

    match s.find(sub) {
        // `find` da un offset en bytes; se traduce a caracteres.
        Some(byte) => Ok(Value::Number(Decimal::from(s[..byte].chars().count()))),
        None => Ok(Value::Number(Decimal::from(-1))),
    }
}

/// Cuantas veces aparece `sub` en el texto, sin superponerse. `sub` vacio da 0.
fn count_occurrences(args: &[Value], _line: usize) -> Result<Value, CaperError> {
    let s   = require_str(args, 0, "count_occurrences")?;
    let sub = require_str(args, 1, "count_occurrences")?;

    if sub.is_empty() {
        return Ok(Value::Number(Decimal::ZERO));
    }
    Ok(Value::Number(Decimal::from(s.matches(sub).count())))
}
