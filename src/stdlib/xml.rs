// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::vm::value::Value;
use indexmap::IndexMap;
use quick_xml::Reader;
use quick_xml::events::Event;

pub fn call(name: &str, args: &[Value], line: usize) -> Result<Option<Value>, CaperError> {
    let result = match name {
        "xml_encode"      => xml_encode(args, line)?,
        "xml_encode_opts" => xml_encode_opts(args, line)?,
        "xml_decode"      => xml_decode(args, line)?,
        "xml_valid"       => xml_valid(args)?,
        "xml_get"         => xml_get(args, line)?,
        "xml_get_all"     => xml_get_all(args, line)?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}

// ---------------------------------------------------------------------------
// xml_encode(v, root)
// ---------------------------------------------------------------------------

fn xml_encode(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let val  = args.get(0).cloned().unwrap_or(Value::Null);
    let root = require_str(args, 1, "xml_encode", line)?;
    let xml  = encode_value(&val, root, 0, "  ", false, None);
    Ok(Value::Str(xml))
}

// ---------------------------------------------------------------------------
// xml_encode_opts(v, root, opts)
// opts: { indent: bool, declaration: bool, namespace: string }
// ---------------------------------------------------------------------------

fn xml_encode_opts(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let val   = args.get(0).cloned().unwrap_or(Value::Null);
    let root  = require_str(args, 1, "xml_encode_opts", line)?;
    let opts  = args.get(2);

    let indent     = bool_opt(opts, "indent", true);
    let decl       = bool_opt(opts, "declaration", false);
    let namespace  = str_opt(opts, "namespace");
    let indent_str = if indent { "  " } else { "" };

    let mut buf = String::new();
    if decl {
        buf.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    }
    buf.push_str(&encode_value(&val, root, 0, indent_str, true, namespace.as_deref()));
    Ok(Value::Str(buf))
}

// ---------------------------------------------------------------------------
// Codificador XML recursivo
// ---------------------------------------------------------------------------

fn encode_value(
    val: &Value,
    tag: &str,
    depth: usize,
    indent: &str,
    is_root: bool,
    namespace: Option<&str>,
) -> String {
    let pad = indent.repeat(depth);
    let nl  = if indent.is_empty() { "" } else { "\n" };

    // Construir tag de apertura con xmlns opcional en el root
    let open = if is_root {
        if let Some(ns) = namespace {
            format!("<{} xmlns=\"{}\">", tag, ns)
        } else {
            format!("<{}>", tag)
        }
    } else {
        format!("<{}>", tag)
    };

    match val {
        Value::Object(m) => {
            if m.is_empty() {
                return format!("{}<{} />{}", pad, tag, nl);
            }
            let mut buf = format!("{}{}{}", pad, open, nl);
            for (k, v) in m {
                buf.push_str(&encode_value(v, k, depth + 1, indent, false, None));
            }
            buf.push_str(&format!("{}</{}>{}", pad, tag, nl));
            buf
        }
        Value::Array(a) => {
            // Envolver cada elemento en un hijo <item> bajo este tag
            let mut buf = format!("{}{}{}", pad, open, nl);
            for item in a {
                buf.push_str(&encode_value(item, "item", depth + 1, indent, false, None));
            }
            buf.push_str(&format!("{}</{}>{}", pad, tag, nl));
            buf
        }
        Value::Str(s) => {
            format!("{}{}{}</{}>{}", pad, open, escape_xml(s), tag, nl)
        }
        Value::Number(n) => {
            format!("{}{}{}</{}>{}", pad, open, n, tag, nl)
        }
        Value::Bool(b) => {
            format!("{}{}{}</{}>{}", pad, open, b, tag, nl)
        }
        Value::Null => {
            format!("{}<{} />{}", pad, tag, nl)
        }
        _ => format!("{}<{} />{}", pad, tag, nl),
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&',  "&amp;")
     .replace('<',  "&lt;")
     .replace('>',  "&gt;")
     .replace('"',  "&quot;")
     .replace('\'', "&apos;")
}

// ---------------------------------------------------------------------------
// xml_decode(s) → Value
// ---------------------------------------------------------------------------

fn xml_decode(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let s = require_str(args, 0, "xml_decode", line)?;
    parse_xml(s).map_err(|e| CaperError::XmlParseError { detail: e })
}

// ---------------------------------------------------------------------------
// xml_valid(s) → bool
// ---------------------------------------------------------------------------

fn xml_valid(args: &[Value]) -> Result<Value, CaperError> {
    let s = match args.get(0) {
        Some(Value::Str(s)) => s.as_str(),
        _ => return Ok(Value::Bool(false)),
    };
    Ok(Value::Bool(parse_xml(s).is_ok()))
}

// ---------------------------------------------------------------------------
// xml_get(s, path) / xml_get_all(s, path)
// path: "root/child/leaf"
// ---------------------------------------------------------------------------

fn xml_get(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let decoded = xml_decode(args, line)?;
    let path = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Null),
    };
    Ok(traverse_path(&decoded, &path))
}

fn xml_get_all(args: &[Value], line: usize) -> Result<Value, CaperError> {
    let decoded = xml_decode(args, line)?;
    let path = match args.get(1) {
        Some(Value::Str(s)) => s.clone(),
        _ => return Ok(Value::Array(vec![])),
    };
    let val = traverse_path(&decoded, &path);
    // Si ya es un array, retornarlo; de lo contrario, envolver en array
    match val {
        Value::Array(_)  => Ok(val),
        Value::Null      => Ok(Value::Array(vec![])),
        other            => Ok(Value::Array(vec![other])),
    }
}

fn traverse_path(val: &Value, path: &str) -> Value {
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let mut cur = val.clone();
    for seg in segments {
        cur = match cur {
            Value::Object(ref m) => m.get(seg).cloned().unwrap_or(Value::Null),
            _ => return Value::Null,
        };
    }
    cur
}

// ---------------------------------------------------------------------------
// Parser XML — basado en eventos, construye un árbol de Value
// ---------------------------------------------------------------------------

fn parse_xml(input: &str) -> Result<Value, String> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);

    // Stack of (tag_name, children: IndexMap<String, Vec<Value>>, text: String)
    let mut stack: Vec<(String, IndexMap<String, Vec<Value>>, String)> = vec![];
    let mut root_value: Option<Value> = None;

    loop {
        match reader.read_event() {
            Ok(Event::Start(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                stack.push((tag, IndexMap::new(), String::new()));
            }
            Ok(Event::End(_)) => {
                let (tag, children, text) = stack.pop()
                    .ok_or_else(|| "unexpected closing tag".to_string())?;

                let node_val = if children.is_empty() {
                    // Leaf node — text or null
                    let t = text.trim().to_string();
                    if t.is_empty() { Value::Null } else { Value::Str(t) }
                } else {
                    // Inner node — object, collapsing repeated keys to arrays
                    let mut m = IndexMap::new();
                    for (k, mut vals) in children {
                        if vals.len() == 1 {
                            m.insert(k, vals.remove(0));
                        } else {
                            m.insert(k, Value::Array(vals));
                        }
                    }
                    Value::Object(m)
                };

                if let Some(parent) = stack.last_mut() {
                    parent.1.entry(tag).or_default().push(node_val);
                } else {
                    root_value = Some(node_val);
                }
            }
            Ok(Event::Empty(e)) => {
                let tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                if let Some(parent) = stack.last_mut() {
                    parent.1.entry(tag).or_default().push(Value::Null);
                }
            }
            Ok(Event::Text(e)) => {
                let t = e.unescape().map_err(|e| e.to_string())?;
                if let Some(frame) = stack.last_mut() {
                    frame.2.push_str(&t);
                }
            }
            Ok(Event::Eof) => break,
            Ok(_)          => {} // Decl, Comment, PI etc. — skip
            Err(e)         => return Err(e.to_string()),
        }
    }

    // Wrap in a root object: { root_tag: value }
    root_value.ok_or_else(|| "empty or invalid XML".to_string())
}

// ---------------------------------------------------------------------------
// Helpers internos
// ---------------------------------------------------------------------------

fn require_str<'a>(args: &'a [Value], idx: usize, func: &str, _line: usize) -> Result<&'a str, CaperError> {
    args.get(idx)
        .and_then(|v| if let Value::Str(s) = v { Some(s.as_str()) } else { None })
        .ok_or_else(|| CaperError::ArgumentError {
            func: func.to_string(),
            detail: format!("argument {} must be a string", idx + 1),
        })
}

fn bool_opt(opts: Option<&Value>, key: &str, default: bool) -> bool {
    match opts {
        Some(Value::Object(m)) => match m.get(key) {
            Some(Value::Bool(b)) => *b,
            _ => default,
        },
        _ => default,
    }
}

fn str_opt(opts: Option<&Value>, key: &str) -> Option<String> {
    match opts {
        Some(Value::Object(m)) => match m.get(key) {
            Some(Value::Str(s)) => Some(s.clone()),
            _ => None,
        },
        _ => None,
    }
}
