// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use base64::{engine::general_purpose, Engine};
use hmac::{Hmac, Mac};
use md5::Md5;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};

type HmacSha256 = Hmac<Sha256>;

pub fn call(name: &str, args: &[Value], _line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "md5"             => hash_md5(args)?,
        "sha1"            => hash_sha1(args)?,
        "sha256"          => hash_sha256(args)?,
        "sha512"          => hash_sha512(args)?,
        "hmac_sha256"     => hmac_sha256(args)?,
        "base64_encode"   => base64_encode(args)?,
        "base64_decode"   => base64_decode(args)?,
        "base64_url_encode" => base64_url_encode(args)?,
        "hex_encode"      => hex_encode_fn(args)?,
        "hex_decode"      => hex_decode_fn(args)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------

fn hash_md5(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "md5")?;
    let mut h = Md5::new();
    h.update(s.as_bytes());
    Ok(Value::Str(hex::encode(h.finalize())))
}

fn hash_sha1(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "sha1")?;
    let mut h = Sha1::new();
    h.update(s.as_bytes());
    Ok(Value::Str(hex::encode(h.finalize())))
}

fn hash_sha256(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "sha256")?;
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    Ok(Value::Str(hex::encode(h.finalize())))
}

fn hash_sha512(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "sha512")?;
    let mut h = Sha512::new();
    h.update(s.as_bytes());
    Ok(Value::Str(hex::encode(h.finalize())))
}

fn hmac_sha256(args: &[Value]) -> Result<Value, CaperError> {
    let data = require_str(args, 0, "hmac_sha256")?;
    let key  = require_str(args, 1, "hmac_sha256")?;
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|e| CaperError::ArgumentError {
            func: "hmac_sha256".to_string(),
            detail: e.to_string(),
        })?;
    mac.update(data.as_bytes());
    Ok(Value::Str(hex::encode(mac.finalize().into_bytes())))
}

fn base64_encode(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "base64_encode")?;
    Ok(Value::Str(general_purpose::STANDARD.encode(s.as_bytes())))
}

fn base64_decode(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "base64_decode")?;
    let bytes = general_purpose::STANDARD.decode(s.as_bytes())
        .map_err(|e| CaperError::ArgumentError {
            func: "base64_decode".to_string(),
            detail: e.to_string(),
        })?;
    let decoded = String::from_utf8(bytes).map_err(|e| CaperError::ArgumentError {
        func: "base64_decode".to_string(),
        detail: e.to_string(),
    })?;
    Ok(Value::Str(decoded))
}

fn base64_url_encode(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "base64_url_encode")?;
    Ok(Value::Str(general_purpose::URL_SAFE_NO_PAD.encode(s.as_bytes())))
}

fn hex_encode_fn(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "hex_encode")?;
    Ok(Value::Str(hex::encode(s.as_bytes())))
}

fn hex_decode_fn(args: &[Value]) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "hex_decode")?;
    let bytes = hex::decode(s).map_err(|e| CaperError::ArgumentError {
        func: "hex_decode".to_string(),
        detail: e.to_string(),
    })?;
    let decoded = String::from_utf8(bytes).map_err(|e| CaperError::ArgumentError {
        func: "hex_decode".to_string(),
        detail: e.to_string(),
    })?;
    Ok(Value::Str(decoded))
}

// ---------------------------------------------------------------------------

fn require_str<'a>(args: &'a [Value], idx: usize, func: &str) -> Result<&'a str, CaperError> {
    args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a string", idx + 1),
        })
}
