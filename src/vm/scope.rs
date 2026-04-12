// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::vm::value::Value;
use std::collections::HashMap;

/// Mutabilidad de un binding.
#[derive(Debug, Clone, PartialEq)]
pub enum Binding {
    Const,     // declarado en header — máxima precedencia, no puede ser sobreescrito
    Immutable, // `let`
    Mutable,   // `var`
}

#[derive(Debug, Clone)]
struct Slot {
    value:   Value,
    binding: Binding,
}

/// Stack de scope léxico.
///
/// Nivel 0: contexto inyectado (variables de runtime del llamador)
/// Nivel 1: constantes del header (cargadas antes de que main se ejecute, tienen precedencia sobre el contexto)
/// Nivel 2+: bloque main y scopes anidados de funciones/bloques
///
/// La búsqueda recorre desde el tope del stack hacia el nivel 0.
/// La asignación encuentra el nivel que posee el nombre y muta en su lugar.
pub struct ScopeStack {
    frames: Vec<HashMap<String, Slot>>,
}

impl ScopeStack {
    /// Crea un nuevo stack de scope con un frame inicial para el contexto inyectado.
    pub fn new(context: HashMap<String, Value>) -> Self {
        let context_frame: HashMap<String, Slot> = context
            .into_iter()
            .map(|(k, v)| (k, Slot { value: v, binding: Binding::Mutable }))
            .collect();

        Self { frames: vec![context_frame] }
    }

    /// Empuja un nuevo frame de scope vacío.
    pub fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    /// Extrae el frame de scope del tope.
    pub fn pop(&mut self) {
        if self.frames.len() > 1 {
            self.frames.pop();
        }
    }

    /// Define una nueva variable en el frame actual (tope).
    pub fn define(&mut self, name: String, value: Value, binding: Binding) {
        let frame = self.frames.last_mut().expect("scope stack is empty");
        frame.insert(name, Slot { value, binding });
    }

    /// Define una constante — colocada en el frame actual con binding Const.
    pub fn define_const(&mut self, name: String, value: Value) {
        self.define(name, value, Binding::Const);
    }

    /// Busca una variable, recorriendo desde el frame más interno al más externo.
    /// Retorna `None` si no se encuentra (la VM lo tratará como `null` o error).
    pub fn get(&self, name: &str) -> Option<&Value> {
        for frame in self.frames.iter().rev() {
            if let Some(slot) = frame.get(name) {
                return Some(&slot.value);
            }
        }
        None
    }

    /// Asigna a una variable existente.
    /// Recorre desde el más interno al más externo para encontrar el frame dueño.
    pub fn assign(&mut self, name: &str, value: Value, line: usize) -> Result<(), crate::error::CaperError> {
        for frame in self.frames.iter_mut().rev() {
            if let Some(slot) = frame.get_mut(name) {
                match slot.binding {
                    Binding::Const => {
                        return Err(crate::error::CaperError::ConstReassignment {
                            name: name.to_string(),
                            line,
                        });
                    }
                    Binding::Immutable => {
                        return Err(crate::error::CaperError::ImmutableReassignment {
                            name: name.to_string(),
                            line,
                        });
                    }
                    Binding::Mutable => {
                        slot.value = value;
                        return Ok(());
                    }
                }
            }
        }

        Err(crate::error::CaperError::UndefinedVariable {
            name: name.to_string(),
            line,
        })
    }

    /// Profundidad de anidamiento actual (número de frames).
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Retorna true si la variable nombrada existe y es `Mutable` (var).
    /// Usado para garantizar que los campos de objetos `let` no puedan ser mutados.
    pub fn is_mutable(&self, name: &str) -> bool {
        for frame in self.frames.iter().rev() {
            if let Some(slot) = frame.get(name) {
                return slot.binding == Binding::Mutable;
            }
        }
        false
    }

    /// Retorna un snapshot plano de todos los bindings visibles actualmente (el más interno gana).
    /// Usado para capturar el entorno envolvente al crear funciones/closures.
    pub fn snapshot(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        for frame in self.frames.iter() {
            for (k, slot) in frame {
                map.insert(k.clone(), slot.value.clone());
            }
        }
        map
    }
}