// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Ejecutor de bytecode para AlcaparraLang (Fase 5).
//!
//! Bucle de ejecución plano — sin recursión por cada llamada de función.
//! Las llamadas de función empujan un CallFrame; Ret lo extrae y restaura el scope.
//! Esto permite el manejo de excepciones entre llamadas de forma correcta.

use crate::error::CaperError;
use crate::instruction::{CompiledProgram, Instruction};
use crate::loader::ImportLoader;
use crate::parser::ast::UseItems;
use crate::plugin::PluginRegistry;
use crate::stdlib;
use crate::vm::scope::{Binding, ScopeStack};
use crate::vm::value::{FunctionDef, Value};
use indexmap::IndexMap;
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const CALL_LIMIT: usize = 500;
const LOOP_LIMIT: usize = 100_000;

// ---------------------------------------------------------------------------
// Estado del iterador para foreach
// ---------------------------------------------------------------------------

enum IterState {
    Array  { items: Vec<Value>, pos: usize },
    Object { entries: Vec<(String, Value)>, pos: usize },
}

// ---------------------------------------------------------------------------
// Marco de llamada
// ---------------------------------------------------------------------------

struct CallFrame {
    chunk_id:         usize,
    ip:               usize,
    /// Scope que estaba activo antes de que se llamara esta función.
    /// Se restaura cuando se ejecuta Ret. None para el frame de nivel raíz.
    saved_scope:      Option<ScopeStack>,
    /// Programa que estaba activo antes de esta llamada (si cambió).
    saved_program:    Option<Rc<CompiledProgram>>,
    /// Profundidad del scope al entrar al frame (para deshacer con Leave).
    base_scope_depth: usize,
}

// ---------------------------------------------------------------------------
// Registro del manejador de excepciones
// ---------------------------------------------------------------------------

struct ExHandler {
    catch_addr:  usize,
    chunk_id:    usize,
    frame_depth: usize,
    scope_depth: usize,
    stack_depth: usize,
    iter_depth:  usize,
    obj_depth:   usize,
}

// ---------------------------------------------------------------------------
// Ejecutor
// ---------------------------------------------------------------------------

pub struct Executor {
    program:    Rc<CompiledProgram>,
    stack:      Vec<Value>,
    frames:     Vec<CallFrame>,
    iter_stack: Vec<IterState>,
    obj_stack:  Vec<IndexMap<String, Value>>,
    ex_stack:   Vec<ExHandler>,
    scope:      ScopeStack,
    loader:     Rc<RefCell<ImportLoader>>,
    plugins:    Rc<RefCell<PluginRegistry>>,
    loop_iters: usize,
}

impl Executor {
    pub fn new(
        program: Rc<CompiledProgram>,
        context: HashMap<String, Value>,
        loader: Rc<RefCell<ImportLoader>>,
    ) -> Self {
        Self::with_plugins(program, context, loader, Rc::new(RefCell::new(PluginRegistry::new())))
    }

    pub fn with_plugins(
        program: Rc<CompiledProgram>,
        context: HashMap<String, Value>,
        loader: Rc<RefCell<ImportLoader>>,
        plugins: Rc<RefCell<PluginRegistry>>,
    ) -> Self {
        Self {
            program,
            stack:      Vec::with_capacity(64),
            frames:     Vec::with_capacity(32),
            iter_stack: Vec::new(),
            obj_stack:  Vec::new(),
            ex_stack:   Vec::new(),
            scope:      ScopeStack::new(context),
            loader,
            plugins,
            loop_iters: 0,
        }
    }

    // -----------------------------------------------------------------------
    // Ayudantes del stack
    // -----------------------------------------------------------------------

    fn push(&mut self, v: Value) { self.stack.push(v); }

    fn pop(&mut self) -> Value {
        self.stack.pop().expect("executor: pop on empty stack")
    }

    fn peek(&self) -> &Value {
        self.stack.last().expect("executor: peek on empty stack")
    }

    fn chunk_code(&self, id: usize) -> &[Instruction] {
        &self.program.chunks[id].code
    }

    fn current_frame(&self) -> &CallFrame {
        self.frames.last().expect("executor: no active frame")
    }

    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().expect("executor: no active frame")
    }

    // -----------------------------------------------------------------------
    // Bucle principal
    // -----------------------------------------------------------------------

    pub fn run(&mut self) -> Result<Value, CaperError> {
        self.frames.push(CallFrame {
            chunk_id:         0,
            ip:               0,
            saved_scope:      None,
            saved_program:    None,
            base_scope_depth: self.scope.depth(),
        });

        loop {
            let (chunk_id, ip) = {
                let f = self.current_frame();
                (f.chunk_id, f.ip)
            };

            let instr = match self.chunk_code(chunk_id).get(ip) {
                Some(i) => i.clone(),
                None    => {
                    // Chunk agotado sin Ret/Emit explícito — se trata como Ret(Null)
                    if self.frames.len() == 1 {
                        return Err(CaperError::EmitMissing);
                    }
                    self.push(Value::Null);
                    self.do_ret();
                    continue;
                }
            };

            self.current_frame_mut().ip += 1;

            match self.step(instr) {
                Ok(true)  => return Ok(self.pop()), // Emit en el nivel raíz
                Ok(false) => {}
                Err(e) => {
                    match self.handle_error(e) {
                        Ok(())   => {} // caught, continue
                        Err(e2)  => return Err(e2), // uncaught
                    }
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // Ejecuta una instrucción. Retorna Ok(true) si se encontró Emit.
    // -----------------------------------------------------------------------

    fn step(&mut self, instr: Instruction) -> Result<bool, CaperError> {
        match instr {
            // --- Constantes ---
            Instruction::Const(v) => self.push(v),

            // --- Variables ---
            Instruction::Load(name) => {
                let v = self.scope.get(&name).cloned().unwrap_or(Value::Null);
                self.push(v);
            }
            Instruction::LetBind(name, line) => {
                let v = self.pop();
                self.scope.define(name, v, Binding::Immutable, line)?;
            }
            Instruction::VarBind(name, line) => {
                let v = self.pop();
                self.scope.define(name, v, Binding::Mutable, line)?;
            }
            Instruction::ConstBind(name) => {
                let v = self.pop();
                self.scope.define_const(name, v);
            }
            Instruction::Store(name, line) => {
                let v = self.pop();
                self.scope.assign(&name, v, line)?;
            }

            // --- Campo / índice ---
            Instruction::GetField(field, line) => {
                let obj = self.pop();
                let v = match obj {
                    Value::Object(ref m) => m.get(&field).cloned().unwrap_or(Value::Null),
                    other => return Err(CaperError::TypeError {
                        line,
                        detail: format!("cannot access field '{}' on {}", field, other.type_name()),
                    }),
                };
                self.push(v);
            }
            Instruction::SetField(field, line) => {
                let value = self.pop();
                let obj   = self.pop();
                match obj {
                    Value::Object(mut m) => { m.insert(field, value); self.push(Value::Object(m)); }
                    other => return Err(CaperError::TypeError {
                        line,
                        detail: format!("cannot set field on {}", other.type_name()),
                    }),
                }
            }
            Instruction::GetIndex(line) => {
                let idx = self.pop();
                let obj = self.pop();
                let v = do_index(obj, idx, line)?;
                self.push(v);
            }
            Instruction::SetIndex(line) => {
                let value = self.pop();
                let idx   = self.pop();
                let obj   = self.pop();
                match (obj, idx) {
                    (Value::Array(mut a), Value::Number(n)) => {
                        let i = to_usize_index(n, a.len(), line)?;
                        a[i] = value;
                        self.push(Value::Array(a));
                    }
                    (Value::Object(mut m), Value::Str(k)) => {
                        m.insert(k, value);
                        self.push(Value::Object(m));
                    }
                    (other, _) => return Err(CaperError::TypeError {
                        line,
                        detail: format!("cannot index-set on {}", other.type_name()),
                    }),
                }
            }

            // --- Operadores ---
            Instruction::BinOp(op, line) => {
                let right = self.pop();
                let left  = self.pop();
                self.push(eval_binop(op, left, right, line)?);
            }
            Instruction::Neg => {
                let v = self.pop();
                match v {
                    Value::Number(n) => self.push(Value::Number(-n)),
                    other => return Err(CaperError::TypeError {
                        line: 0,
                        detail: format!("unary '-' requires number, got {}", other.type_name()),
                    }),
                }
            }
            Instruction::Not => {
                let v = self.pop();
                self.push(Value::Bool(!v.is_truthy()));
            }

            // --- Colecciones ---
            Instruction::MakeArray(n) => {
                let mut items = vec![Value::Null; n];
                for i in (0..n).rev() { items[i] = self.pop(); }
                self.push(Value::Array(items));
            }
            Instruction::ArraySpread => {
                let spread = self.pop();
                let base   = self.pop();
                match (base, spread) {
                    (Value::Array(mut a), Value::Array(b)) => { a.extend(b); self.push(Value::Array(a)); }
                    (Value::Array(mut a), other)           => { a.push(other); self.push(Value::Array(a)); }
                    (base, _)                              => self.push(base),
                }
            }
            Instruction::ObjStart => { self.obj_stack.push(IndexMap::new()); }
            Instruction::ObjSet(key) => {
                let v = self.pop();
                self.obj_stack.last_mut().expect("ObjSet: no builder").insert(key, v);
            }
            Instruction::ObjSpread => {
                let spread = self.pop();
                if let Value::Object(m) = spread {
                    let builder = self.obj_stack.last_mut().expect("ObjSpread: no builder");
                    for (k, v) in m { builder.insert(k, v); }
                }
            }
            Instruction::ObjEnd => {
                let m = self.obj_stack.pop().expect("ObjEnd: no builder");
                self.push(Value::Object(m));
            }

            // --- Control de flujo ---
            Instruction::Jump(addr) => { self.current_frame_mut().ip = addr; }
            Instruction::JumpFalse(addr) => {
                if !self.peek().is_truthy() { self.current_frame_mut().ip = addr; }
            }
            Instruction::JumpTrue(addr) => {
                if self.peek().is_truthy() { self.current_frame_mut().ip = addr; }
            }
            Instruction::JumpNotNull(addr) => {
                if !self.peek().is_null() { self.current_frame_mut().ip = addr; }
            }

            // --- Stack de valores ---
            Instruction::Pop => { self.pop(); }
            Instruction::Dup => { let v = self.peek().clone(); self.push(v); }

            // --- Scope ---
            Instruction::Enter => self.scope.push(),
            Instruction::Leave => self.scope.pop(),

            // --- Bucles ---
            Instruction::IterInit => {
                let v = self.pop();
                self.iter_stack.push(match v {
                    Value::Array(a)  => IterState::Array  { items: a, pos: 0 },
                    Value::Object(m) => IterState::Object { entries: m.into_iter().collect(), pos: 0 },
                    other => return Err(CaperError::TypeError {
                        line: 0,
                        detail: format!("foreach: expected array or object, got {}", other.type_name()),
                    }),
                });
                self.loop_iters = 0;
            }
            Instruction::IterNext { done, key, val } => {
                self.loop_iters += 1;
                if self.loop_iters > LOOP_LIMIT {
                    return Err(CaperError::LoopLimitExceeded { limit: LOOP_LIMIT });
                }
                match self.iter_stack.last_mut().expect("IterNext: no iter") {
                    IterState::Array { items, pos } => {
                        if *pos >= items.len() {
                            self.current_frame_mut().ip = done;
                        } else {
                            let item = items[*pos].clone();
                            let idx  = *pos;
                            *pos += 1;
                            if let Some(k) = &key {
                                // define en frame fresco (Enter antes de IterNext): no puede redeclarar
                                self.scope.define(k.clone(), Value::Number(Decimal::from(idx as u64)), Binding::Immutable, 0).ok();
                            }
                            self.scope.define(val, item, Binding::Immutable, 0).ok();
                        }
                    }
                    IterState::Object { entries, pos } => {
                        if *pos >= entries.len() {
                            self.current_frame_mut().ip = done;
                        } else {
                            let (k, v) = entries[*pos].clone();
                            *pos += 1;
                            if let Some(kn) = &key {
                                self.scope.define(kn.clone(), Value::Str(k), Binding::Immutable, 0).ok();
                            }
                            self.scope.define(val, v, Binding::Immutable, 0).ok();
                        }
                    }
                }
            }
            Instruction::IterDone => { self.iter_stack.pop(); self.loop_iters = 0; }

            // --- Funciones ---
            Instruction::Closure { chunk_id, name } => {
                let captured = self.scope.snapshot();
                let chunk    = &self.program.chunks[chunk_id];
                let fd = FunctionDef {
                    name:     name.clone(),
                    params:   chunk.params.clone(),
                    body:     vec![],
                    chunk_id: Some(chunk_id),
                    program:  Some(Rc::clone(&self.program)),
                    captured,
                };
                self.push(Value::Function(Rc::new(fd)));
            }
            Instruction::Call(n_args, line) => {
                if self.frames.len() >= CALL_LIMIT {
                    return Err(CaperError::StackOverflow { limit: CALL_LIMIT });
                }
                let args   = self.pop_args(n_args);
                let callee = self.pop();
                self.setup_call(callee, args, line)?;
            }
            Instruction::CallNamed(name, n_args, line) => {
                if self.frames.len() >= CALL_LIMIT {
                    return Err(CaperError::StackOverflow { limit: CALL_LIMIT });
                }
                let args = self.pop_args(n_args);
                self.call_named_dispatch(&name, args, line)?;
            }
            Instruction::CallMethod(field, n_args, line) => {
                if self.frames.len() >= CALL_LIMIT {
                    return Err(CaperError::StackOverflow { limit: CALL_LIMIT });
                }
                let args = self.pop_args(n_args);
                let obj  = self.pop();
                // Si obj es un objeto real con un campo función, llamarlo directamente
                if let Value::Object(ref m) = obj {
                    if let Some(Value::Function(fd)) = m.get(&field).cloned() {
                        self.setup_call(Value::Function(fd), args, line)?;
                        return Ok(false);
                    }
                }
                // Sino, despachar por nombre de campo
                self.call_named_dispatch(&field, args, line)?;
            }
            Instruction::Ret => {
                let ret_val = self.pop();
                self.do_ret();
                self.push(ret_val);
            }

            // --- Salida del script ---
            Instruction::Emit(_line) => {
                // Si estamos dentro de una función (más de 1 frame), tratar como return
                if self.frames.len() > 1 {
                    let ret_val = self.pop();
                    self.do_ret();
                    self.push(ret_val);
                } else {
                    return Ok(true); // Emit de nivel raíz: el valor está en TOS
                }
            }

            // --- Excepciones ---
            Instruction::TryPush(catch_addr) => {
                let f = self.current_frame();
                self.ex_stack.push(ExHandler {
                    catch_addr,
                    chunk_id:    f.chunk_id,
                    frame_depth: self.frames.len(),
                    scope_depth: self.scope.depth(),
                    stack_depth: self.stack.len(),
                    iter_depth:  self.iter_stack.len(),
                    obj_depth:   self.obj_stack.len(),
                });
            }
            Instruction::TryPop => { self.ex_stack.pop(); }
            Instruction::Throw(line) => {
                let v = self.pop();
                let (code, message) = throw_to_parts(v);
                return Err(CaperError::UserThrow { line, code, message });
            }

            // --- Rango ---
            Instruction::Range(line) => {
                let end   = self.pop();
                let start = self.pop();
                let s = require_decimal(start, line, "range start")?;
                let e = require_decimal(end, line, "range end")?;
                let mut arr = vec![];
                let mut cur = s;
                while cur <= e { arr.push(Value::Number(cur)); cur += Decimal::ONE; }
                self.push(Value::Array(arr));
            }

            // --- Importaciones ---
            Instruction::UseModule { path, items } => {
                self.execute_use(&path, &items)?;
            }
        }
        Ok(false)
    }

    // -----------------------------------------------------------------------
    // Extrae N argumentos en orden de llamada (arg0, arg1, ...)
    // -----------------------------------------------------------------------

    fn pop_args(&mut self, n: usize) -> Vec<Value> {
        let mut args: Vec<Value> = (0..n).map(|_| self.pop()).collect();
        args.reverse();
        args
    }

    // -----------------------------------------------------------------------
    // Ret — extrae el frame y restaura el scope
    // -----------------------------------------------------------------------

    fn do_ret(&mut self) {
        let frame = self.frames.pop().expect("do_ret: no frame");
        // Deshacer cualquier frame de scope pendiente dentro de esta llamada
        while self.scope.depth() > frame.base_scope_depth {
            self.scope.pop();
        }
        // Restaurar scope y programa previos a la llamada
        if let Some(saved) = frame.saved_scope    { self.scope   = saved; }
        if let Some(saved) = frame.saved_program  { self.program = saved; }
    }

    // -----------------------------------------------------------------------
    // Configura un frame de llamada de función (NO recursa — el bucle plano lo maneja)
    // -----------------------------------------------------------------------

    fn setup_call(&mut self, callee: Value, args: Vec<Value>, line: usize) -> Result<(), CaperError> {
        match callee {
            Value::Function(fd) => {
                if args.len() != fd.params.len() {
                    return Err(CaperError::ArgumentError {
                        func: fd.name.clone().unwrap_or_else(|| "<closure>".to_string()),
                        detail: format!("expected {} arg(s), got {}", fd.params.len(), args.len()),
                    });
                }
                let chunk_id = fd.chunk_id.ok_or_else(|| CaperError::TypeError {
                    line,
                    detail: "called a tree-walker function from the bytecode executor".to_string(),
                })?;

                // Obtener el programa de esta función (puede ser de otra biblioteca)
                let fn_program = fd.program.as_ref()
                    .map(Rc::clone)
                    .unwrap_or_else(|| Rc::clone(&self.program));

                // Guardar scope actual e instalar el scope capturado
                let saved_scope = std::mem::replace(
                    &mut self.scope,
                    ScopeStack::new(fd.captured.clone()),
                );

                // Frame para parámetros / cuerpo
                self.scope.push();

                // Auto-referencia para recursión (define en frame fresco: no puede redeclarar)
                if let Some(ref name) = fd.name {
                    self.scope.define(name.clone(), Value::Function(Rc::clone(&fd)), Binding::Immutable, 0).ok();
                }
                for (param, val) in fd.params.iter().zip(args) {
                    self.scope.define(param.clone(), val, Binding::Immutable, 0).ok();
                }

                let base_scope_depth = self.scope.depth();

                // Cambiar programa si esta función pertenece a otro CompiledProgram
                let saved_program = if !Rc::ptr_eq(&fn_program, &self.program) {
                    let old = Rc::clone(&self.program);
                    self.program = fn_program;
                    Some(old)
                } else {
                    None
                };

                self.frames.push(CallFrame {
                    chunk_id,
                    ip: 0,
                    saved_scope: Some(saved_scope),
                    saved_program,
                    base_scope_depth,
                });
                Ok(())
            }
            Value::NativeFunction(nf) => {
                let result = (nf.func)(args, line)?;
                self.push(result);
                Ok(())
            }
            other => Err(CaperError::TypeError {
                line,
                detail: format!("{} is not callable", other.type_name()),
            }),
        }
    }

    // -----------------------------------------------------------------------
    // Despacho de llamada con nombre: scope → HOF → stdlib → plugins → error
    // -----------------------------------------------------------------------

    fn call_named_dispatch(&mut self, name: &str, args: Vec<Value>, line: usize) -> Result<(), CaperError> {
        // 1. Búsqueda en scope (funciones de usuario y nativas)
        if let Some(v) = self.scope.get(name).cloned() {
            match &v {
                Value::Function(_)       => return self.setup_call(v, args, line),
                Value::NativeFunction(nf) => {
                    let result = (nf.func.clone())(args, line)?;
                    self.push(result);
                    return Ok(());
                }
                _ => {}
            }
        }
        // 2. HOF (funciones de orden superior)
        if stdlib::HOF_NAMES.contains(&name) {
            let result = self.call_hof(name, args, line)?;
            self.push(result);
            return Ok(());
        }
        // 3. Biblioteca estándar
        if let Some(result) = stdlib::call(name, args.clone(), line)? {
            self.push(result);
            return Ok(());
        }
        // 4. Plugins (sin pista de módulo — buscar en todos)
        let plugin_result = self.plugins.borrow().call(None, name, args, line)?;
        if let Some(result) = plugin_result {
            self.push(result);
            return Ok(());
        }
        Err(CaperError::UndefinedVariable { name: name.to_string(), line })
    }

    // -----------------------------------------------------------------------
    // Despacho de HOF — usa run_fn() que ejecuta el closure temporalmente
    // -----------------------------------------------------------------------

    fn call_hof(&mut self, name: &str, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        match name {
            "map"        => self.hof_map(args, line),
            "filter"     |
            "find_all"   => self.hof_filter(args, line),
            "reduce"     => self.hof_reduce(args, line),
            "find"       => self.hof_find(args, line),
            "any"        => self.hof_any(args, line),
            "all"        => self.hof_all(args, line),
            "sort_by"    => self.hof_sort_by(args, line),
            "find_index" => self.hof_find_index(args, line),
            "group_by"   => self.hof_group_by(args, line),
            "partition"  => self.hof_partition(args, line),
            _ => Err(CaperError::UndefinedVariable { name: name.to_string(), line }),
        }
    }

    /// Ejecuta un valor de función hasta completarse y retorna su resultado.
    /// Usado por HOFs. Empuja y ejecuta un frame de función temporalmente.
    fn run_fn(&mut self, fd: Rc<FunctionDef>, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let pre_frame_depth = self.frames.len();
        let pre_stack_depth = self.stack.len();

        self.setup_call(Value::Function(fd), args, line)?;

        // Ejecutar hasta volver a la profundidad de frame previa a la llamada
        loop {
            if self.frames.len() <= pre_frame_depth {
                // Se ejecutó Ret — el resultado está en el stack
                return Ok(self.pop());
            }

            let (cid, ip) = {
                let f = self.current_frame();
                (f.chunk_id, f.ip)
            };

            let instr = match self.chunk_code(cid).get(ip) {
                Some(i) => i.clone(),
                None    => {
                    self.push(Value::Null);
                    self.do_ret();
                    return Ok(self.pop());
                }
            };
            self.current_frame_mut().ip += 1;

            match self.step(instr) {
                Ok(true) => {
                    // Emit dentro de función = return
                    return Ok(self.pop());
                }
                Ok(false) => {
                    if self.frames.len() <= pre_frame_depth {
                        return Ok(self.pop());
                    }
                }
                Err(e) => {
                    // Restaurar stack en caso de error
                    self.stack.truncate(pre_stack_depth);
                    return Err(e);
                }
            }
        }
    }

    fn hof_require_arr(args: &[Value], idx: usize, fn_name: &str) -> Result<Vec<Value>, CaperError> {
        match args.get(idx) {
            Some(Value::Array(a)) => Ok(a.clone()),
            Some(other) => Err(CaperError::ArgumentError {
                func: fn_name.to_string(),
                detail: format!("argument {} must be an array, got {}", idx + 1, other.type_name()),
            }),
            None => Err(CaperError::ArgumentError {
                func: fn_name.to_string(),
                detail: format!("argument {} is required", idx + 1),
            }),
        }
    }

    fn hof_require_fn(args: &[Value], idx: usize, fn_name: &str) -> Result<Rc<FunctionDef>, CaperError> {
        match args.get(idx) {
            Some(Value::Function(fd)) => Ok(Rc::clone(fd)),
            Some(other) => Err(CaperError::ArgumentError {
                func: fn_name.to_string(),
                detail: format!("argument {} must be a function, got {}", idx + 1, other.type_name()),
            }),
            None => Err(CaperError::ArgumentError {
                func: fn_name.to_string(),
                detail: format!("argument {} is required", idx + 1),
            }),
        }
    }

    fn hof_map(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "map")?;
        let fd  = Self::hof_require_fn(&args, 1, "map")?;
        let mut out = Vec::with_capacity(arr.len());
        for item in arr {
            out.push(self.run_fn(Rc::clone(&fd), vec![item], line)?);
        }
        Ok(Value::Array(out))
    }

    fn hof_filter(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "filter")?;
        let fd  = Self::hof_require_fn(&args, 1, "filter")?;
        let mut out = vec![];
        for item in arr {
            if self.run_fn(Rc::clone(&fd), vec![item.clone()], line)?.is_truthy() {
                out.push(item);
            }
        }
        Ok(Value::Array(out))
    }

    fn hof_reduce(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr  = Self::hof_require_arr(&args, 0, "reduce")?;
        let fd   = Self::hof_require_fn(&args, 1, "reduce")?;
        let init = args.get(2).cloned().unwrap_or(Value::Null);
        let mut acc = init;
        for item in arr {
            acc = self.run_fn(Rc::clone(&fd), vec![acc, item], line)?;
        }
        Ok(acc)
    }

    fn hof_find(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "find")?;
        let fd  = Self::hof_require_fn(&args, 1, "find")?;
        for item in arr {
            if self.run_fn(Rc::clone(&fd), vec![item.clone()], line)?.is_truthy() {
                return Ok(item);
            }
        }
        Ok(Value::Null)
    }

    fn hof_any(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "any")?;
        let fd  = Self::hof_require_fn(&args, 1, "any")?;
        for item in arr {
            if self.run_fn(Rc::clone(&fd), vec![item], line)?.is_truthy() {
                return Ok(Value::Bool(true));
            }
        }
        Ok(Value::Bool(false))
    }

    fn hof_all(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "all")?;
        let fd  = Self::hof_require_fn(&args, 1, "all")?;
        for item in arr {
            if !self.run_fn(Rc::clone(&fd), vec![item], line)?.is_truthy() {
                return Ok(Value::Bool(false));
            }
        }
        Ok(Value::Bool(true))
    }

    fn hof_sort_by(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let mut arr = Self::hof_require_arr(&args, 0, "sort_by")?;
        // sort_by(arr, "campo") — ordenar objetos por campo nombrado
        if let Some(Value::Str(key)) = args.get(1) {
            let key = key.clone();
            let mut err: Option<CaperError> = None;
            arr.sort_by(|a, b| {
                if err.is_some() { return std::cmp::Ordering::Equal; }
                let va = match a { Value::Object(m) => m.get(&key).cloned().unwrap_or(Value::Null), other => other.clone() };
                let vb = match b { Value::Object(m) => m.get(&key).cloned().unwrap_or(Value::Null), other => other.clone() };
                match crate::stdlib::arrays::cmp_values(&va, &vb, line) {
                    Ok(ord) => ord,
                    Err(e)  => { err = Some(e); std::cmp::Ordering::Equal }
                }
            });
            if let Some(e) = err { return Err(e); }
            return Ok(Value::Array(arr));
        }
        let fd  = Self::hof_require_fn(&args, 1, "sort_by")?;
        let mut indexed: Vec<(usize, Value)> = arr.into_iter().enumerate().collect();
        let mut err: Option<CaperError> = None;
        // No se puede llamar run_fn dentro del comparador de sort (problemas de borrow),
        // por lo que se precomputan claves o se usa un wrapper.
        // Enfoque simple: ejecutar sort_by mediante un sort estable con captura de errores.
        let fdc = Rc::clone(&fd);
        // ¿Recolectar claves de comparación? No — sort_by estándar espera comparador (a,b).
        // Workaround sin unsafe: capturar errores via RefCell.
        let err_cell = std::cell::RefCell::new(None::<CaperError>);
        indexed.sort_by(|(_, a), (_, b)| {
            if err_cell.borrow().is_some() { return std::cmp::Ordering::Equal; }
            // Se necesita &mut self aquí, lo que conflictúa con el closure.
            // Alternativa: ejecutar la función por separado vía manipulación de stack.
            // Limitación conocida; por ahora retornar Equal y usar un pase separado.
            let _ = (a, b, &fdc);
            std::cmp::Ordering::Equal
        });
        // Ordenamiento real usando un pase comparador separado:
        let n = indexed.len();
        for i in 0..n {
            for j in (i+1)..n {
                if err.is_some() { break; }
                let a = indexed[i].1.clone();
                let b = indexed[j].1.clone();
                match self.run_fn(Rc::clone(&fd), vec![a, b], line) {
                    Ok(Value::Number(n)) if n > Decimal::ZERO => {
                        indexed.swap(i, j);
                    }
                    Ok(_)  => {}
                    Err(e) => { err = Some(e); break; }
                }
            }
        }
        if let Some(e) = err { return Err(e); }
        Ok(Value::Array(indexed.into_iter().map(|(_, v)| v).collect()))
    }

    fn hof_find_index(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "find_index")?;
        let fd  = Self::hof_require_fn(&args, 1, "find_index")?;
        for (i, item) in arr.into_iter().enumerate() {
            if self.run_fn(Rc::clone(&fd), vec![item], line)?.is_truthy() {
                return Ok(Value::Number(Decimal::from(i as u64)));
            }
        }
        Ok(Value::Number(Decimal::from(-1i64)))
    }

    fn hof_group_by(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "group_by")?;
        let fd  = Self::hof_require_fn(&args, 1, "group_by")?;
        let mut map: IndexMap<String, Vec<Value>> = IndexMap::new();
        for item in arr {
            let key = self.run_fn(Rc::clone(&fd), vec![item.clone()], line)?;
            map.entry(key.to_string()).or_default().push(item);
        }
        Ok(Value::Object(map.into_iter().map(|(k, v)| (k, Value::Array(v))).collect()))
    }

    fn hof_partition(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "partition")?;
        let fd  = Self::hof_require_fn(&args, 1, "partition")?;
        let (mut yes, mut no) = (vec![], vec![]);
        for item in arr {
            if self.run_fn(Rc::clone(&fd), vec![item.clone()], line)?.is_truthy() {
                yes.push(item);
            } else {
                no.push(item);
            }
        }
        Ok(Value::Array(vec![Value::Array(yes), Value::Array(no)]))
    }

    // -----------------------------------------------------------------------
    // Manejo de excepciones
    // -----------------------------------------------------------------------

    fn handle_error(&mut self, e: CaperError) -> Result<(), CaperError> {
        let handler = match self.ex_stack.pop() {
            Some(h) => h,
            None    => return Err(e),
        };

        // Deshacer frames de llamada (restaurando scopes y programas en el camino)
        while self.frames.len() > handler.frame_depth {
            let frame = self.frames.pop().unwrap();
            while self.scope.depth() > frame.base_scope_depth { self.scope.pop(); }
            if let Some(saved) = frame.saved_scope   { self.scope   = saved; }
            if let Some(saved) = frame.saved_program { self.program = saved; }
        }
        // Restaurar scope a la profundidad guardada por el manejador
        while self.scope.depth() > handler.scope_depth {
            self.scope.pop();
        }
        // Truncar stacks de valores / iteradores / objetos
        self.stack.truncate(handler.stack_depth);
        self.iter_stack.truncate(handler.iter_depth);
        self.obj_stack.truncate(handler.obj_depth);

        // Saltar al bloque catch
        if let Some(frame) = self.frames.last_mut() {
            frame.chunk_id = handler.chunk_id;
            frame.ip       = handler.catch_addr;
        } else {
            self.frames.push(CallFrame {
                chunk_id:         handler.chunk_id,
                ip:               handler.catch_addr,
                saved_scope:      None,
                saved_program:    None,
                base_scope_depth: self.scope.depth(),
            });
        }

        // Empujar el valor de error (será vinculado por LetBind en el bloque catch)
        self.push(error_to_value(&e));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Importaciones
    // -----------------------------------------------------------------------

    fn execute_use(&mut self, path: &[String], items: &UseItems) -> Result<(), CaperError> {
        crate::stdlib::validate_use(path)?;
        if path.first().map(|s| s.as_str()) == Some("std") {
            return Ok(()); // stdlib — no requiere carga
        }
        // Biblioteca de usuario
        let (exported, lib_prog) = crate::loader::ImportLoader::load(&self.loader, path)?;
        let lib_prog = Rc::new(lib_prog);

        // Vincular las funciones exportadas, adjuntando el programa de la biblioteca
        // para que el ejecutor de bytecode pueda llamarlas
        let rewrite = |mut v: Value| -> Value {
            if let Value::Function(ref fd) = v {
                if fd.program.is_none() {
                    let new_fd = FunctionDef {
                        name:     fd.name.clone(),
                        params:   fd.params.clone(),
                        body:     fd.body.clone(),
                        chunk_id: fd.chunk_id,
                        program:  Some(Rc::clone(&lib_prog)),
                        captured: fd.captured.clone(),
                    };
                    v = Value::Function(Rc::new(new_fd));
                }
            }
            v
        };

        match items {
            UseItems::All => {
                for (name, val) in exported {
                    self.scope.define(name, rewrite(val), Binding::Immutable, 0).ok();
                }
            }
            UseItems::Single => {
                if let Some(fn_name) = path.last() {
                    if let Some(val) = exported.get(fn_name).cloned() {
                        self.scope.define(fn_name.clone(), rewrite(val), Binding::Immutable, 0).ok();
                    }
                }
            }
            UseItems::Named(names) => {
                for name in names {
                    if let Some(val) = exported.get(name).cloned() {
                        self.scope.define(name.clone(), rewrite(val), Binding::Immutable, 0).ok();
                    }
                }
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Funciones auxiliares libres
// ---------------------------------------------------------------------------

fn eval_binop(
    op: crate::parser::ast::BinOp,
    left: Value,
    right: Value,
    line: usize,
) -> Result<Value, CaperError> {
    use crate::parser::ast::BinOp;
    use rust_decimal::MathematicalOps;

    match op {
        BinOp::Add => match (&left, &right) {
            (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
            (Value::Str(a),   Value::Str(b))    => Ok(Value::Str(format!("{}{}", a, b))),
            (Value::Str(a),   other)             => Ok(Value::Str(format!("{}{}", a, other))),
            (other,           Value::Str(b))     => Ok(Value::Str(format!("{}{}", other, b))),
            _ => Err(CaperError::TypeError {
                line,
                detail: format!("'+' requires numbers or strings, got {} and {}", left.type_name(), right.type_name()),
            }),
        },
        BinOp::Sub => arith(left, right, line, |a, b| a - b),
        BinOp::Mul => arith(left, right, line, |a, b| a * b),
        BinOp::Div => {
            let (a, b) = require_numbers(&left, &right, line, "/")?;
            if b.is_zero() { return Err(CaperError::DivisionByZero { line }); }
            Ok(Value::Number(a / b))
        }
        BinOp::Mod => {
            let (a, b) = require_numbers(&left, &right, line, "%")?;
            if b.is_zero() { return Err(CaperError::DivisionByZero { line }); }
            Ok(Value::Number(a % b))
        }
        BinOp::Pow => {
            let (a, b) = require_numbers(&left, &right, line, "^")?;
            Ok(Value::Number(a.powd(b)))
        }
        BinOp::Eq    => Ok(Value::Bool(values_eq(&left, &right))),
        BinOp::NotEq => Ok(Value::Bool(!values_eq(&left, &right))),
        BinOp::Lt    => compare(left, right, line, |o| o.is_lt()),
        BinOp::LtEq  => compare(left, right, line, |o| o.is_le()),
        BinOp::Gt    => compare(left, right, line, |o| o.is_gt()),
        BinOp::GtEq  => compare(left, right, line, |o| o.is_ge()),
        BinOp::And | BinOp::Or => unreachable!("And/Or compilados como saltos"),
    }
}

fn arith(l: Value, r: Value, line: usize, f: impl Fn(Decimal, Decimal) -> Decimal) -> Result<Value, CaperError> {
    let (a, b) = require_numbers(&l, &r, line, "arithmetic")?;
    Ok(Value::Number(f(a, b)))
}

fn require_numbers(l: &Value, r: &Value, line: usize, op: &str) -> Result<(Decimal, Decimal), CaperError> {
    match (l.as_decimal(), r.as_decimal()) {
        (Some(a), Some(b)) => Ok((a, b)),
        _ => Err(CaperError::TypeError {
            line,
            detail: format!("'{}' requires numbers, got {} and {}", op, l.type_name(), r.type_name()),
        }),
    }
}

fn compare(l: Value, r: Value, line: usize, pred: impl Fn(std::cmp::Ordering) -> bool) -> Result<Value, CaperError> {
    let ord = match (&l, &r) {
        (Value::Number(a), Value::Number(b)) => a.cmp(b),
        (Value::Str(a),    Value::Str(b))    => a.cmp(b),
        _ => return Err(CaperError::TypeError {
            line,
            detail: format!("cannot compare {} and {}", l.type_name(), r.type_name()),
        }),
    };
    Ok(Value::Bool(pred(ord)))
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null,      Value::Null)      => true,
        (Value::Bool(x),   Value::Bool(y))   => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::Str(x),    Value::Str(y))    => x == y,
        (Value::Array(x),  Value::Array(y))  => {
            x.len() == y.len() && x.iter().zip(y).all(|(a, b)| values_eq(a, b))
        }
        _ => false,
    }
}

fn do_index(obj: Value, idx: Value, line: usize) -> Result<Value, CaperError> {
    match (&obj, &idx) {
        (Value::Array(a), Value::Number(n)) => {
            let i = n.to_i64_sat();
            let len = a.len() as i64;
            let actual = if i < 0 { len + i } else { i };
            if actual < 0 || actual >= len {
                return Err(CaperError::IndexOutOfBounds { index: i, length: a.len() });
            }
            Ok(a[actual as usize].clone())
        }
        (Value::Object(m), Value::Str(k)) => Ok(m.get(k).cloned().unwrap_or(Value::Null)),
        (Value::Str(s), Value::Number(n)) => {
            let i = n.to_i64_sat();
            let chars: Vec<char> = s.chars().collect();
            let len = chars.len() as i64;
            let actual = if i < 0 { len + i } else { i };
            if actual < 0 || actual >= len {
                return Err(CaperError::IndexOutOfBounds { index: i, length: chars.len() });
            }
            Ok(Value::Str(chars[actual as usize].to_string()))
        }
        _ => Err(CaperError::TypeError {
            line,
            detail: format!("cannot index {} with {}", obj.type_name(), idx.type_name()),
        }),
    }
}

fn throw_to_parts(v: Value) -> (String, String) {
    match v {
        Value::Object(ref m) => {
            let code = m.get("code").and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_else(|| "THROW".to_string());
            let msg  = m.get("message").and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                .unwrap_or_else(|| v.to_string());
            (code, msg)
        }
        Value::Str(s) => ("THROW".to_string(), s),
        other         => ("THROW".to_string(), other.to_string()),
    }
}

fn error_to_value(e: &CaperError) -> Value {
    let (code, message) = match e {
        CaperError::UserThrow { code, message, .. } => (code.clone(), message.clone()),
        other => (other.code().to_string(), other.to_string()),
    };
    let mut m = IndexMap::new();
    m.insert("code".to_string(),    Value::Str(code));
    m.insert("message".to_string(), Value::Str(message));
    Value::Object(m)
}

fn require_decimal(v: Value, line: usize, what: &str) -> Result<Decimal, CaperError> {
    v.as_decimal().ok_or_else(|| CaperError::TypeError {
        line,
        detail: format!("{} must be a number", what),
    })
}

fn to_usize_index(n: Decimal, len: usize, _line: usize) -> Result<usize, CaperError> {
    use rust_decimal::prelude::ToPrimitive;
    let i = n.to_i64().unwrap_or(-1);
    let actual = if i < 0 { len as i64 + i } else { i };
    if actual < 0 || actual as usize >= len {
        Err(CaperError::IndexOutOfBounds { index: i, length: len })
    } else {
        Ok(actual as usize)
    }
}

trait DecimalExt { fn to_i64_sat(&self) -> i64; }
impl DecimalExt for Decimal {
    fn to_i64_sat(&self) -> i64 {
        use rust_decimal::prelude::ToPrimitive;
        self.to_i64().unwrap_or(0)
    }
}
