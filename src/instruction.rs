// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Conjunto de instrucciones de bytecode para AlcaparraLang.
//!
//! La VM es basada en stack. Cada `Chunk` es una lista plana de instrucciones
//! que representa el cuerpo de una función (o el script de nivel raíz).

use crate::parser::ast::{BinOp, UseItems};
use crate::vm::value::Value;

// ---------------------------------------------------------------------------
// Chunk — cuerpo compilado de una función
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct Chunk {
    pub name: Option<String>,
    pub params: Vec<String>,
    pub code: Vec<Instruction>,
}

impl Chunk {
    pub fn new(name: Option<String>, params: Vec<String>) -> Self {
        Self { name, params, code: vec![] }
    }

    /// Agrega una instrucción y retorna su dirección.
    pub fn emit(&mut self, instr: Instruction) -> usize {
        let addr = self.code.len();
        self.code.push(instr);
        addr
    }

    /// Reemplaza la instrucción en `addr` (usado para backpatching de saltos).
    pub fn patch(&mut self, addr: usize, instr: Instruction) {
        self.code[addr] = instr;
    }

    /// Dirección final actual (= dirección de la próxima instrucción).
    pub fn here(&self) -> usize {
        self.code.len()
    }
}

// ---------------------------------------------------------------------------
// CompiledProgram
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct CompiledProgram {
    /// chunks[0] es el cuerpo del script de nivel raíz.
    /// chunks[1..] son los cuerpos de funciones/closures.
    pub chunks: Vec<Chunk>,
}

// ---------------------------------------------------------------------------
// Conjunto de instrucciones
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub enum Instruction {
    // --- Constantes ---
    /// Empuja un valor literal.
    Const(Value),

    // --- Variables ---
    /// Empuja el valor de una variable (Null si no está definida).
    Load(String),
    /// Pop → define un binding inmutable (`let`).
    LetBind(String),
    /// Pop → define un binding mutable (`var`).
    VarBind(String),
    /// Pop → define un binding const (header `const`, máxima precedencia).
    ConstBind(String),
    /// Pop → asigna a un binding existente. Verifica mutabilidad.
    Store(String, usize),

    // --- Acceso a campo / índice ---
    /// Pop objeto, empuja objeto.campo.
    GetField(String, usize),
    /// TOS = valor, debajo = objeto; establece objeto.campo = valor; empuja el nuevo objeto.
    SetField(String, usize),
    /// TOS = índice, debajo = objeto; empuja objeto[índice].
    GetIndex(usize),
    /// TOS = valor, siguiente = índice, siguiente = nombre_obj (string para actualizar scope).
    SetIndex(usize),

    // --- Operadores ---
    /// Pop dos valores, aplica el operador binario, empuja el resultado.
    BinOp(BinOp, usize),
    /// Pop un valor, negación numérica, empuja el resultado.
    Neg,
    /// Pop un valor, NOT lógico, empuja el resultado.
    Not,

    // --- Colecciones ---
    /// Pop n valores (empujados en orden, TOS es el último), construye array, lo empuja.
    MakeArray(usize),
    /// TOS = array a hacer spread; se hace pop y se extiende en el array en construcción.
    /// Se usa junto con MakeArray cuando la cantidad de elementos varía.
    ArraySpread,

    /// Empuja un builder de objeto vacío en el stack de builders de objeto.
    ObjStart,
    /// Pop del valor TOS, inserta bajo `key` en el builder de objeto actual.
    ObjSet(String),
    /// Pop del objeto TOS, fusiona todos sus campos en el builder de objeto actual.
    ObjSpread,
    /// Pop del builder de objeto actual, empuja el Value::Object terminado.
    ObjEnd,

    // --- Control de flujo ---
    Jump(usize),
    /// Si TOS es falso, saltar; TOS NO se hace pop.
    JumpFalse(usize),
    /// Si TOS es verdadero, saltar; TOS NO se hace pop.
    JumpTrue(usize),
    /// Si TOS no es Null, saltar; TOS NO se hace pop. (para `??`)
    JumpNotNull(usize),

    // --- Stack de valores ---
    Pop,
    Dup,

    // --- Scope ---
    Enter,
    Leave,

    // --- Bucles (foreach) ---
    /// Pop del iterable (array u objeto), empuja un IterState en el stack de iteradores.
    IterInit,
    /// Avanza el IterState actual. Si se agotó: salta a `done`.
    /// Sino vincula `val` (y opcionalmente `key`) en el scope actual.
    IterNext { done: usize, key: Option<String>, val: String },
    /// Pop del IterState actual del stack de iteradores (fin del foreach).
    IterDone,

    // --- Funciones ---
    /// Crea un `Value::Function` desde el chunk `chunk_id`, capturando el scope actual.
    Closure { chunk_id: usize, name: Option<String> },
    /// Pop de `n_args` argumentos (TOS = último arg), luego pop del callee `Value::Function`.
    Call(usize, usize),
    /// Despacho por nombre: scope → HOF → stdlib → error.
    /// Stack: [..., arg0, ..., argN-1]. NO hace pop del callee desde el stack.
    CallNamed(String, usize, usize),
    /// Despacho de llamada de campo: stack es [..., objeto, arg0, ..., argN-1].
    /// Si objeto es un Value::Object con el campo, llama esa función.
    /// Sino despacha `field` como nombre (búsqueda en stdlib/scope).
    CallMethod(String, usize, usize),
    /// Pop del valor de retorno, retorna al frame llamador (empuja el valor allí).
    Ret,

    // --- Salida del script ---
    /// Pop del valor, lo registra como resultado del script, termina la ejecución.
    /// Lleva la línea fuente para diagnósticos del linter (0 = centinela/desconocido).
    Emit(usize),

    // --- Excepciones ---
    /// Empuja un manejador de excepción apuntando a `catch_addr`.
    TryPush(usize),
    /// Pop del manejador de excepción actual (fin del bloque try).
    TryPop,
    /// Pop TOS como valor a lanzar, lo eleva como error.
    Throw(usize),

    // --- Rango ---
    /// Pop del fin, pop del inicio, empuja Array(inicio..=fin).
    Range(usize),

    // --- Importaciones ---
    UseModule { path: Vec<String>, items: UseItems },
}
