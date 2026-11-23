// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use crate::vm::value::Value;
use std::collections::HashMap;
use std::rc::Rc;

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
    /// Frame 0 — el entorno capturado (contexto inyectado y constantes del
    /// header). Se **comparte** por `Rc` en vez de copiarse: se instala en cada
    /// llamada a funcion, y para una funcion de nivel superior contiene el
    /// contexto completo. Copiarlo hacia que el costo de una llamada fuera
    /// proporcional al tamano del contexto aunque el script no lo tocara: una
    /// nomina de 100 trabajadores tardaba 14 segundos y una de 500 no terminaba.
    /// Sus bindings son `Mutable`; asignar a uno paga una copia por `make_mut`.
    base:   Rc<HashMap<String, Value>>,
    frames: Vec<HashMap<String, Slot>>,
}

impl ScopeStack {
    /// Crea un nuevo stack de scope con un frame inicial para el contexto inyectado.
    pub fn new(context: HashMap<String, Value>) -> Self {
        Self::from_shared(Rc::new(context))
    }

    /// Igual que `new`, pero sin copiar: comparte el entorno que recibe.
    /// Es la que usa cada llamada a funcion.
    pub fn from_shared(context: Rc<HashMap<String, Value>>) -> Self {
        Self { base: context, frames: Vec::new() }
    }

    /// Empuja un nuevo frame de scope vacío.
    pub fn push(&mut self) {
        self.frames.push(HashMap::new());
    }

    /// Extrae el frame de scope del tope.
    pub fn pop(&mut self) {
        self.frames.pop();
    }

    /// Define una nueva variable en el frame actual (tope).
    ///
    /// Retorna error si el nombre ya está declarado en el **mismo** frame (redeclaración).
    /// El shadowing en scopes anidados (frames distintos) sí está permitido.
    pub fn define(
        &mut self,
        name: String,
        value: Value,
        binding: Binding,
        line: usize,
    ) -> Result<(), crate::error::CaperError> {
        if self.frames.is_empty() { self.frames.push(HashMap::new()); }
        let frame = self.frames.last_mut().expect("recien empujado");
        if frame.contains_key(&name) {
            return Err(crate::error::CaperError::VariableRedeclaration { name, line });
        }
        frame.insert(name, Slot { value, binding });
        Ok(())
    }

    /// Define una constante — colocada en el frame actual con binding Const.
    pub fn define_const(&mut self, name: String, value: Value) {
        // Las constantes del header se cargan antes del main; la redeclaración se ignora.
        let _ = self.define(name, value, Binding::Const, 0);
    }

    /// Busca una variable, recorriendo desde el frame más interno al más externo.
    /// Retorna `None` si no se encuentra (la VM lo tratará como `null` o error).
    pub fn get(&self, name: &str) -> Option<&Value> {
        for frame in self.frames.iter().rev() {
            if let Some(slot) = frame.get(name) {
                return Some(&slot.value);
            }
        }
        self.base.get(name)
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

        // El frame base (contexto + constantes) se trata como `Mutable`, igual
        // que antes. La copia se paga aqui y solo aqui: un script que nunca
        // asigna a una variable del contexto no copia nunca.
        if self.base.contains_key(name) {
            Rc::make_mut(&mut self.base).insert(name.to_string(), value);
            return Ok(());
        }

        Err(crate::error::CaperError::UndefinedVariable {
            name: name.to_string(),
            line,
        })
    }

    /// Profundidad de anidamiento actual (número de frames).
    pub fn depth(&self) -> usize {
        self.frames.len() + 1
    }

    /// Retorna true si la variable nombrada existe y es `Mutable` (var).
    /// Usado para garantizar que los campos de objetos `let` no puedan ser mutados.
    pub fn is_mutable(&self, name: &str) -> bool {
        for frame in self.frames.iter().rev() {
            if let Some(slot) = frame.get(name) {
                return slot.binding == Binding::Mutable;
            }
        }
        self.base.contains_key(name)
    }

    /// Retorna un snapshot plano de todos los bindings visibles actualmente (el más interno gana).
    /// Usado para capturar el entorno envolvente al crear funciones/closures.
    pub fn snapshot(&self) -> Rc<HashMap<String, Value>> {
        // Sin frames propios el entorno visible ES el base: se comparte tal cual
        // y la captura sale gratis.
        if self.frames.iter().all(|f| f.is_empty()) {
            return Rc::clone(&self.base);
        }
        let mut map = (*self.base).clone();
        for frame in self.frames.iter() {
            for (k, slot) in frame {
                map.insert(k.clone(), slot.value.clone());
            }
        }
        Rc::new(map)
    }
}