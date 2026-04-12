// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use regex::Regex;

pub fn call(name: &str, args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "regex_match"       => regex_match(args, line)?,
        "regex_test"        => regex_test(args, line)?,
        "regex_find"        => regex_find(args, line)?,
        "regex_find_all"    => regex_find_all(args, line)?,
        "regex_groups"      => regex_groups(args, line)?,
        "regex_replace"     => regex_replace(args, line)?,
        "regex_replace_all" => regex_replace_all(args, line)?,
        "regex_split"       => regex_split(args, line)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// regex_match — coincidencia anclada en el string completo
// ---------------------------------------------------------------------------

fn regex_match(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re) = require_str_pattern(args, "regex_match", line)?;
    let anchored = format!("^(?:{})$", re.as_str());
    let full = Regex::new(&anchored).map_err(|e| invalid_re(&anchored, &e.to_string()))?;
    Ok(Value::Bool(full.is_match(s)))
}

// ---------------------------------------------------------------------------
// regex_test — coincidencia en cualquier parte del string
// ---------------------------------------------------------------------------

fn regex_test(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re) = require_str_pattern(args, "regex_test", line)?;
    Ok(Value::Bool(re.is_match(s)))
}

// ---------------------------------------------------------------------------
// regex_find — primera coincidencia o null
// ---------------------------------------------------------------------------

fn regex_find(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re) = require_str_pattern(args, "regex_find", line)?;
    match re.find(s) {
        Some(m) => Ok(Value::Str(m.as_str().to_string())),
        None    => Ok(Value::Null),
    }
}

// ---------------------------------------------------------------------------
// regex_find_all — todas las coincidencias no superpuestas
// ---------------------------------------------------------------------------

fn regex_find_all(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re) = require_str_pattern(args, "regex_find_all", line)?;
    let matches: Vec<Value> = re.find_iter(s)
        .map(|m| Value::Str(m.as_str().to_string()))
        .collect();
    Ok(Value::Array(matches))
}

// ---------------------------------------------------------------------------
// regex_groups — grupos de captura de la primera coincidencia
// ---------------------------------------------------------------------------

fn regex_groups(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re) = require_str_pattern(args, "regex_groups", line)?;
    match re.captures(s) {
        None => Ok(Value::Array(vec![])),
        Some(caps) => {
            // caps[0] is the full match; skip it, return groups 1..
            let groups: Vec<Value> = caps.iter().skip(1)
                .map(|m| match m {
                    Some(m) => Value::Str(m.as_str().to_string()),
                    None    => Value::Null,
                })
                .collect();
            Ok(Value::Array(groups))
        }
    }
}

// ---------------------------------------------------------------------------
// regex_replace / regex_replace_all
// ---------------------------------------------------------------------------

fn regex_replace(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re, repl) = require_str_pattern_repl(args, "regex_replace", line)?;
    Ok(Value::Str(re.replacen(s, 1, repl).into_owned()))
}

fn regex_replace_all(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re, repl) = require_str_pattern_repl(args, "regex_replace_all", line)?;
    Ok(Value::Str(re.replace_all(s, repl).into_owned()))
}

// ---------------------------------------------------------------------------
// regex_split
// ---------------------------------------------------------------------------

fn regex_split(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let (s, re) = require_str_pattern(args, "regex_split", line)?;
    let parts: Vec<Value> = re.split(s)
        .map(|p| Value::Str(p.to_string()))
        .collect();
    Ok(Value::Array(parts))
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

fn require_str_pattern<'a>(
    args: &'a [Value],
    func: &str,
    _line: usize,
) -> Result<(&'a str, Regex), CaperError> {
    let s = require_str(args, 0, func)?;
    let pattern = require_str(args, 1, func)?;
    let re = Regex::new(pattern).map_err(|e| invalid_re(pattern, &e.to_string()))?;
    Ok((s, re))
}

fn require_str_pattern_repl<'a>(
    args: &'a [Value],
    func: &str,
    _line: usize,
) -> Result<(&'a str, Regex, &'a str), CaperError> {
    let s = require_str(args, 0, func)?;
    let pattern = require_str(args, 1, func)?;
    let repl = require_str(args, 2, func)?;
    let re = Regex::new(pattern).map_err(|e| invalid_re(pattern, &e.to_string()))?;
    Ok((s, re, repl))
}

fn require_str<'a>(args: &'a [Value], idx: usize, func: &str) -> Result<&'a str, CaperError> {
    args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a string", idx + 1),
        })
}

fn invalid_re(pattern: &str, detail: &str) -> CaperError {
    CaperError::InvalidRegex {
        pattern: pattern.to_string(),
        detail: detail.to_string(),
    }
}
