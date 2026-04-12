// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Compilador AST → bytecode para AlcaparraLang.
//!
//! Recorre el AST y emite `Instruction`s en `Chunk`s.
//! `chunks[0]` es siempre el cuerpo del script de nivel raíz.
//! Los cuerpos de funciones/closures obtienen su propio chunk (añadido a `program.chunks`).

use crate::error::CaperError;
use crate::instruction::{Chunk, CompiledProgram, Instruction};
use crate::parser::ast::*;
use crate::vm::value::Value;

// ---------------------------------------------------------------------------
// Punto de entrada público
// ---------------------------------------------------------------------------

pub fn compile(program: Program) -> Result<CompiledProgram, CaperError> {
    let mut c = Compiler::new();
    c.compile_program(program)?;
    Ok(c.program)
}

// ---------------------------------------------------------------------------
// Contexto de bucle — para backpatching de break/continue
// ---------------------------------------------------------------------------

struct LoopContext {
    /// Direcciones de los marcadores `Jump(0)` emitidos para `break`.
    breaks: Vec<usize>,
    /// Direcciones de los marcadores `Jump(0)` emitidos para `continue`.
    continues: Vec<usize>,
}

impl LoopContext {
    fn new() -> Self {
        Self { breaks: vec![], continues: vec![] }
    }
}

// ---------------------------------------------------------------------------
// Compilador
// ---------------------------------------------------------------------------

struct Compiler {
    program:     CompiledProgram,
    chunk_idx:   usize,          // índice del chunk en compilación
    loop_stack:  Vec<LoopContext>,
}

impl Compiler {
    fn new() -> Self {
        // Pre-asignar chunk 0 para el cuerpo de nivel raíz
        let main_chunk = Chunk::new(None, vec![]);
        Self {
            program: CompiledProgram { chunks: vec![main_chunk] },
            chunk_idx: 0,
            loop_stack: vec![],
        }
    }

    // -----------------------------------------------------------------------
    // Ayudantes de chunk
    // -----------------------------------------------------------------------

    fn emit(&mut self, instr: Instruction) -> usize {
        self.program.chunks[self.chunk_idx].emit(instr)
    }

    fn patch(&mut self, addr: usize, instr: Instruction) {
        self.program.chunks[self.chunk_idx].patch(addr, instr);
    }

    fn here(&self) -> usize {
        self.program.chunks[self.chunk_idx].here()
    }

    /// Crea un nuevo chunk (para el cuerpo de una función/closure).
    /// Retorna el índice del nuevo chunk.
    fn new_chunk(&mut self, name: Option<String>, params: Vec<String>) -> usize {
        let idx = self.program.chunks.len();
        self.program.chunks.push(Chunk::new(name, params));
        idx
    }

    // -----------------------------------------------------------------------
    // Programa
    // -----------------------------------------------------------------------

    fn compile_program(&mut self, program: Program) -> Result<(), CaperError> {
        // Las constantes del header se emiten al inicio del chunk 0
        if let Some(header) = program.header {
            self.emit(Instruction::Enter);
            for c in &header.constants {
                self.compile_expr(&c.value)?;
                self.emit(Instruction::ConstBind(c.name.clone()));
            }
        }

        self.emit(Instruction::Enter); // scope principal
        for stmt in &program.body {
            self.compile_stmt(stmt)?;
        }
        self.emit(Instruction::Leave);

        // Emitir un Emit(Null) centinela para que el ejecutor siempre reciba un valor.
        // Los scripts reales terminan con un Emit explícito que sale antes.
        // Línea 0 marca esto como el centinela generado por el compilador (usado por el linter).
        self.emit(Instruction::Const(Value::Null));
        self.emit(Instruction::Emit(0));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Sentencias
    // -----------------------------------------------------------------------

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CaperError> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                self.compile_expr(value)?;
                self.emit(Instruction::LetBind(name.clone()));
            }

            Stmt::Var { name, value, .. } => {
                self.compile_expr(value)?;
                self.emit(Instruction::VarBind(name.clone()));
            }

            Stmt::Assign { target, value, line } => {
                self.compile_assign(target, value, *line)?;
            }

            Stmt::Emit { value, line } => {
                self.compile_expr(value)?;
                self.emit(Instruction::Emit(*line));
            }

            Stmt::If { condition, then_block, else_if_clauses, else_block, .. } => {
                self.compile_if(condition, then_block, else_if_clauses, else_block)?;
            }

            Stmt::While { condition, body, .. } => {
                self.compile_while(condition, body)?;
            }

            Stmt::Foreach { key, value, iter, body, .. } => {
                self.compile_foreach(key.as_deref(), value, iter, body)?;
            }

            Stmt::Break { .. } => {
                let addr = self.emit(Instruction::Jump(0)); // marcador
                self.loop_stack.last_mut()
                    .expect("break outside loop")
                    .breaks.push(addr);
            }

            Stmt::Continue { .. } => {
                let addr = self.emit(Instruction::Jump(0)); // marcador
                self.loop_stack.last_mut()
                    .expect("continue outside loop")
                    .continues.push(addr);
            }

            Stmt::FnDecl { name, params, body, .. } => {
                let chunk_id = self.new_chunk(Some(name.clone()), params.clone());
                self.compile_fn_body(chunk_id, body)?;
                self.emit(Instruction::Closure {
                    chunk_id,
                    name: Some(name.clone()),
                });
                self.emit(Instruction::LetBind(name.clone()));
            }

            Stmt::TryCatch { try_block, error_var, catch_block, .. } => {
                self.compile_try_catch_stmt(try_block, error_var, catch_block)?;
            }

            Stmt::Use { path, items, .. } => {
                self.emit(Instruction::UseModule { path: path.clone(), items: items.clone() });
            }

            Stmt::Throw { value, line } => {
                self.compile_expr(value)?;
                self.emit(Instruction::Throw(*line));
            }

            Stmt::ExprStmt(expr) => {
                self.compile_expr(expr)?;
                self.emit(Instruction::Pop);
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Asignación
    // -----------------------------------------------------------------------

    fn compile_assign(
        &mut self,
        target: &AssignTarget,
        value: &Expr,
        line: usize,
    ) -> Result<(), CaperError> {
        match target {
            AssignTarget::Ident(name) => {
                self.compile_expr(value)?;
                self.emit(Instruction::Store(name.clone(), line));
            }

            AssignTarget::Field { .. } => {
                // Extraer el camino completo del campo desde la variable raíz
                if let Some((root, fields)) = extract_field_path(target) {
                    // Cargar objeto raíz
                    self.emit(Instruction::Load(root.to_string()));
                    // Para cada campo intermedio: Dup del padre, luego GetField
                    // así tanto el padre como el hijo están en el stack para el write-back
                    for f in &fields[..fields.len() - 1] {
                        self.emit(Instruction::Dup);
                        self.emit(Instruction::GetField(f.to_string(), line));
                    }
                    // Compilar nuevo valor
                    self.compile_expr(value)?;
                    // SetField para cada nivel, de abajo hacia arriba
                    for f in fields.iter().rev() {
                        self.emit(Instruction::SetField(f.to_string(), line));
                    }
                    // Almacenar raíz reconstruida
                    self.emit(Instruction::Store(root.to_string(), line));
                } else if let AssignTarget::Field { object, field } = target {
                    // Raíz no-ident — esfuerzo máximo, sin write-back
                    self.compile_expr(object)?;
                    self.compile_expr(value)?;
                    self.emit(Instruction::SetField(field.clone(), line));
                    self.emit(Instruction::Pop);
                }
            }

            AssignTarget::Index { object, index } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.compile_expr(value)?;
                self.emit(Instruction::SetIndex(line));
                if let Expr::Ident(name) = object.as_ref() {
                    self.emit(Instruction::Store(name.clone(), line));
                } else {
                    self.emit(Instruction::Pop);
                }
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // If / else-if / else (si / si-sino / sino)
    // -----------------------------------------------------------------------

    fn compile_if(
        &mut self,
        condition: &Expr,
        then_block: &[Stmt],
        else_if_clauses: &[(Expr, Vec<Stmt>)],
        else_block: &Option<Vec<Stmt>>,
    ) -> Result<(), CaperError> {
        // Direcciones de Jump(0) al final de cada rama tomada (para saltar el resto)
        let mut end_jumps: Vec<usize> = vec![];

        // Primera condición
        self.compile_expr(condition)?;
        let jf = self.emit(Instruction::JumpFalse(0));

        self.emit(Instruction::Pop); // consume the condition
        self.emit(Instruction::Enter);
        for s in then_block { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);
        end_jumps.push(self.emit(Instruction::Jump(0)));

        let mut next = self.here();
        self.patch(jf, Instruction::JumpFalse(next));

        // Cláusulas else-if
        for (cond, body) in else_if_clauses {
            self.emit(Instruction::Pop);
            self.compile_expr(cond)?;
            let jf2 = self.emit(Instruction::JumpFalse(0));

            self.emit(Instruction::Pop);
            self.emit(Instruction::Enter);
            for s in body { self.compile_stmt(s)?; }
            self.emit(Instruction::Leave);
            end_jumps.push(self.emit(Instruction::Jump(0)));

            next = self.here();
            self.patch(jf2, Instruction::JumpFalse(next));
        }

        // else (sino)
        self.emit(Instruction::Pop); // consumir la última condición
        if let Some(else_stmts) = else_block {
            self.emit(Instruction::Enter);
            for s in else_stmts { self.compile_stmt(s)?; }
            self.emit(Instruction::Leave);
        }

        // Parchear todos los saltos finales hasta aquí
        let end = self.here();
        for addr in end_jumps {
            self.patch(addr, Instruction::Jump(end));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // While (mientras)
    // -----------------------------------------------------------------------

    fn compile_while(&mut self, condition: &Expr, body: &[Stmt]) -> Result<(), CaperError> {
        let loop_start = self.here();

        self.compile_expr(condition)?;
        let jf = self.emit(Instruction::JumpFalse(0));
        self.emit(Instruction::Pop); // consumir la condición verdadera

        self.loop_stack.push(LoopContext::new());
        self.emit(Instruction::Enter);
        for s in body { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);

        // Parchear continues → de vuelta a la evaluación de la condición
        let ctx = self.loop_stack.pop().unwrap();
        for addr in &ctx.continues {
            self.patch(*addr, Instruction::Jump(loop_start));
        }

        self.emit(Instruction::Jump(loop_start));

        // loop_end: JumpFalse aterriza aquí (la condición está en el stack)
        let loop_end = self.here();
        self.patch(jf, Instruction::JumpFalse(loop_end));
        self.emit(Instruction::Pop); // consumir la condición falsa

        // after_pop: break aterriza aquí (la condición ya fue consumida)
        let after_pop = self.here();
        for addr in &ctx.breaks {
            self.patch(*addr, Instruction::Jump(after_pop));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Foreach (para cada)
    // -----------------------------------------------------------------------

    fn compile_foreach(
        &mut self,
        key: Option<&str>,
        val_name: &str,
        iter_expr: &Expr,
        body: &[Stmt],
    ) -> Result<(), CaperError> {
        // Empujar el iterable, inicializar el iterador
        self.compile_expr(iter_expr)?;
        self.emit(Instruction::IterInit);

        let iter_check = self.here();

        self.loop_stack.push(LoopContext::new());
        self.emit(Instruction::Enter);

        // IterNext saltará a `done` cuando el iterador se agote
        let iter_next_addr = self.emit(Instruction::IterNext {
            done: 0, // backpatch
            key: key.map(|s| s.to_string()),
            val: val_name.to_string(),
        });

        for s in body { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);

        // Parchear continues → de vuelta al inicio del bucle (re-verificar iterador)
        let ctx = self.loop_stack.pop().unwrap();
        for addr in &ctx.continues {
            self.patch(*addr, Instruction::Jump(iter_check));
        }

        self.emit(Instruction::Jump(iter_check));

        let loop_end = self.here();
        // Parchear la dirección done de IterNext
        self.patch(iter_next_addr, Instruction::IterNext {
            done: loop_end,
            key: key.map(|s| s.to_string()),
            val: val_name.to_string(),
        });

        self.emit(Instruction::IterDone);

        // Parchear breaks → después de IterDone
        let after = self.here();
        for addr in &ctx.breaks {
            self.patch(*addr, Instruction::Jump(after));
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Sentencia TryCatch
    // -----------------------------------------------------------------------

    fn compile_try_catch_stmt(
        &mut self,
        try_block: &[Stmt],
        error_var: &str,
        catch_block: &[Stmt],
    ) -> Result<(), CaperError> {
        let try_push_addr = self.emit(Instruction::TryPush(0)); // catch_addr placeholder

        self.emit(Instruction::Enter);
        for s in try_block { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);
        self.emit(Instruction::TryPop);

        // Saltar el bloque catch (salida normal)
        let skip_catch = self.emit(Instruction::Jump(0));

        // El bloque catch comienza aquí
        let catch_addr = self.here();
        self.patch(try_push_addr, Instruction::TryPush(catch_addr));

        // Al entrar al catch: el valor de error está en el stack
        self.emit(Instruction::Enter);
        self.emit(Instruction::LetBind(error_var.to_string()));
        for s in catch_block { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);

        let end = self.here();
        self.patch(skip_catch, Instruction::Jump(end));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Compilación del cuerpo de función
    // -----------------------------------------------------------------------

    fn compile_fn_body(&mut self, chunk_id: usize, body: &[Stmt]) -> Result<(), CaperError> {
        let saved = self.chunk_idx;
        self.chunk_idx = chunk_id;

        self.emit(Instruction::Enter);
        for stmt in body {
            self.compile_stmt(stmt)?;
        }
        self.emit(Instruction::Leave);
        // Retorno null implícito si no hay emit
        self.emit(Instruction::Const(Value::Null));
        self.emit(Instruction::Ret);

        self.chunk_idx = saved;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Expresiones
    // -----------------------------------------------------------------------

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), CaperError> {
        match expr {
            Expr::Number(d)    => { self.emit(Instruction::Const(Value::Number(*d))); }
            Expr::StringLit(s) => { self.emit(Instruction::Const(Value::Str(s.clone()))); }
            Expr::Bool(b)      => { self.emit(Instruction::Const(Value::Bool(*b))); }
            Expr::Null         => { self.emit(Instruction::Const(Value::Null)); }
            Expr::Ident(name)  => { self.emit(Instruction::Load(name.clone())); }

            Expr::BinOp { op, left, right, line } => {
                self.compile_binop(op, left, right, *line)?;
            }

            Expr::UnaryOp { op, operand, .. } => {
                self.compile_expr(operand)?;
                match op {
                    UnaryOp::Neg => { self.emit(Instruction::Neg); }
                    UnaryOp::Not => { self.emit(Instruction::Not); }
                }
            }

            Expr::NullCoalesce { left, right, .. } => {
                self.compile_expr(left)?;
                // JumpNotNull NO hace pop; si no es null, saltear el lado derecho
                let jnn = self.emit(Instruction::JumpNotNull(0));
                self.emit(Instruction::Pop);
                self.compile_expr(right)?;
                let end = self.here();
                self.patch(jnn, Instruction::JumpNotNull(end));
            }

            Expr::Array(elements) => {
                self.compile_array(elements)?;
            }

            Expr::Object(fields) => {
                self.compile_object(fields)?;
            }

            Expr::Field { object, field, line } => {
                self.compile_expr(object)?;
                self.emit(Instruction::GetField(field.clone(), *line));
            }

            Expr::Index { object, index, line } => {
                self.compile_expr(object)?;
                self.compile_expr(index)?;
                self.emit(Instruction::GetIndex(*line));
            }

            Expr::Range { start, end, line } => {
                // Compilar como array literal [start..end]
                self.compile_range(start, end, *line)?;
            }

            Expr::Call { callee, args, line } => {
                self.compile_call(callee, args, *line)?;
            }

            Expr::Closure { params, body, .. } => {
                self.compile_closure(params, body)?;
            }

            Expr::IfExpr { condition, then_block, else_block, line } => {
                self.compile_if_expr(condition, then_block, else_block, *line)?;
            }

            Expr::Match { subject, arms, line } => {
                self.compile_match(subject, arms, *line)?;
            }

            Expr::TryCatchExpr { try_block, error_var, catch_block, .. } => {
                self.compile_try_catch_expr(try_block, error_var, catch_block)?;
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // BinOp — con cortocircuito para And/Or
    // -----------------------------------------------------------------------

    fn compile_binop(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
        line: usize,
    ) -> Result<(), CaperError> {
        match op {
            BinOp::And => {
                // left && right: si left es falso, cortocircuito a false
                self.compile_expr(left)?;
                let jf = self.emit(Instruction::JumpFalse(0));
                self.emit(Instruction::Pop);
                self.compile_expr(right)?;
                let end = self.here();
                self.patch(jf, Instruction::JumpFalse(end));
            }
            BinOp::Or => {
                // left || right: si left es verdadero, cortocircuito
                self.compile_expr(left)?;
                let jt = self.emit(Instruction::JumpTrue(0));
                self.emit(Instruction::Pop);
                self.compile_expr(right)?;
                let end = self.here();
                self.patch(jt, Instruction::JumpTrue(end));
            }
            _ => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                self.emit(Instruction::BinOp(op.clone(), line));
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Array (con spread)
    // -----------------------------------------------------------------------

    fn compile_array(&mut self, elements: &[ArrayElement]) -> Result<(), CaperError> {
        let has_spread = elements.iter().any(|e| matches!(e, ArrayElement::Spread(_)));

        if !has_spread {
            // Caso simple: empujar N valores, MakeArray(N)
            let n = elements.len();
            for el in elements {
                if let ArrayElement::Expr(e) = el {
                    self.compile_expr(e)?;
                }
            }
            self.emit(Instruction::MakeArray(n));
        } else {
            // Construir incrementalmente: empujar cada lote como MakeArray, luego ArraySpread para extender
            let mut batch: Vec<&Expr> = vec![];
            let mut first = true;

            for el in elements {
                match el {
                    ArrayElement::Expr(e) => {
                        batch.push(e);
                    }
                    ArrayElement::Spread(e) => {
                        // Vaciar lote
                        let n = batch.len();
                        for b in &batch { self.compile_expr(b)?; }
                        batch.clear();
                        if first {
                            self.emit(Instruction::MakeArray(n));
                            first = false;
                        } else {
                            self.emit(Instruction::MakeArray(n));
                            // En este punto: TOS=nuevo_lote, debajo=acumulado
                            // El ejecutor maneja la fusión
                        }
                        self.compile_expr(e)?;
                        self.emit(Instruction::ArraySpread);
                    }
                }
            }
            // Vaciar lote final
            let n = batch.len();
            for b in &batch { self.compile_expr(b)?; }
            if first {
                self.emit(Instruction::MakeArray(n));
            } else {
                self.emit(Instruction::MakeArray(n));
                // fusionar lote final con el acumulado
                self.emit(Instruction::ArraySpread);
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Objeto (con spread)
    // -----------------------------------------------------------------------

    fn compile_object(&mut self, fields: &[ObjectField]) -> Result<(), CaperError> {
        self.emit(Instruction::ObjStart);
        for field in fields {
            match field {
                ObjectField::Named(key, val) => {
                    self.compile_expr(val)?;
                    self.emit(Instruction::ObjSet(key.clone()));
                }
                ObjectField::Spread(e) => {
                    self.compile_expr(e)?;
                    self.emit(Instruction::ObjSpread);
                }
            }
        }
        self.emit(Instruction::ObjEnd);
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Range — expandido en tiempo de ejecución via instrucción especial
    // -----------------------------------------------------------------------

    fn compile_range(&mut self, start: &Expr, end: &Expr, line: usize) -> Result<(), CaperError> {
        // Emitir los extremos del rango y luego la instrucción Range dedicada.
        // Instrucción Instruction::Range(line) — ambos operandos están en el stack.
        self.compile_expr(start)?;
        self.compile_expr(end)?;
        self.emit(Instruction::Range(line));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Llamada de función
    // -----------------------------------------------------------------------

    fn compile_call(&mut self, callee: &Expr, args: &[Expr], line: usize) -> Result<(), CaperError> {
        let n = args.len();
        match callee {
            // Llamada directa por nombre: map(arr, fn) / len(s) / fn_usuario(x)
            Expr::Ident(name) => {
                for arg in args { self.compile_expr(arg)?; }
                self.emit(Instruction::CallNamed(name.clone(), n, line));
            }
            // Llamada de campo: std.math.round(x, 0) o obj.campo_fn(x)
            Expr::Field { object, field, .. } => {
                self.compile_expr(object)?;
                for arg in args { self.compile_expr(arg)?; }
                self.emit(Instruction::CallMethod(field.clone(), n, line));
            }
            // Llamada de valor: get_fn()(args) o arr[0](args)
            _ => {
                self.compile_expr(callee)?;
                for arg in args { self.compile_expr(arg)?; }
                self.emit(Instruction::Call(n, line));
            }
        }
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Closure
    // -----------------------------------------------------------------------

    fn compile_closure(&mut self, params: &[String], body: &ClosureBody) -> Result<(), CaperError> {
        let chunk_id = self.new_chunk(None, params.to_vec());
        let saved = self.chunk_idx;
        self.chunk_idx = chunk_id;

        self.emit(Instruction::Enter);
        match body {
            ClosureBody::Expr(e) => {
                self.compile_expr(e)?;
                self.emit(Instruction::Ret);
            }
            ClosureBody::Block(stmts) => {
                for s in stmts { self.compile_stmt(s)?; }
                self.emit(Instruction::Const(Value::Null));
                self.emit(Instruction::Ret);
            }
        }
        self.emit(Instruction::Leave);

        self.chunk_idx = saved;
        self.emit(Instruction::Closure { chunk_id, name: None });
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Expresión if
    // -----------------------------------------------------------------------

    fn compile_if_expr(
        &mut self,
        condition: &Expr,
        then_block: &[Stmt],
        else_block: &[Stmt],
        _line: usize,
    ) -> Result<(), CaperError> {
        self.compile_expr(condition)?;
        let jf = self.emit(Instruction::JumpFalse(0));
        self.emit(Instruction::Pop);

        // rama then — debe dejar un valor en el stack via Emit en el bloque
        self.emit(Instruction::Enter);
        for s in then_block { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);
        let skip_else = self.emit(Instruction::Jump(0));

        let else_addr = self.here();
        self.patch(jf, Instruction::JumpFalse(else_addr));
        self.emit(Instruction::Pop);

        // rama else
        self.emit(Instruction::Enter);
        for s in else_block { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);

        let end = self.here();
        self.patch(skip_else, Instruction::Jump(end));
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Match
    // -----------------------------------------------------------------------

    fn compile_match(&mut self, subject: &Expr, arms: &[MatchArm], _line: usize) -> Result<(), CaperError> {
        // El subject se evalúa una vez; se hace Dup antes de cada test de brazo
        self.compile_expr(subject)?; // TOS = subject

        let mut end_jumps: Vec<usize> = vec![];

        for arm in arms {
            // Dup del subject para el test de este brazo
            self.emit(Instruction::Dup);

            match &arm.pattern {
                MatchPattern::Wildcard => {
                    // Siempre coincide — hacer pop del subject duplicado, verificar guard
                    self.emit(Instruction::Pop);
                    if let Some(guard) = &arm.guard {
                        self.compile_expr(guard)?;
                        let jf = self.emit(Instruction::JumpFalse(0));
                        self.emit(Instruction::Pop);
                        self.compile_match_arm_body(&arm.body)?;
                        end_jumps.push(self.emit(Instruction::Jump(0)));
                        let next = self.here();
                        self.patch(jf, Instruction::JumpFalse(next));
                        self.emit(Instruction::Pop);
                    } else {
                        self.compile_match_arm_body(&arm.body)?;
                        // No se necesita end_jump — pero se añade uno para normalizar
                        end_jumps.push(self.emit(Instruction::Jump(0)));
                    }
                }

                MatchPattern::Binding(name) => {
                    // Vincular subject duplicado al nombre (siempre coincide)
                    self.emit(Instruction::Enter);
                    self.emit(Instruction::LetBind(name.clone()));
                    if let Some(guard) = &arm.guard {
                        self.compile_expr(guard)?;
                        let jf = self.emit(Instruction::JumpFalse(0));
                        self.emit(Instruction::Pop);
                        self.compile_match_arm_body(&arm.body)?;
                        self.emit(Instruction::Leave);
                        end_jumps.push(self.emit(Instruction::Jump(0)));
                        let next = self.here();
                        self.patch(jf, Instruction::JumpFalse(next));
                        self.emit(Instruction::Pop);
                        self.emit(Instruction::Leave);
                    } else {
                        self.compile_match_arm_body(&arm.body)?;
                        self.emit(Instruction::Leave);
                        end_jumps.push(self.emit(Instruction::Jump(0)));
                    }
                }

                MatchPattern::Literal(lit) => {
                    // Stack antes: [..., subject, dup_subject]
                    // Comparar dup con el literal
                    self.compile_expr(lit)?;
                    self.emit(Instruction::BinOp(BinOp::Eq, 0));
                    // Stack: [..., subject, bool]
                    let jf = self.emit(Instruction::JumpFalse(0)); // saltar si no coincide

                    // --- Ruta de coincidencia ---
                    self.emit(Instruction::Pop); // pop del bool true
                    // Stack: [..., subject]
                    if let Some(guard) = &arm.guard {
                        self.compile_expr(guard)?;
                        let jf2 = self.emit(Instruction::JumpFalse(0));
                        self.emit(Instruction::Pop);
                        self.compile_match_arm_body(&arm.body)?;
                        end_jumps.push(self.emit(Instruction::Jump(0)));
                        // Guard fallido: pop del bool de guard, continuar con el siguiente brazo
                        let guard_fail = self.here();
                        self.patch(jf2, Instruction::JumpFalse(guard_fail));
                        self.emit(Instruction::Pop); // pop del bool false del guard
                        // Saltar sobre el Pop de fallo literal (subject sigue en el stack)
                        let skip_lit_pop = self.emit(Instruction::Jump(0));
                        // --- Ruta de fallo literal ---
                        let lit_fail = self.here();
                        self.patch(jf, Instruction::JumpFalse(lit_fail));
                        self.emit(Instruction::Pop); // pop del bool false — subject sigue en el stack
                        let after = self.here();
                        self.patch(skip_lit_pop, Instruction::Jump(after));
                    } else {
                        self.compile_match_arm_body(&arm.body)?;
                        end_jumps.push(self.emit(Instruction::Jump(0)));
                        // --- Ruta de fallo ---
                        let skip = self.here();
                        self.patch(jf, Instruction::JumpFalse(skip));
                        self.emit(Instruction::Pop); // pop del bool false — subject permanece
                    }
                }
            }
        }

        // Por defecto: pop del subject, empujar Null
        self.emit(Instruction::Pop);
        self.emit(Instruction::Const(Value::Null));

        let end = self.here();
        for addr in end_jumps {
            self.patch(addr, Instruction::Jump(end));
        }
        Ok(())
    }

    fn compile_match_arm_body(&mut self, body: &Expr) -> Result<(), CaperError> {
        // Pop del subject que sigue en el stack antes de evaluar el cuerpo
        self.emit(Instruction::Pop);
        self.compile_expr(body)?;
        Ok(())
    }

    // -----------------------------------------------------------------------
    // Expresión TryCatch
    // -----------------------------------------------------------------------

    fn compile_try_catch_expr(
        &mut self,
        try_block: &[Stmt],
        error_var: &str,
        catch_block: &[Stmt],
    ) -> Result<(), CaperError> {
        // Misma estructura que la variante de sentencia, pero necesitamos un resultado de valor.
        // La instrucción Emit dentro de expresiones if/match empuja un valor.
        let try_push_addr = self.emit(Instruction::TryPush(0));

        self.emit(Instruction::Enter);
        for s in try_block { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);
        self.emit(Instruction::TryPop);

        let skip_catch = self.emit(Instruction::Jump(0));

        let catch_addr = self.here();
        self.patch(try_push_addr, Instruction::TryPush(catch_addr));

        self.emit(Instruction::Enter);
        self.emit(Instruction::LetBind(error_var.to_string()));
        for s in catch_block { self.compile_stmt(s)?; }
        self.emit(Instruction::Leave);

        let end = self.here();
        self.patch(skip_catch, Instruction::Jump(end));
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper: extrae (nombre_variable_raíz, [campo1, campo2, ...]) de un target de
// asignación de campos anidados como `a.b.c`.
// Retorna None si la raíz no es un Ident simple.
// ---------------------------------------------------------------------------

fn extract_field_path<'a>(target: &'a AssignTarget) -> Option<(&'a str, Vec<&'a str>)> {
    let mut fields: Vec<&str> = vec![];
    match target {
        AssignTarget::Field { object, field } => {
            fields.push(field.as_str());
            let mut cur: &Expr = object.as_ref();
            loop {
                match cur {
                    Expr::Ident(name) => {
                        fields.reverse();
                        return Some((name.as_str(), fields));
                    }
                    Expr::Field { object: inner, field: f, .. } => {
                        fields.push(f.as_str());
                        cur = inner.as_ref();
                    }
                    _ => return None,
                }
            }
        }
        _ => None,
    }
}
