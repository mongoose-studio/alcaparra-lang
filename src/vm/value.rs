// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::error::CaperError;
use crate::instruction::CompiledProgram;
use crate::parser::ast::Stmt;
use indexmap::IndexMap;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

/// Función o closure definida por el usuario.
///
/// Las declaraciones `fn` con nombre capturan el scope en el momento de la declaración.
/// Los closures (`|x| => expr`) hacen lo mismo.
/// El scope capturado es un mapa plano (fusionado de todos los frames visibles en
/// el momento de creación) para que el cuerpo de la función pueda resolver variables libres.
#[derive(Debug, Clone)]
pub struct FunctionDef {
    pub name: Option<String>,
    pub params: Vec<String>,
    /// Cuerpo del intérprete tree-walker (usado por el intérprete antiguo).
    pub body: Vec<Stmt>,
    /// Índice del chunk de bytecode (usado por el ejecutor de bytecode, Fase 5+).
    pub chunk_id: Option<usize>,
    /// Programa compilado al que pertenece este chunk (necesario para llamadas cross-biblioteca).
    pub program: Option<Rc<CompiledProgram>>,
    /// Snapshot del scope envolvente en el momento de declaración del closure/fn.
    /// Entorno capturado, compartido: ver `ScopeStack::base`.
    pub captured: Rc<HashMap<String, Value>>,
}

/// Las funciones nunca son iguales entre sí.
impl PartialEq for FunctionDef {
    fn eq(&self, _: &Self) -> bool { false }
}

/// Función nativa inyectada por el host, invocable desde scripts.
///
/// El closure recibe los argumentos de la llamada y un número de línea para mensajes de error.
/// Envuelve tu lógica Rust aquí para exponerla a los scripts de AlcaparraLang.
pub struct NativeFn {
    pub name: String,
    pub func: Rc<dyn Fn(Vec<Value>, usize) -> Result<Value, CaperError>>,
}

impl fmt::Debug for NativeFn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native fn {}>", self.name)
    }
}

impl Clone for NativeFn {
    fn clone(&self) -> Self {
        NativeFn { name: self.name.clone(), func: Rc::clone(&self.func) }
    }
}

impl PartialEq for NativeFn {
    fn eq(&self, _: &Self) -> bool { false }
}

/// El tipo de valor en tiempo de ejecución.
///
/// Usa `rust_decimal::Decimal` para todos los números — nunca f64.
/// Los objetos usan `IndexMap` para preservar el orden de inserción en la salida de `emit`.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(Decimal),
    Str(String),
    Bool(bool),
    Null,
    Array(Vec<Value>),
    Object(IndexMap<String, Value>),
    Function(Rc<FunctionDef>),
    /// Función Rust nativa inyectada por el host (desde plugins o `register_fn`).
    NativeFunction(NativeFn),
}

impl Value {
    pub fn number(d: Decimal) -> Self { Value::Number(d) }
    pub fn str(s: impl Into<String>) -> Self { Value::Str(s.into()) }
    pub fn bool(b: bool) -> Self { Value::Bool(b) }
    pub fn null() -> Self { Value::Null }
    pub fn array(v: Vec<Value>) -> Self { Value::Array(v) }
    pub fn object(m: IndexMap<String, Value>) -> Self { Value::Object(m) }

    /// Veracidad — consistente con las convenciones de lenguajes de scripting comunes.
    ///
    /// Falso: `null`, `false`, `0`, `""`, `[]`, `{}`
    /// Verdadero: todo lo demás
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Null              => false,
            Value::Bool(b)           => *b,
            Value::Number(n)         => !n.is_zero(),
            Value::Str(s)            => !s.is_empty(),
            Value::Array(a)          => !a.is_empty(),
            Value::Object(o)         => !o.is_empty(),
            Value::Function(_)       => true,
            Value::NativeFunction(_) => true,
        }
    }

    pub fn is_null(&self) -> bool { matches!(self, Value::Null) }
    pub fn is_number(&self) -> bool { matches!(self, Value::Number(_)) }
    pub fn is_string(&self) -> bool { matches!(self, Value::Str(_)) }
    pub fn is_bool(&self) -> bool { matches!(self, Value::Bool(_)) }
    pub fn is_array(&self) -> bool { matches!(self, Value::Array(_)) }
    pub fn is_object(&self) -> bool { matches!(self, Value::Object(_)) }
    pub fn is_function(&self) -> bool {
        matches!(self, Value::Function(_) | Value::NativeFunction(_))
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Number(_)         => "number",
            Value::Str(_)            => "string",
            Value::Bool(_)           => "bool",
            Value::Null              => "null",
            Value::Array(_)          => "array",
            Value::Object(_)         => "object",
            Value::Function(_)       => "function",
            Value::NativeFunction(_) => "function",
        }
    }

    /// Intenta convertir a Decimal para aritmética.
    /// Conversion **laxa**: parsea strings y trata los booleanos como 1 y 0.
    /// Es la que necesitan `to_number` y las funciones que convierten a
    /// proposito. **No** la use la aritmetica: ver `as_number`.
    pub fn as_decimal(&self) -> Option<Decimal> {
        match self {
            Value::Number(n) => Some(*n),
            Value::Str(s)    => s.parse().ok(),
            Value::Bool(b)   => Some(if *b { Decimal::ONE } else { Decimal::ZERO }),
            _                => None,
        }
    }

    /// Conversion **estricta**: solo un numero es un numero.
    ///
    /// La aritmetica usa esta y no `as_decimal` porque un contexto JSON trae
    /// numeros como string con muchisima facilidad —PostgreSQL serializa
    /// `numeric` como string, y PHP arrastra eso hasta el JSON—. Con la
    /// conversion laxa, `"1850000" * 26 / 30` daba un resultado correcto y la
    /// primera suma lo concatenaba: la liquidacion salia mal sin un solo error.
    /// Mejor que reviente en la primera operacion.
    pub fn as_number(&self) -> Option<Decimal> {
        match self {
            Value::Number(n) => Some(*n),
            _                => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n)            => write!(f, "{n}"),
            Value::Str(s)               => write!(f, "{s}"),
            Value::Bool(b)              => write!(f, "{b}"),
            Value::Null                 => write!(f, "null"),
            Value::NativeFunction(nf)   => write!(f, "<native fn {}>", nf.name),
            Value::Function(fd)  => {
                let name = fd.name.as_deref().unwrap_or("<closure>");
                write!(f, "<fn {name}>")
            }
            Value::Array(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{v}")?;
                }
                write!(f, "]")
            }
            Value::Object(o) => {
                write!(f, "{{")?;
                for (i, (k, v)) in o.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, "}}")
            }
        }
    }
}

// Conversión a serde_json para respuestas HTTP
impl From<Value> for serde_json::Value {
    fn from(v: Value) -> Self {
        match v {
            Value::NativeFunction(nf) => {
                serde_json::Value::String(format!("<native fn {}>", nf.name))
            }
            Value::Function(fd) => {
                let name = fd.name.as_deref().unwrap_or("<closure>").to_string();
                serde_json::Value::String(format!("<fn {name}>"))
            }
            Value::Number(n) => {
                // Serializar como número — mantener forma entera si no hay parte fraccionaria
                if n.fract().is_zero() {
                    if let Some(i) = n.to_i64_checked() {
                        return serde_json::Value::Number(i.into());
                    }
                }
                serde_json::Value::Number(
                    serde_json::Number::from_f64(
                        n.to_string().parse::<f64>().unwrap_or(0.0)
                    ).unwrap_or(serde_json::Number::from(0))
                )
            }
            Value::Str(s)   => serde_json::Value::String(s),
            Value::Bool(b)  => serde_json::Value::Bool(b),
            Value::Null     => serde_json::Value::Null,
            Value::Array(a) => serde_json::Value::Array(a.into_iter().map(Into::into).collect()),
            Value::Object(o) => {
                let map = o.into_iter()
                    .map(|(k, v)| (k, serde_json::Value::from(v)))
                    .collect();
                serde_json::Value::Object(map)
            }
        }
    }
}

// Helper para conversión a i64 usado en el impl Display arriba
trait DecimalToI64 { fn to_i64_checked(&self) -> Option<i64>; }
impl DecimalToI64 for Decimal {
    fn to_i64_checked(&self) -> Option<i64> {
        use rust_decimal::prelude::ToPrimitive;
        (*self).to_i64()
    }
}

/// Convierte un `serde_json::Value` en un `Value`.
/// Usado por plugins dinámicos para deserializar resultados JSON del ABI C.
pub fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null       => Value::Null,
        serde_json::Value::Bool(b)    => Value::Bool(b),
        serde_json::Value::String(s)  => Value::Str(s),
        serde_json::Value::Number(n)  => {
            let d: Decimal = n.to_string().parse().unwrap_or_default();
            Value::Number(d)
        }
        serde_json::Value::Array(arr) => {
            Value::Array(arr.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => {
            let m = map.into_iter().map(|(k, v)| (k, json_to_value(v))).collect();
            Value::Object(m)
        }
    }
}