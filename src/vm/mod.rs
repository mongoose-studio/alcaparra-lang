// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Intérprete tree-walking (Fases 1-3).
//!
//! Recorre el AST directamente sin compilar a bytecode.
//! La VM de bytecode fue añadida en la Fase 5 para mayor rendimiento.

pub mod executor;
pub mod scope;
pub mod value;

use crate::error::CaperError;
use crate::loader::ImportLoader;
use crate::parser::ast::*;
use crate::stdlib;
use indexmap::IndexMap;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use scope::{Binding, ScopeStack};
use value::{FunctionDef, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const STACK_LIMIT: usize = 500;
const LOOP_LIMIT: usize = 100_000;

/// Señal usada para control de flujo dentro de la VM.
enum Signal {
    None,
    Emit(Value),
    Break,
    Continue,
}

pub fn execute(
    program: Program,
    context: HashMap<String, Value>,
    loader: Rc<RefCell<ImportLoader>>,
) -> Result<Value, CaperError> {
    let mut vm = Vm::new(context, loader);
    vm.run_program(program)
}

/// Ejecuta un archivo `.caper` en modo biblioteca: sólo se registran
/// declaraciones `fn`. Los `emit` de nivel raíz y otros statements
/// emiten una advertencia por stderr y se ignoran — no afectan el resultado.
pub fn execute_library(
    program: Program,
    loader: Rc<RefCell<ImportLoader>>,
) -> Result<HashMap<String, Value>, CaperError> {
    let mut vm = Vm::new(HashMap::new(), Rc::clone(&loader));

    // Las constantes del header son privadas a esta biblioteca
    if let Some(header) = program.header {
        vm.scope.push();
        for c in header.constants {
            let val = vm.eval_expr(&c.value)?;
            vm.scope.define_const(c.name, val);
        }
    }

    vm.scope.push();
    for stmt in &program.body {
        match stmt {
            Stmt::FnDecl { .. } | Stmt::Use { .. } => {
                vm.run_stmt(stmt)?;
            }
            Stmt::Emit { .. } => {
                eprintln!("[alcaparra] WARNING: top-level emit in library file ignored");
            }
            _ => {
                eprintln!("[alcaparra] WARNING: non-fn statement in library file ignored");
            }
        }
    }

    // Exportar sólo valores de tipo Function
    Ok(vm.scope.snapshot()
        .into_iter()
        .filter(|(_, v)| v.is_function())
        .collect())
}

struct Vm {
    scope: ScopeStack,
    call_depth: usize,
    loader: Rc<RefCell<ImportLoader>>,
}

impl Vm {
    fn new(context: HashMap<String, Value>, loader: Rc<RefCell<ImportLoader>>) -> Self {
        Self {
            scope: ScopeStack::new(context),
            call_depth: 0,
            loader,
        }
    }

    fn run_program(&mut self, program: Program) -> Result<Value, CaperError> {
        // Cargar constantes del header en un nuevo frame (tienen prioridad sobre el contexto)
        if let Some(header) = program.header {
            self.scope.push();
            for c in header.constants {
                let val = self.eval_expr(&c.value)?;
                self.scope.define_const(c.name, val);
            }
        }

        // Ejecutar el cuerpo del programa
        self.scope.push();
        let signal = self.run_stmts(&program.body)?;

        match signal {
            Signal::Emit(v) => Ok(v),
            _ => Err(CaperError::EmitMissing),
        }
    }

    fn run_stmts(&mut self, stmts: &[Stmt]) -> Result<Signal, CaperError> {
        for stmt in stmts {
            let signal = self.run_stmt(stmt)?;
            match signal {
                Signal::None => {}
                other => return Ok(other),
            }
        }
        Ok(Signal::None)
    }

    fn run_stmt(&mut self, stmt: &Stmt) -> Result<Signal, CaperError> {
        match stmt {
            Stmt::Let { name, value, line } => {
                let val = self.eval_expr(value)?;
                self.scope.define(name.clone(), val, Binding::Immutable, *line)?;
            }
            Stmt::Var { name, value, line } => {
                let val = self.eval_expr(value)?;
                self.scope.define(name.clone(), val, Binding::Mutable, *line)?;
            }
            Stmt::Assign { target, value, line } => {
                let val = self.eval_expr(value)?;
                self.do_assign(target, val, *line)?;
            }
            Stmt::Emit { value, .. } => {
                let val = self.eval_expr(value)?;
                return Ok(Signal::Emit(val));
            }
            Stmt::If { condition, then_block, else_if_clauses, else_block, .. } => {
                if self.eval_expr(condition)?.is_truthy() {
                    self.scope.push();
                    let sig = self.run_stmts(then_block)?;
                    self.scope.pop();
                    if !matches!(sig, Signal::None) { return Ok(sig); }
                } else {
                    let mut matched = false;
                    for (cond, body) in else_if_clauses {
                        if self.eval_expr(cond)?.is_truthy() {
                            self.scope.push();
                            let sig = self.run_stmts(body)?;
                            self.scope.pop();
                            if !matches!(sig, Signal::None) { return Ok(sig); }
                            matched = true;
                            break;
                        }
                    }
                    if !matched {
                        if let Some(body) = else_block {
                            self.scope.push();
                            let sig = self.run_stmts(body)?;
                            self.scope.pop();
                            if !matches!(sig, Signal::None) { return Ok(sig); }
                        }
                    }
                }
            }
            Stmt::While { condition, body, .. } => {
                let mut iters = 0usize;
                loop {
                    if !self.eval_expr(condition)?.is_truthy() { break; }
                    iters += 1;
                    if iters > LOOP_LIMIT {
                        return Err(CaperError::LoopLimitExceeded { limit: LOOP_LIMIT });
                    }
                    self.scope.push();
                    let sig = self.run_stmts(body)?;
                    self.scope.pop();
                    match sig {
                        Signal::Break    => break,
                        Signal::Continue => continue,
                        Signal::Emit(v)  => return Ok(Signal::Emit(v)),
                        Signal::None     => {}
                    }
                }
            }
            Stmt::Foreach { key, value: val_name, iter, body, line } => {
                let collection = self.eval_expr(iter)?;
                let mut iters = 0usize;
                match collection {
                    Value::Array(arr) => {
                        for (idx, item) in arr.into_iter().enumerate() {
                            iters += 1;
                            if iters > LOOP_LIMIT {
                                return Err(CaperError::LoopLimitExceeded { limit: LOOP_LIMIT });
                            }
                            self.scope.push();
                            if let Some(k) = key {
                                self.scope.define(
                                    k.clone(),
                                    Value::Number(Decimal::from(idx as u64)),
                                    Binding::Immutable,
                                    0,
                                ).ok();
                            }
                            self.scope.define(val_name.clone(), item, Binding::Immutable, 0).ok();
                            let sig = self.run_stmts(body)?;
                            self.scope.pop();
                            match sig {
                                Signal::Break    => break,
                                Signal::Continue => continue,
                                Signal::Emit(v)  => return Ok(Signal::Emit(v)),
                                Signal::None     => {}
                            }
                        }
                    }
                    Value::Object(obj) => {
                        for (k, v) in obj.into_iter() {
                            iters += 1;
                            if iters > LOOP_LIMIT {
                                return Err(CaperError::LoopLimitExceeded { limit: LOOP_LIMIT });
                            }
                            self.scope.push();
                            if let Some(key_name) = key {
                                self.scope.define(key_name.clone(), Value::Str(k), Binding::Immutable, 0).ok();
                            }
                            self.scope.define(val_name.clone(), v, Binding::Immutable, 0).ok();
                            let sig = self.run_stmts(body)?;
                            self.scope.pop();
                            match sig {
                                Signal::Break    => break,
                                Signal::Continue => continue,
                                Signal::Emit(v)  => return Ok(Signal::Emit(v)),
                                Signal::None     => {}
                            }
                        }
                    }
                    other => {
                        return Err(CaperError::TypeError {
                            line: *line,
                            detail: format!("foreach: expected array or object, got {}", other.type_name()),
                        });
                    }
                }
            }
            Stmt::Break    { .. } => return Ok(Signal::Break),
            Stmt::Continue { .. } => return Ok(Signal::Continue),
            Stmt::FnDecl { name, params, body, .. } => {
                let captured = self.scope.snapshot();
                let func = Value::Function(Rc::new(FunctionDef {
                    name: Some(name.clone()),
                    params: params.clone(),
                    body: body.clone(),
                    chunk_id: None,
                    program: None,
                    captured,
                }));
                self.scope.define(name.clone(), func, Binding::Immutable, 0).ok();
            }
            Stmt::TryCatch { try_block, error_var, catch_block, .. } => {
                self.scope.push();
                let sig = self.run_stmts(try_block);
                self.scope.pop();
                match sig {
                    Ok(s) => {
                        if !matches!(s, Signal::None) { return Ok(s); }
                    }
                    Err(e) => {
                        self.scope.push();
                        self.scope.define(error_var.clone(), error_to_value(&e), Binding::Immutable, 0).ok();
                        let sig = self.run_stmts(catch_block)?;
                        self.scope.pop();
                        if !matches!(sig, Signal::None) { return Ok(sig); }
                    }
                }
            }
            Stmt::Use { path, items, .. } => {
                if path.first().map(|s| s.as_str()) == Some("std") {
                    // Stdlib: validar ruta; las funciones se resuelven por nombre en tiempo de llamada
                    stdlib::validate_use(path)?;
                } else {
                    // Biblioteca de usuario: cargar e inyectar funciones en el scope actual
                    let (fns, _compiled) = ImportLoader::load(&self.loader, path)?;
                    match items {
                        UseItems::All | UseItems::Single => {
                            for (name, val) in fns {
                                self.scope.define(name, val, Binding::Immutable, 0).ok();
                            }
                        }
                        UseItems::Named(names) => {
                            for name in names {
                                let val = fns.get(name).cloned().ok_or_else(|| {
                                    CaperError::ImportNotFound {
                                        path: format!("{}.{name}", path.join(".")),
                                    }
                                })?;
                                self.scope.define(name.clone(), val, Binding::Immutable, 0).ok();
                            }
                        }
                    }
                }
            }

            Stmt::Throw { value, line } => {
                let val = self.eval_expr(value)?;
                return Err(match &val {
                    Value::Str(s) => CaperError::UserThrow {
                        line: *line,
                        code: "THROW".to_string(),
                        message: s.clone(),
                    },
                    Value::Object(m) => CaperError::UserThrow {
                        line: *line,
                        code: m.get("code")
                            .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                            .unwrap_or_else(|| "THROW".to_string()),
                        message: m.get("message")
                            .and_then(|v| if let Value::Str(s) = v { Some(s.clone()) } else { None })
                            .unwrap_or_else(|| val.to_string()),
                    },
                    other => CaperError::UserThrow {
                        line: *line,
                        code: "THROW".to_string(),
                        message: other.to_string(),
                    },
                });
            }
            Stmt::ExprStmt(expr) => {
                self.eval_expr(expr)?;
            }
        }
        Ok(Signal::None)
    }

    // --- Expression evaluation ---

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, CaperError> {
        match expr {
            Expr::Number(n)    => Ok(Value::Number(*n)),
            Expr::StringLit(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b)      => Ok(Value::Bool(*b)),
            Expr::Null         => Ok(Value::Null),

            Expr::Ident(name) => {
                Ok(self.scope.get(name).cloned().unwrap_or(Value::Null))
            }

            Expr::BinOp { op, left, right, line } => {
                self.eval_binop(op, left, right, *line)
            }

            Expr::UnaryOp { op, operand, line } => {
                let val = self.eval_expr(operand)?;
                match op {
                    UnaryOp::Neg => {
                        match val.as_decimal() {
                            Some(n) => Ok(Value::Number(-n)),
                            None => Err(CaperError::TypeError {
                                line: *line,
                                detail: format!("Cannot negate {}", val.type_name()),
                            }),
                        }
                    }
                    UnaryOp::Not => Ok(Value::Bool(!val.is_truthy())),
                }
            }

            Expr::NullCoalesce { left, right, .. } => {
                let v = self.eval_expr(left)?;
                if v.is_null() { self.eval_expr(right) } else { Ok(v) }
            }

            Expr::Array(elements) => {
                let mut arr = vec![];
                for el in elements {
                    match el {
                        ArrayElement::Expr(e) => arr.push(self.eval_expr(e)?),
                        ArrayElement::Spread(e) => {
                            if let Value::Array(inner) = self.eval_expr(e)? {
                                arr.extend(inner);
                            }
                        }
                    }
                }
                Ok(Value::Array(arr))
            }

            Expr::Object(fields) => {
                let mut map = IndexMap::new();
                for field in fields {
                    match field {
                        ObjectField::Named(k, v) => { map.insert(k.clone(), self.eval_expr(v)?); }
                        ObjectField::Spread(e) => {
                            if let Value::Object(inner) = self.eval_expr(e)? {
                                map.extend(inner);
                            }
                        }
                    }
                }
                Ok(Value::Object(map))
            }

            Expr::Index { object, index, line } => {
                let obj = self.eval_expr(object)?;
                let idx = self.eval_expr(index)?;
                self.do_index(obj, idx, *line)
            }

            Expr::Field { object, field, line } => {
                let obj = self.eval_expr(object)?;
                match obj {
                    Value::Object(m) => Ok(m.get(field).cloned().unwrap_or(Value::Null)),
                    _ => Err(CaperError::TypeError {
                        line: *line,
                        detail: format!("Cannot access field '{field}' on {}", obj.type_name()),
                    }),
                }
            }

            Expr::Range { start, end, line } => {
                let s = self.eval_expr(start)?;
                let e = self.eval_expr(end)?;
                match (s.as_decimal(), e.as_decimal()) {
                    (Some(s), Some(e)) => {
                        let start_i = s.to_i64_saturating();
                        let end_i   = e.to_i64_saturating();
                        let arr = (start_i..end_i)
                            .map(|n| Value::Number(Decimal::from(n)))
                            .collect();
                        Ok(Value::Array(arr))
                    }
                    _ => Err(CaperError::TypeError {
                        line: *line,
                        detail: "Range operands must be numbers".to_string(),
                    }),
                }
            }

            Expr::Call { callee, args, line } => {
                self.eval_call(callee, args, *line)
            }

            Expr::Closure { params, body, .. } => {
                let captured = self.scope.snapshot();
                let stmts = match body {
                    ClosureBody::Expr(e) => vec![Stmt::Emit { value: *e.clone(), line: 0 }],
                    ClosureBody::Block(stmts) => stmts.clone(),
                };
                Ok(Value::Function(Rc::new(FunctionDef {
                    name: None,
                    params: params.clone(),
                    body: stmts,
                    chunk_id: None,
                    program: None,
                    captured,
                })))
            }

            Expr::IfExpr { condition, then_block, else_block, .. } => {
                let cond = self.eval_expr(condition)?;
                let stmts = if cond.is_truthy() { then_block } else { else_block };
                self.scope.push();
                let sig = self.run_stmts(stmts)?;
                self.scope.pop();
                match sig {
                    Signal::Emit(v) => Ok(v),
                    _               => Ok(Value::Null),
                }
            }

            Expr::Match { subject, arms, line } => {
                let val = self.eval_expr(subject)?;
                for arm in arms {
                    if self.match_arm(&val, &arm.pattern, &arm.guard, *line)? {
                        return self.eval_expr(&arm.body);
                    }
                }
                Ok(Value::Null) // ningún brazo coincidió y no hay comodín — retorna null
            }

            Expr::TryCatchExpr { try_block, error_var, catch_block, .. } => {
                self.scope.push();
                let sig = self.run_stmts(try_block);
                self.scope.pop();
                match sig {
                    Ok(Signal::Emit(v)) => Ok(v),
                    Ok(_) => Ok(Value::Null),
                    Err(e) => {
                        self.scope.push();
                        self.scope.define(error_var.clone(), error_to_value(&e), Binding::Immutable, 0).ok();
                        let sig = self.run_stmts(catch_block)?;
                        self.scope.pop();
                        match sig {
                            Signal::Emit(v) => Ok(v),
                            _ => Ok(Value::Null),
                        }
                    }
                }
            }
        }
    }

    fn eval_binop(&mut self, op: &BinOp, left: &Expr, right: &Expr, line: usize) -> Result<Value, CaperError> {
        // Cortocircuito para operadores lógicos
        if *op == BinOp::And {
            let l = self.eval_expr(left)?;
            return if !l.is_truthy() { Ok(Value::Bool(false)) } else {
                Ok(Value::Bool(self.eval_expr(right)?.is_truthy()))
            };
        }
        if *op == BinOp::Or {
            let l = self.eval_expr(left)?;
            return if l.is_truthy() { Ok(Value::Bool(true)) } else {
                Ok(Value::Bool(self.eval_expr(right)?.is_truthy()))
            };
        }

        let l = self.eval_expr(left)?;
        let r = self.eval_expr(right)?;

        match op {
            BinOp::Add => {
                match (&l, &r) {
                    (Value::Str(a), _) => Ok(Value::Str(format!("{a}{r}"))),
                    (_, Value::Str(b)) => Ok(Value::Str(format!("{l}{b}"))),
                    _ => {
                        let a = l.as_decimal().ok_or_else(|| CaperError::TypeError {
                            line, detail: format!("Cannot add {} and {}", l.type_name(), r.type_name()),
                        })?;
                        let b = r.as_decimal().ok_or_else(|| CaperError::TypeError {
                            line, detail: format!("Cannot add {} and {}", l.type_name(), r.type_name()),
                        })?;
                        Ok(Value::Number(a + b))
                    }
                }
            }
            BinOp::Sub | BinOp::Mul | BinOp::Pow => {
                let a = numeric(&l, op, line)?;
                let b = numeric(&r, op, line)?;
                let result = match op {
                    BinOp::Sub => a - b,
                    BinOp::Mul => a * b,
                    BinOp::Pow => {
                        let exp = b.to_i64_saturating();
                        a.powi(exp)
                    }
                    _ => unreachable!(),
                };
                Ok(Value::Number(result))
            }
            BinOp::Div => {
                let a = numeric(&l, op, line)?;
                let b = numeric(&r, op, line)?;
                if b.is_zero() { return Err(CaperError::DivisionByZero { line }); }
                Ok(Value::Number(a / b))
            }
            BinOp::Mod => {
                let a = numeric(&l, op, line)?;
                let b = numeric(&r, op, line)?;
                if b.is_zero() { return Err(CaperError::DivisionByZero { line }); }
                Ok(Value::Number(a % b))
            }
            BinOp::Eq  => Ok(Value::Bool(l == r)),
            BinOp::NotEq => Ok(Value::Bool(l != r)),
            BinOp::Lt  => Ok(Value::Bool(cmp_values(&l, &r, line)? < 0)),
            BinOp::LtEq => Ok(Value::Bool(cmp_values(&l, &r, line)? <= 0)),
            BinOp::Gt  => Ok(Value::Bool(cmp_values(&l, &r, line)? > 0)),
            BinOp::GtEq => Ok(Value::Bool(cmp_values(&l, &r, line)? >= 0)),
            BinOp::And | BinOp::Or => unreachable!(), // handled above
        }
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], line: usize) -> Result<Value, CaperError> {
        if self.call_depth >= STACK_LIMIT {
            return Err(CaperError::StackOverflow { limit: STACK_LIMIT });
        }

        // Evaluar argumentos de forma eager
        let arg_values: Vec<Value> = args.iter()
            .map(|a| self.eval_expr(a))
            .collect::<Result<_, _>>()?;

        // Intentar resolver el callee a un Value::Function (definido por usuario)
        // o recaer en la stdlib por nombre.
        let callee_val = self.eval_callee(callee)?;

        match callee_val {
            Some(Value::Function(fd)) => {
                self.call_function(fd, arg_values, line)
            }
            Some(other) => Err(CaperError::TypeError {
                line,
                detail: format!("{} is not callable", other.type_name()),
            }),
            None => {
                // El callee no era un valor vinculado a variable — extraer nombre para stdlib
                let fn_name = self.callee_name(callee, line)?;

                // HOF (map, filter, etc.) necesitan acceso a la VM para los callbacks
                if stdlib::HOF_NAMES.contains(&fn_name.as_str()) {
                    return self.eval_hof(&fn_name, arg_values, line);
                }

                if let Some(result) = stdlib::call(&fn_name, arg_values, line)? {
                    return Ok(result);
                }
                Err(CaperError::UndefinedVariable { name: fn_name, line })
            }
        }
    }

    /// Resuelve la expresión callee a un Value si es una variable o campo.
    /// Retorna `None` cuando el callee es un nombre simple que debe buscarse
    /// en la stdlib (evita evaluar el identificador como variable primero).
    fn eval_callee(&mut self, callee: &Expr) -> Result<Option<Value>, CaperError> {
        match callee {
            Expr::Ident(name) => Ok(self.scope.get(name).cloned()),
            Expr::Field { object, field, .. } => {
                let obj = self.eval_expr(object)?;
                // Si el objeto es un objeto real, buscar el campo
                if let Value::Object(ref m) = obj {
                    Ok(Some(m.get(field).cloned().unwrap_or(Value::Null)))
                } else {
                    // ej. `std.math.round` — field es el nombre de la función en stdlib
                    Ok(None)
                }
            }
            _ => {
                let val = self.eval_expr(callee)?;
                Ok(Some(val))
            }
        }
    }

    /// Extrae el nombre de función como string desde una expresión callee (para despacho a stdlib).
    fn callee_name(&self, callee: &Expr, line: usize) -> Result<String, CaperError> {
        match callee {
            Expr::Ident(name) => Ok(name.clone()),
            Expr::Field { field, .. } => Ok(field.clone()),
            _ => Err(CaperError::TypeError {
                line,
                detail: "Callee is not callable".to_string(),
            }),
        }
    }

    /// Ejecuta una función definida por usuario o closure con los argumentos dados.
    fn call_function(&mut self, fd: Rc<FunctionDef>, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        if args.len() != fd.params.len() {
            return Err(CaperError::ArgumentError {
                func: fd.name.clone().unwrap_or_else(|| "<closure>".to_string()),
                detail: format!("expected {} argument(s), got {}", fd.params.len(), args.len()),
            });
        }

        // Intercambiar temporalmente el scope capturado, llamar, luego restaurar
        let saved_scope = std::mem::replace(
            &mut self.scope,
            ScopeStack::new(fd.captured.clone()),
        );

        // Empujar un frame para parámetros y variables locales
        self.scope.push();

        // Inyectar la función bajo su propio nombre para que las llamadas recursivas la resuelvan
        if let Some(ref name) = fd.name {
            self.scope.define(name.clone(), Value::Function(Rc::clone(&fd)), Binding::Immutable, 0).ok();
        }

        for (param, val) in fd.params.iter().zip(args) {
            self.scope.define(param.clone(), val, Binding::Immutable, 0).ok();
        }

        self.call_depth += 1;
        let sig = self.run_stmts(&fd.body);
        self.call_depth -= 1;

        // Restaurar el scope externo
        self.scope = saved_scope;

        match sig? {
            Signal::Emit(v) => Ok(v),
            Signal::None    => Ok(Value::Null),
            Signal::Break | Signal::Continue => Err(CaperError::SyntaxError {
                line, col: 0,
                detail: "break/continue outside loop".to_string(),
            }),
        }
    }

    fn do_index(&self, obj: Value, idx: Value, line: usize) -> Result<Value, CaperError> {
        match (&obj, &idx) {
            (Value::Array(arr), Value::Number(n)) => {
                let i = n.to_i64_saturating();
                let len = arr.len() as i64;
                let actual = if i < 0 { len + i } else { i };
                if actual < 0 || actual >= len {
                    return Err(CaperError::IndexOutOfBounds { index: i, length: arr.len() });
                }
                Ok(arr[actual as usize].clone())
            }
            (Value::Object(map), Value::Str(key)) => {
                Ok(map.get(key).cloned().unwrap_or(Value::Null))
            }
            _ => Err(CaperError::TypeError {
                line,
                detail: format!("Cannot index {} with {}", obj.type_name(), idx.type_name()),
            }),
        }
    }

    fn do_assign(&mut self, target: &AssignTarget, value: Value, line: usize) -> Result<(), CaperError> {
        match target {
            AssignTarget::Ident(name) => {
                self.scope.assign(name, value, line)
            }
            AssignTarget::Field { object, field } => {
                let root = self.resolve_root(object, line)?;
                if !self.scope.is_mutable(&root) {
                    return Err(CaperError::ImmutableReassignment { name: root, line });
                }
                let mut container = self.scope.get(&root).cloned().ok_or_else(|| {
                    CaperError::UndefinedVariable { name: root.clone(), line }
                })?;
                self.set_nested_field(&mut container, object, field, value, &root, line)?;
                self.scope.assign(&root, container, line)
            }
            AssignTarget::Index { object, index } => {
                let root = self.resolve_root(object, line)?;
                if !self.scope.is_mutable(&root) {
                    return Err(CaperError::ImmutableReassignment { name: root, line });
                }
                let idx_val = self.eval_expr(index)?;
                let mut container = self.scope.get(&root).cloned().ok_or_else(|| {
                    CaperError::UndefinedVariable { name: root.clone(), line }
                })?;
                self.set_nested_index(&mut container, object, idx_val, value, &root, line)?;
                self.scope.assign(&root, container, line)
            }
        }
    }

    /// Recorre una cadena de Expr para encontrar el nombre de variable raíz (Ident más a la izquierda).
    fn resolve_root(&self, expr: &Expr, line: usize) -> Result<String, CaperError> {
        match expr {
            Expr::Ident(name) => Ok(name.clone()),
            Expr::Field { object, .. } => self.resolve_root(object, line),
            Expr::Index { object, .. } => self.resolve_root(object, line),
            _ => Err(CaperError::SyntaxError {
                line, col: 0,
                detail: "Invalid assignment target".to_string(),
            }),
        }
    }

    /// Muta `container` siguiendo el camino descrito por `object_expr`,
    /// luego establece `field` al valor `value`.
    fn set_nested_field(
        &mut self,
        container: &mut Value,
        object_expr: &Expr,
        field: &str,
        value: Value,
        root: &str,
        line: usize,
    ) -> Result<(), CaperError> {
        // Navegar hasta el sub-valor correcto siguiendo el camino
        let target = self.navigate_mut(container, object_expr, root, line)?;
        match target {
            Value::Object(map) => {
                map.insert(field.to_string(), value);
                Ok(())
            }
            other => Err(CaperError::TypeError {
                line,
                detail: format!("Cannot set field on {}", other.type_name()),
            }),
        }
    }

    fn set_nested_index(
        &mut self,
        container: &mut Value,
        object_expr: &Expr,
        idx_val: Value,
        value: Value,
        root: &str,
        line: usize,
    ) -> Result<(), CaperError> {
        let target = self.navigate_mut(container, object_expr, root, line)?;
        match (target, &idx_val) {
            (Value::Array(arr), Value::Number(n)) => {
                let i = {
                    use rust_decimal::prelude::ToPrimitive;
                    n.to_i64().unwrap_or(i64::MAX)
                };
                let len = arr.len() as i64;
                let actual = if i < 0 { len + i } else { i };
                if actual < 0 || actual >= len {
                    return Err(CaperError::IndexOutOfBounds { index: i, length: arr.len() });
                }
                arr[actual as usize] = value;
                Ok(())
            }
            (Value::Object(map), Value::Str(key)) => {
                map.insert(key.clone(), value);
                Ok(())
            }
            (other, _) => Err(CaperError::TypeError {
                line,
                detail: format!("Cannot index-assign into {}", other.type_name()),
            }),
        }
    }

    /// Navega dentro de `container` siguiendo el camino en `expr`, deteniéndose
    /// justo antes del último acceso de campo/índice. Retorna una referencia mutable
    /// al sub-valor que debe ser mutado por el llamador.
    fn navigate_mut<'a>(
        &mut self,
        container: &'a mut Value,
        expr: &Expr,
        root: &str,
        line: usize,
    ) -> Result<&'a mut Value, CaperError> {
        match expr {
            Expr::Ident(name) if name == root => Ok(container),
            Expr::Field { object, field, .. } => {
                let parent = self.navigate_mut(container, object, root, line)?;
                match parent {
                    Value::Object(map) => {
                        map.entry(field.clone())
                            .or_insert(Value::Null);
                        Ok(map.get_mut(field.as_str()).unwrap())
                    }
                    other => Err(CaperError::TypeError {
                        line,
                        detail: format!("Cannot access field on {}", other.type_name()),
                    }),
                }
            }
            Expr::Index { object, index, .. } => {
                let idx_val = self.eval_expr(index)?;
                let parent = self.navigate_mut(container, object, root, line)?;
                match (parent, &idx_val) {
                    (Value::Array(arr), Value::Number(n)) => {
                        use rust_decimal::prelude::ToPrimitive;
                        let i = n.to_i64().unwrap_or(i64::MAX);
                        let len = arr.len() as i64;
                        let actual = if i < 0 { len + i } else { i };
                        if actual < 0 || actual >= len {
                            return Err(CaperError::IndexOutOfBounds { index: i, length: arr.len() });
                        }
                        Ok(&mut arr[actual as usize])
                    }
                    (Value::Object(map), Value::Str(key)) => {
                        map.entry(key.clone()).or_insert(Value::Null);
                        Ok(map.get_mut(key.as_str()).unwrap())
                    }
                    (other, _) => Err(CaperError::TypeError {
                        line,
                        detail: format!("Cannot index into {}", other.type_name()),
                    }),
                }
            }
            _ => Err(CaperError::SyntaxError {
                line, col: 0,
                detail: "Invalid assignment path".to_string(),
            }),
        }
    }

    // --- Funciones de orden superior ---
    // Necesitan acceso a call_function, por lo que viven en la VM y no en stdlib.

    fn eval_hof(&mut self, name: &str, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        match name {
            "map"     => self.hof_map(args, line),
            "filter"  => self.hof_filter(args, line),
            "reduce"  => self.hof_reduce(args, line),
            "find"    => self.hof_find(args, line),
            "any"     => self.hof_any(args, line),
            "all"       => self.hof_all(args, line),
            "sort_by"   => self.hof_sort_by(args, line),
            "find_all"  => self.hof_filter(args, line),   // alias
            "find_index"=> self.hof_find_index(args, line),
            "group_by"  => self.hof_group_by(args, line),
            "partition" => self.hof_partition(args, line),
            _ => Err(CaperError::UndefinedVariable { name: name.to_string(), line }),
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
        let mut result = Vec::with_capacity(arr.len());
        for item in arr {
            result.push(self.call_function(Rc::clone(&fd), vec![item], line)?);
        }
        Ok(Value::Array(result))
    }

    fn hof_filter(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "filter")?;
        let fd  = Self::hof_require_fn(&args, 1, "filter")?;
        let mut result = vec![];
        for item in arr {
            let keep = self.call_function(Rc::clone(&fd), vec![item.clone()], line)?;
            if keep.is_truthy() { result.push(item); }
        }
        Ok(Value::Array(result))
    }

    fn hof_reduce(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr  = Self::hof_require_arr(&args, 0, "reduce")?;
        let fd   = Self::hof_require_fn(&args, 1, "reduce")?;
        let init = args.get(2).cloned().ok_or_else(|| CaperError::ArgumentError {
            func: "reduce".to_string(),
            detail: "argument 3 (initial value) is required".to_string(),
        })?;
        let mut acc = init;
        for item in arr {
            acc = self.call_function(Rc::clone(&fd), vec![acc, item], line)?;
        }
        Ok(acc)
    }

    fn hof_find(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "find")?;
        let fd  = Self::hof_require_fn(&args, 1, "find")?;
        for item in arr {
            let matches = self.call_function(Rc::clone(&fd), vec![item.clone()], line)?;
            if matches.is_truthy() { return Ok(item); }
        }
        Ok(Value::Null)
    }

    fn hof_any(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "any")?;
        let fd  = Self::hof_require_fn(&args, 1, "any")?;
        for item in arr {
            let matches = self.call_function(Rc::clone(&fd), vec![item], line)?;
            if matches.is_truthy() { return Ok(Value::Bool(true)); }
        }
        Ok(Value::Bool(false))
    }

    fn hof_all(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "all")?;
        let fd  = Self::hof_require_fn(&args, 1, "all")?;
        for item in arr {
            let matches = self.call_function(Rc::clone(&fd), vec![item], line)?;
            if !matches.is_truthy() { return Ok(Value::Bool(false)); }
        }
        Ok(Value::Bool(true))
    }

    fn hof_sort_by(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let mut arr = Self::hof_require_arr(&args, 0, "sort_by")?;
        match args.get(1) {
            // sort_by(arr, "campo") — ordenar objetos por nombre de campo
            Some(Value::Str(key)) => {
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
                Ok(Value::Array(arr))
            }
            // sort_by(arr, fn) — ordenar con comparador (a, b) => número
            Some(Value::Function(_)) => {
                let fd = Self::hof_require_fn(&args, 1, "sort_by")?;
                let mut err: Option<CaperError> = None;
                // Recolectar (índice, valor) para evitar problemas de borrow mutable
                let mut indexed: Vec<(usize, Value)> = arr.into_iter().enumerate().collect();
                indexed.sort_by(|(_, a), (_, b)| {
                    if err.is_some() { return std::cmp::Ordering::Equal; }
                    match self.call_function(Rc::clone(&fd), vec![a.clone(), b.clone()], line) {
                        Ok(v) => match v.as_decimal() {
                            Some(n) if n < rust_decimal::Decimal::ZERO => std::cmp::Ordering::Less,
                            Some(n) if n > rust_decimal::Decimal::ZERO => std::cmp::Ordering::Greater,
                            _ => std::cmp::Ordering::Equal,
                        },
                        Err(e) => { err = Some(e); std::cmp::Ordering::Equal }
                    }
                });
                if let Some(e) = err { return Err(e); }
                Ok(Value::Array(indexed.into_iter().map(|(_, v)| v).collect()))
            }
            Some(other) => Err(CaperError::ArgumentError {
                func: "sort_by".to_string(),
                detail: format!("argument 2 must be a string or function, got {}", other.type_name()),
            }),
            None => Err(CaperError::ArgumentError {
                func: "sort_by".to_string(),
                detail: "argument 2 is required".to_string(),
            }),
        }
    }

    fn hof_find_index(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "find_index")?;
        let fd  = Self::hof_require_fn(&args, 1, "find_index")?;
        for (i, item) in arr.into_iter().enumerate() {
            let result = self.call_function(Rc::clone(&fd), vec![item], line)?;
            if result.is_truthy() {
                return Ok(Value::Number(rust_decimal::Decimal::from(i as i64)));
            }
        }
        Ok(Value::Number(rust_decimal::Decimal::from(-1i64)))
    }

    fn hof_group_by(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "group_by")?;
        let fd  = Self::hof_require_fn(&args, 1, "group_by")?;
        let mut groups: IndexMap<String, Vec<Value>> = IndexMap::new();
        for item in arr {
            let key = self.call_function(Rc::clone(&fd), vec![item.clone()], line)?;
            let k = match key {
                Value::Str(s)    => s,
                Value::Number(n) => n.to_string(),
                Value::Bool(b)   => b.to_string(),
                _                => "null".to_string(),
            };
            groups.entry(k).or_default().push(item);
        }
        let result: IndexMap<String, Value> = groups
            .into_iter()
            .map(|(k, v)| (k, Value::Array(v)))
            .collect();
        Ok(Value::Object(result))
    }

    fn hof_partition(&mut self, args: Vec<Value>, line: usize) -> Result<Value, CaperError> {
        let arr = Self::hof_require_arr(&args, 0, "partition")?;
        let fd  = Self::hof_require_fn(&args, 1, "partition")?;
        let mut yes = vec![];
        let mut no  = vec![];
        for item in arr {
            let result = self.call_function(Rc::clone(&fd), vec![item.clone()], line)?;
            if result.is_truthy() { yes.push(item); } else { no.push(item); }
        }
        Ok(Value::Array(vec![Value::Array(yes), Value::Array(no)]))
    }

    fn match_arm(
        &mut self,
        value: &Value,
        pattern: &MatchPattern,
        guard: &Option<Box<Expr>>,
        _line: usize,
    ) -> Result<bool, CaperError> {
        let pattern_match = match pattern {
            MatchPattern::Wildcard => true,
            MatchPattern::Literal(expr) => {
                let lit = self.eval_expr(expr)?;
                *value == lit
            }
            MatchPattern::Binding(name) => {
                // Vincular el valor al nombre — visible en el guard y el cuerpo
                self.scope.define(name.clone(), value.clone(), Binding::Immutable, 0).ok();
                true
            }
        };

        if !pattern_match { return Ok(false); }

        if let Some(guard_expr) = guard {
            Ok(self.eval_expr(guard_expr)?.is_truthy())
        } else {
            Ok(true)
        }
    }
}

// --- Helpers ---

/// Construye el objeto `e` vinculado en bloques `catch e { }`.
/// Para UserThrow, expone directamente el código y mensaje del usuario.
/// Para errores internos, expone el código de error estático y el mensaje formateado.
fn error_to_value(e: &CaperError) -> Value {
    let (code, message) = match e {
        CaperError::UserThrow { code, message, .. } => {
            (code.clone(), message.clone())
        }
        other => (other.code().to_string(), other.to_string()),
    };
    let mut m = IndexMap::new();
    m.insert("code".into(), Value::Str(code));
    m.insert("message".into(), Value::Str(message));
    Value::Object(m)
}

fn numeric(v: &Value, op: &BinOp, line: usize) -> Result<Decimal, CaperError> {
    v.as_decimal().ok_or_else(|| CaperError::TypeError {
        line,
        detail: format!("Operator {:?} requires a number, got {}", op, v.type_name()),
    })
}

fn cmp_values(a: &Value, b: &Value, line: usize) -> Result<i32, CaperError> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Ok(x.cmp(y) as i32),
        (Value::Str(x), Value::Str(y)) => Ok(x.cmp(y) as i32),
        _ => Err(CaperError::TypeError {
            line,
            detail: format!("Cannot compare {} and {}", a.type_name(), b.type_name()),
        }),
    }
}

trait ToI64Saturating { fn to_i64_saturating(&self) -> i64; }
impl ToI64Saturating for Decimal {
    fn to_i64_saturating(&self) -> i64 {
        use rust_decimal::prelude::ToPrimitive;
        self.to_i64().unwrap_or(i64::MAX)
    }
}