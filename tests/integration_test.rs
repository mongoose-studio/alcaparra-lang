// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use alcaparra::{Interpreter, Value};
use indexmap::IndexMap;
use rust_decimal_macros::dec;
use std::collections::HashMap;
use std::path::PathBuf;

fn run(source: &str, ctx: HashMap<String, Value>) -> Value {
    let mut interp = Interpreter::new();
    interp.run("test", source, ctx).expect("script failed")
}

fn run_ok(source: &str, ctx: HashMap<String, Value>) -> Result<Value, alcaparra::CaperError> {
    let mut interp = Interpreter::new();
    interp.run("test", source, ctx)
}

fn run_with_base(source: &str, ctx: HashMap<String, Value>, base: PathBuf) -> Value {
    let mut interp = Interpreter::with_loader(base, HashMap::new());
    interp.run("test", source, ctx).expect("script failed")
}

fn run_with_base_ok(source: &str, ctx: HashMap<String, Value>, base: PathBuf) -> Result<Value, alcaparra::CaperError> {
    let mut interp = Interpreter::with_loader(base, HashMap::new());
    interp.run("test", source, ctx)
}

fn ctx(pairs: &[(&str, Value)]) -> HashMap<String, Value> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
}

fn obj(pairs: &[(&str, Value)]) -> Value {
    let mut m = IndexMap::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v.clone());
    }
    Value::Object(m)
}

fn num(n: rust_decimal::Decimal) -> Value { Value::Number(n) }
fn s(v: &str) -> Value { Value::Str(v.to_string()) }
fn b(v: bool) -> Value { Value::Bool(v) }

// ---------------------------------------------------------------------------
// Literales y expresiones básicas
// ---------------------------------------------------------------------------

#[test]
fn emit_number_literal() {
    let v = run("emit 42;", ctx(&[]));
    assert_eq!(v, num(dec!(42)));
}

#[test]
fn emit_string_literal() {
    let v = run(r#"emit "hola";"#, ctx(&[]));
    assert_eq!(v, s("hola"));
}

#[test]
fn emit_bool() {
    assert_eq!(run("emit true;",  ctx(&[])), b(true));
    assert_eq!(run("emit false;", ctx(&[])), b(false));
}

#[test]
fn emit_null() {
    assert_eq!(run("emit null;", ctx(&[])), Value::Null);
}

// ---------------------------------------------------------------------------
// Arithmetic
// ---------------------------------------------------------------------------

#[test]
fn arithmetic_basic() {
    assert_eq!(run("emit 2 + 3;",   ctx(&[])), num(dec!(5)));
    assert_eq!(run("emit 10 - 4;",  ctx(&[])), num(dec!(6)));
    assert_eq!(run("emit 3 * 4;",   ctx(&[])), num(dec!(12)));
    assert_eq!(run("emit 10 / 4;",  ctx(&[])), num(dec!(2.5)));
    assert_eq!(run("emit 10 % 3;",  ctx(&[])), num(dec!(1)));
    assert_eq!(run("emit 2 ** 8;",  ctx(&[])), num(dec!(256)));
}

#[test]
fn string_concatenation() {
    let v = run(r#"emit "hola" + " " + "mundo";"#, ctx(&[]));
    assert_eq!(v, s("hola mundo"));
}

#[test]
fn unary_negation() {
    assert_eq!(run("emit -5;", ctx(&[])), num(dec!(-5)));
}

#[test]
fn division_by_zero() {
    let err = run_ok("emit 1 / 0;", ctx(&[]));
    assert!(err.is_err());
    assert!(err.unwrap_err().to_string().contains("zero"));
}

// ---------------------------------------------------------------------------
// let / var / assign
// ---------------------------------------------------------------------------

#[test]
fn let_binding() {
    let v = run("let x = 10; emit x;", ctx(&[]));
    assert_eq!(v, num(dec!(10)));
}

#[test]
fn var_reassign() {
    let v = run("var x = 1; x = 2; emit x;", ctx(&[]));
    assert_eq!(v, num(dec!(2)));
}

#[test]
fn let_immutable_error() {
    let err = run_ok("let x = 1; x = 2; emit x;", ctx(&[]));
    assert!(err.is_err());
}

#[test]
fn let_redeclaration_same_scope_is_error() {
    // Redeclarar una variable con `let` en el mismo scope debe ser error.
    let err = run_ok("main { let x = 1; let x = 2; emit x; }", ctx(&[]));
    assert!(
        err.is_err(),
        "expected redeclaration error, got {:?}", err
    );
    let code = err.unwrap_err().code().to_string();
    assert_eq!(code, "VARIABLE_REDECLARATION");
}

#[test]
fn let_shadowing_nested_scope_is_ok() {
    // Shadowing en un scope más interno sí está permitido.
    let v = run(
        "let x = 1; if true { let x = 99; emit x; } emit x;",
        ctx(&[]),
    );
    // El if con emit interno retorna 99.
    assert_eq!(v, num(dec!(99)));
}

// ---------------------------------------------------------------------------
// Context injection
// ---------------------------------------------------------------------------

#[test]
fn context_variable() {
    let v = run("emit SUELDO;", ctx(&[("SUELDO", num(dec!(850000)))]));
    assert_eq!(v, num(dec!(850000)));
}

#[test]
fn null_coalesce_undefined() {
    // Variable not in context → null → fallback
    let v = run("emit X ?? 99;", ctx(&[]));
    assert_eq!(v, num(dec!(99)));
}

#[test]
fn null_coalesce_defined() {
    let v = run("emit X ?? 99;", ctx(&[("X", num(dec!(5)))]));
    assert_eq!(v, num(dec!(5)));
}

// ---------------------------------------------------------------------------
// if / else
// ---------------------------------------------------------------------------

#[test]
fn if_true_branch() {
    let v = run("var r = 0; if 1 > 0 { r = 1; } emit r;", ctx(&[]));
    assert_eq!(v, num(dec!(1)));
}

#[test]
fn if_else_branch() {
    let v = run("var r = 0; if 1 > 2 { r = 1; } else { r = 2; } emit r;", ctx(&[]));
    assert_eq!(v, num(dec!(2)));
}

#[test]
fn if_else_if_chain() {
    let src = r#"
        let n = 5;
        var cat = "none";
        if n < 3 { cat = "low"; }
        else if n < 7 { cat = "mid"; }
        else { cat = "high"; }
        emit cat;
    "#;
    assert_eq!(run(src, ctx(&[])), s("mid"));
}

// ---------------------------------------------------------------------------
// Comparaciones y operadores lógicos
// ---------------------------------------------------------------------------

#[test]
fn comparisons() {
    assert_eq!(run("emit 1 < 2;",  ctx(&[])), b(true));
    assert_eq!(run("emit 2 > 1;",  ctx(&[])), b(true));
    assert_eq!(run("emit 1 == 1;", ctx(&[])), b(true));
    assert_eq!(run("emit 1 != 2;", ctx(&[])), b(true));
    assert_eq!(run("emit 1 >= 1;", ctx(&[])), b(true));
    assert_eq!(run("emit 1 <= 0;", ctx(&[])), b(false));
}

#[test]
fn logical_and_or() {
    assert_eq!(run("emit true and false;", ctx(&[])), b(false));
    assert_eq!(run("emit true or false;",  ctx(&[])), b(true));
    assert_eq!(run("emit not true;",       ctx(&[])), b(false));
}

// ---------------------------------------------------------------------------
// while loop
// ---------------------------------------------------------------------------

#[test]
fn while_loop() {
    let src = "var i = 0; var s = 0; while (i < 5) { s = s + i; i = i + 1; } emit s;";
    assert_eq!(run(src, ctx(&[])), num(dec!(10)));
}

#[test]
fn while_break() {
    let src = "var i = 0; while (i < 100) { if i == 3 { break; } i = i + 1; } emit i;";
    assert_eq!(run(src, ctx(&[])), num(dec!(3)));
}

#[test]
fn while_continue() {
    let src = r#"
        var i = 0; var s = 0;
        while (i < 5) {
            i = i + 1;
            if i == 3 { continue; }
            s = s + i;
        }
        emit s;
    "#;
    // sum of 1+2+4+5 = 12 (skip 3)
    assert_eq!(run(src, ctx(&[])), num(dec!(12)));
}

// ---------------------------------------------------------------------------
// foreach
// ---------------------------------------------------------------------------

#[test]
fn foreach_array() {
    let src = "var s = 0; foreach item in [1, 2, 3, 4] { s = s + item; } emit s;";
    assert_eq!(run(src, ctx(&[])), num(dec!(10)));
}

#[test]
fn foreach_array_with_index() {
    let src = r#"
        var last_idx = 0;
        foreach idx, val in [10, 20, 30] { last_idx = idx; }
        emit last_idx;
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(2)));
}

#[test]
fn foreach_object() {
    let src = r#"
        var count = 0;
        foreach val in { a: 1, b: 2, c: 3 } { count = count + val; }
        emit count;
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(6)));
}

// ---------------------------------------------------------------------------
// Arrays
// ---------------------------------------------------------------------------

#[test]
fn array_index() {
    assert_eq!(run("let a = [10, 20, 30]; emit a[1];", ctx(&[])), num(dec!(20)));
}

#[test]
fn array_negative_index() {
    assert_eq!(run("let a = [10, 20, 30]; emit a[-1];", ctx(&[])), num(dec!(30)));
}

#[test]
fn array_spread() {
    let src = "let a = [1, 2]; let b = [...a, 3]; emit b[2];";
    assert_eq!(run(src, ctx(&[])), num(dec!(3)));
}

#[test]
fn array_out_of_bounds() {
    assert!(run_ok("let a = [1]; emit a[5];", ctx(&[])).is_err());
}

// ---------------------------------------------------------------------------
// Objects
// ---------------------------------------------------------------------------

#[test]
fn object_field_access() {
    let src = r#"let o = { x: 1, y: 2 }; emit o.x;"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(1)));
}

#[test]
fn object_index_access() {
    let src = r#"let o = { x: 42 }; emit o["x"];"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(42)));
}

#[test]
fn nested_object() {
    let src = r#"let o = { inner: { val: 7 } }; emit o.inner.val;"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(7)));
}

// ---------------------------------------------------------------------------
// match
// ---------------------------------------------------------------------------

#[test]
fn match_literal() {
    let src = r#"
        let x = 2;
        let r = match x { 1 => "one", 2 => "two", _ => "other" };
        emit r;
    "#;
    assert_eq!(run(src, ctx(&[])), s("two"));
}

#[test]
fn match_wildcard() {
    let src = r#"let r = match 99 { 1 => "one", _ => "other" }; emit r;"#;
    assert_eq!(run(src, ctx(&[])), s("other"));
}

#[test]
fn match_binding_with_guard() {
    let src = r#"
        let monto = 1_200_000;
        let tramo = match monto {
            n if n <= 800_000   => "exento",
            n if n <= 1_000_000 => "bajo",
            n if n <= 2_000_000 => "medio-alto",
            _                   => "alto"
        };
        emit tramo;
    "#;
    assert_eq!(run(src, ctx(&[])), s("medio-alto"));
}

// ---------------------------------------------------------------------------
// try / catch
// ---------------------------------------------------------------------------

#[test]
fn try_catch_catches_error() {
    let src = r#"
        var result = "ok";
        try {
            let x = 1 / 0;
        } catch e {
            result = "caught";
        }
        emit result;
    "#;
    assert_eq!(run(src, ctx(&[])), s("caught"));
}

#[test]
fn try_catch_no_error() {
    let src = r#"
        var result = "ok";
        try { result = "no-error"; } catch e { result = "caught"; }
        emit result;
    "#;
    assert_eq!(run(src, ctx(&[])), s("no-error"));
}

// ---------------------------------------------------------------------------
// stdlib math
// ---------------------------------------------------------------------------

#[test]
fn stdlib_round() {
    let src = "emit round(3.567, 2);";
    assert_eq!(run(src, ctx(&[])), num(dec!(3.57)));
}

#[test]
fn stdlib_abs() {
    assert_eq!(run("emit abs(-5);", ctx(&[])), num(dec!(5)));
}

#[test]
fn stdlib_floor_ceil() {
    assert_eq!(run("emit floor(3.9);", ctx(&[])), num(dec!(3)));
    assert_eq!(run("emit ceil(3.1);",  ctx(&[])), num(dec!(4)));
}

#[test]
fn stdlib_min_max() {
    assert_eq!(run("emit min(3, 7);", ctx(&[])), num(dec!(3)));
    assert_eq!(run("emit max(3, 7);", ctx(&[])), num(dec!(7)));
}

#[test]
fn stdlib_sqrt() {
    assert_eq!(run("emit sqrt(4);", ctx(&[])), num(dec!(2)));
}

// ---------------------------------------------------------------------------
// header { const } — override context
// ---------------------------------------------------------------------------

#[test]
fn header_const_overrides_context() {
    let src = r#"
        header { const TASA = 0.20; }
        main { emit TASA; }
    "#;
    // Incluso si el contexto proporciona TASA = 0.10, la constante tiene prioridad.
    let v = run(src, ctx(&[("TASA", num(dec!(0.10)))]));
    assert_eq!(v, num(dec!(0.20)));
}

#[test]
fn header_const_immutable() {
    let src = r#"
        header { const K = 5; }
        main { K = 10; emit K; }
    "#;
    assert!(run_ok(src, ctx(&[])).is_err());
}

// ---------------------------------------------------------------------------
// Field & index mutation
// ---------------------------------------------------------------------------

#[test]
fn field_mutation() {
    let src = r#"
        var o = { x: 1, y: 2 };
        o.x = 99;
        emit o.x;
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(99)));
}

#[test]
fn index_mutation_array() {
    let src = r#"
        var a = [10, 20, 30];
        a[1] = 99;
        emit a[1];
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(99)));
}

#[test]
fn nested_field_mutation() {
    let src = r#"
        var o = { inner: { val: 1 } };
        o.inner.val = 42;
        emit o.inner.val;
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(42)));
}

#[test]
fn foreach_accumulates_via_mutation() {
    let src = r#"
        var o = { total: 0 };
        foreach item in [1, 2, 3] {
            o.total = o.total + item;
        }
        emit o.total;
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(6)));
}

// ---------------------------------------------------------------------------
// Funciones User-defined
// ---------------------------------------------------------------------------

#[test]
fn fn_basic_call() {
    let src = r#"
        fn double(x) { emit x * 2; }
        emit double(5);
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(10)));
}

#[test]
fn fn_multiple_params() {
    let src = r#"
        fn add(a, b) { emit a + b; }
        emit add(3, 4);
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(7)));
}

#[test]
fn fn_recursion() {
    let src = r#"
        fn fact(n) {
            if n <= 1 { emit 1; }
            else { emit n * fact(n - 1); }
        }
        emit fact(5);
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(120)));
}

#[test]
fn fn_captures_outer_var() {
    let src = r#"
        let base = 100;
        fn add_base(x) { emit x + base; }
        emit add_base(42);
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(142)));
}

#[test]
fn fn_wrong_arity() {
    let src = r#"
        fn f(x) { emit x; }
        emit f(1, 2);
    "#;
    assert!(run_ok(src, ctx(&[])).is_err());
}

// ---------------------------------------------------------------------------
// Closures
// ---------------------------------------------------------------------------

#[test]
fn closure_expr_body() {
    let src = r#"
        let double = |x| => x * 2;
        emit double(7);
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(14)));
}

#[test]
fn closure_block_body() {
    let src = r#"
        let greet = |name| => {
            emit "hello " + name;
        };
        emit greet("world");
    "#;
    assert_eq!(run(src, ctx(&[])), s("hello world"));
}

#[test]
fn closure_captures_enclosing_scope() {
    let src = r#"
        let factor = 3;
        let triple = |x| => x * factor;
        emit triple(5);
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(15)));
}

#[test]
fn closure_passed_as_arg() {
    // Los cierres pasados a las funciones funcionan porque eval_callee resuelve Value::Function.
    let src = r#"
        fn apply(f, x) { emit f(x); }
        let inc = |n| => n + 1;
        emit apply(inc, 9);
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(10)));
}

// ---------------------------------------------------------------------------
// stdlib/strings
// ---------------------------------------------------------------------------

#[test]
fn str_len() {
    assert_eq!(run(r#"emit len("hola");"#, ctx(&[])), num(dec!(4)));
    assert_eq!(run(r#"emit len("");"#,    ctx(&[])), num(dec!(0)));
}

#[test]
fn str_upper_lower() {
    assert_eq!(run(r#"emit upper("hola");"#, ctx(&[])), s("HOLA"));
    assert_eq!(run(r#"emit lower("HOLA");"#, ctx(&[])), s("hola"));
}

#[test]
fn str_trim() {
    assert_eq!(run(r#"emit trim("  hi  ");"#,  ctx(&[])), s("hi"));
    assert_eq!(run(r#"emit ltrim("  hi  ");"#, ctx(&[])), s("hi  "));
    assert_eq!(run(r#"emit rtrim("  hi  ");"#, ctx(&[])), s("  hi"));
}

#[test]
fn str_contains_starts_ends() {
    assert_eq!(run(r#"emit contains("hola mundo", "mundo");"#,  ctx(&[])), b(true));
    assert_eq!(run(r#"emit starts_with("hola", "ho");"#,        ctx(&[])), b(true));
    assert_eq!(run(r#"emit ends_with("hola", "la");"#,          ctx(&[])), b(true));
    assert_eq!(run(r#"emit contains("hola", "xyz");"#,          ctx(&[])), b(false));
}

#[test]
fn str_replace() {
    assert_eq!(run(r#"emit replace("aabbaa", "aa", "X");"#,     ctx(&[])), s("Xbbaa"));
    assert_eq!(run(r#"emit replace_all("aabbaa", "aa", "X");"#, ctx(&[])), s("XbbX"));
}

#[test]
fn str_split_join() {
    let src = r#"let parts = split("a,b,c", ","); emit join(parts, "-");"#;
    assert_eq!(run(src, ctx(&[])), s("a-b-c"));
}

#[test]
fn str_substr() {
    assert_eq!(run(r#"emit substr("hola mundo", 5);"#,    ctx(&[])), s("mundo"));
    assert_eq!(run(r#"emit substr("hola mundo", 0, 4);"#, ctx(&[])), s("hola"));
}

#[test]
fn str_pad() {
    assert_eq!(run(r#"emit pad_left("42", 5, "0");"#,  ctx(&[])), s("00042"));
    assert_eq!(run(r#"emit pad_right("42", 5, "-");"#, ctx(&[])), s("42---"));
}

#[test]
fn str_to_str() {
    assert_eq!(run("emit str(42);",    ctx(&[])), s("42"));
    assert_eq!(run("emit str(true);",  ctx(&[])), s("true"));
    assert_eq!(run("emit str(null);",  ctx(&[])), s("null"));
}

// ---------------------------------------------------------------------------
// stdlib/arrays (non-HOF)
// ---------------------------------------------------------------------------

#[test]
fn arr_count_len() {
    assert_eq!(run("emit count([1, 2, 3]);", ctx(&[])), num(dec!(3)));
    assert_eq!(run("emit len([1, 2]);",      ctx(&[])), num(dec!(2)));
}

#[test]
fn arr_push_unshift() {
    assert_eq!(run("emit push([1,2], 3);",   ctx(&[])), Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))]));
    assert_eq!(run("emit unshift([2,3], 1);",ctx(&[])), Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))]));
}

#[test]
fn arr_pop() {
    let src = "let r = pop([10, 20, 30]); emit r[0];";
    assert_eq!(run(src, ctx(&[])), num(dec!(30)));
}

#[test]
fn arr_first_last() {
    assert_eq!(run("emit first([10,20,30]);", ctx(&[])), num(dec!(10)));
    assert_eq!(run("emit last([10,20,30]);",  ctx(&[])), num(dec!(30)));
}

#[test]
fn arr_slice() {
    assert_eq!(run("emit slice([1,2,3,4,5], 1, 3);", ctx(&[])),
        Value::Array(vec![num(dec!(2)), num(dec!(3))]));
}

#[test]
fn arr_reverse() {
    assert_eq!(run("emit reverse([1,2,3]);", ctx(&[])),
        Value::Array(vec![num(dec!(3)), num(dec!(2)), num(dec!(1))]));
}

#[test]
fn arr_sort() {
    assert_eq!(run("emit sort([3,1,2]);", ctx(&[])),
        Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))]));
}

#[test]
fn arr_unique() {
    assert_eq!(run("emit unique([1,2,1,3,2]);", ctx(&[])),
        Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))]));
}

#[test]
fn arr_flatten() {
    assert_eq!(run("emit flatten([[1,2],[3,[4]]]);", ctx(&[])),
        Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3)), Value::Array(vec![num(dec!(4))])]));
    assert_eq!(run("emit flatten_deep([[1,[2,[3]]],[4]]);", ctx(&[])),
        Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3)), num(dec!(4))]));
}

#[test]
fn arr_contains_index_of() {
    assert_eq!(run("emit contains([1,2,3], 2);",   ctx(&[])), b(true));
    assert_eq!(run("emit contains([1,2,3], 9);",   ctx(&[])), b(false));
    assert_eq!(run("emit index_of([10,20,30], 20);", ctx(&[])), num(dec!(1)));
    assert_eq!(run("emit index_of([10,20,30], 99);", ctx(&[])), num(dec!(-1)));
}

#[test]
fn arr_sum_avg() {
    assert_eq!(run("emit sum([1,2,3,4]);",   ctx(&[])), num(dec!(10)));
    assert_eq!(run("emit avg([1,2,3,4]);",   ctx(&[])), num(dec!(2.5)));
}

#[test]
fn arr_min_max_of() {
    assert_eq!(run("emit min_of([3,1,4,1,5]);", ctx(&[])), num(dec!(1)));
    assert_eq!(run("emit max_of([3,1,4,1,5]);", ctx(&[])), num(dec!(5)));
}

#[test]
fn arr_zip() {
    let src = "emit zip([1,2],[\"a\",\"b\"]);";
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![
        Value::Array(vec![num(dec!(1)), s("a")]),
        Value::Array(vec![num(dec!(2)), s("b")]),
    ]));
}

#[test]
fn arr_range() {
    assert_eq!(run("emit range(0, 4);", ctx(&[])),
        Value::Array(vec![num(dec!(0)), num(dec!(1)), num(dec!(2)), num(dec!(3))]));
    assert_eq!(run("emit range(0, 6, 2);", ctx(&[])),
        Value::Array(vec![num(dec!(0)), num(dec!(2)), num(dec!(4))]));
}

// ---------------------------------------------------------------------------
// stdlib/arrays HOF
// ---------------------------------------------------------------------------

#[test]
fn hof_map() {
    let src = "emit map([1,2,3], |x| => x * 2);";
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(2)), num(dec!(4)), num(dec!(6))]));
}

#[test]
fn hof_filter() {
    let src = "emit filter([1,2,3,4,5], |x| => x > 2);";
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(3)), num(dec!(4)), num(dec!(5))]));
}

#[test]
fn hof_reduce() {
    let src = "emit reduce([1,2,3,4], |acc, x| => acc + x, 0);";
    assert_eq!(run(src, ctx(&[])), num(dec!(10)));
}

#[test]
fn hof_find() {
    let src = "emit find([1,2,3,4], |x| => x > 2);";
    assert_eq!(run(src, ctx(&[])), num(dec!(3)));
    let src2 = "emit find([1,2,3], |x| => x > 9);";
    assert_eq!(run(src2, ctx(&[])), Value::Null);
}

#[test]
fn hof_any_all() {
    assert_eq!(run("emit any([1,2,3], |x| => x > 2);",  ctx(&[])), b(true));
    assert_eq!(run("emit any([1,2,3], |x| => x > 9);",  ctx(&[])), b(false));
    assert_eq!(run("emit all([1,2,3], |x| => x > 0);",  ctx(&[])), b(true));
    assert_eq!(run("emit all([1,2,3], |x| => x > 1);",  ctx(&[])), b(false));
}

#[test]
fn hof_sort_by_field() {
    let src = r#"
        let items = [
            { name: "c", val: 3 },
            { name: "a", val: 1 },
            { name: "b", val: 2 }
        ];
        let sorted = sort_by(items, "val");
        emit sorted[0].name;
    "#;
    assert_eq!(run(src, ctx(&[])), s("a"));
}

#[test]
fn hof_sort_by_fn() {
    let src = "emit sort_by([3,1,2], |a, b| => a - b);";
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))]));
}

#[test]
fn hof_chained() {
    // map + filter pipeline
    let src = r#"
        let nums = [1,2,3,4,5,6];
        let evens_doubled = map(filter(nums, |x| => x % 2 == 0), |x| => x * 2);
        emit sum(evens_doubled);
    "#;
    // evens: [2,4,6] → doubled: [4,8,12] → sum: 24
    assert_eq!(run(src, ctx(&[])), num(dec!(24)));
}

// ---------------------------------------------------------------------------
// stdlib/objects
// ---------------------------------------------------------------------------

#[test]
fn obj_keys_values() {
    let src = r#"let o = { a: 1, b: 2 }; emit keys(o);"#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![s("a"), s("b")]));
    let src2 = r#"let o = { a: 1, b: 2 }; emit values(o);"#;
    assert_eq!(run(src2, ctx(&[])), Value::Array(vec![num(dec!(1)), num(dec!(2))]));
}

#[test]
fn obj_entries() {
    let src = r#"let o = { x: 10 }; emit entries(o);"#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![
        Value::Array(vec![s("x"), num(dec!(10))]),
    ]));
}

#[test]
fn obj_has_get() {
    assert_eq!(run(r#"emit has({ a: 1 }, "a");"#,           ctx(&[])), b(true));
    assert_eq!(run(r#"emit has({ a: 1 }, "b");"#,           ctx(&[])), b(false));
    assert_eq!(run(r#"emit get({ a: 1 }, "a", 0);"#,        ctx(&[])), num(dec!(1)));
    assert_eq!(run(r#"emit get({ a: 1 }, "missing", 99);"#, ctx(&[])), num(dec!(99)));
}

#[test]
fn obj_merge() {
    let src = r#"emit merge({ a: 1, b: 2 }, { b: 99, c: 3 });"#;
    let expected = obj(&[("a", num(dec!(1))), ("b", num(dec!(99))), ("c", num(dec!(3)))]);
    assert_eq!(run(src, ctx(&[])), expected);
}

#[test]
fn obj_omit_pick() {
    let src_omit = r#"emit omit({ a:1, b:2, c:3 }, ["b","c"]);"#;
    assert_eq!(run(src_omit, ctx(&[])), obj(&[("a", num(dec!(1)))]));
    let src_pick = r#"emit pick({ a:1, b:2, c:3 }, ["a","c"]);"#;
    assert_eq!(run(src_pick, ctx(&[])), obj(&[("a", num(dec!(1))), ("c", num(dec!(3)))]));
}

#[test]
fn obj_count() {
    assert_eq!(run(r#"emit count({ a:1, b:2, c:3 });"#, ctx(&[])), num(dec!(3)));
}

// ---------------------------------------------------------------------------
// stdlib/types
// ---------------------------------------------------------------------------

#[test]
fn types_type_of() {
    assert_eq!(run("emit type_of(42);",    ctx(&[])), s("number"));
    assert_eq!(run(r#"emit type_of("s");"#,ctx(&[])), s("string"));
    assert_eq!(run("emit type_of(true);",  ctx(&[])), s("bool"));
    assert_eq!(run("emit type_of(null);",  ctx(&[])), s("null"));
    assert_eq!(run("emit type_of([]);",    ctx(&[])), s("array"));
    assert_eq!(run("emit type_of({});",    ctx(&[])), s("object"));
}

#[test]
fn types_predicates() {
    assert_eq!(run("emit is_null(null);",   ctx(&[])), b(true));
    assert_eq!(run("emit is_null(0);",      ctx(&[])), b(false));
    assert_eq!(run("emit is_number(42);",   ctx(&[])), b(true));
    assert_eq!(run(r#"emit is_string("x");"#, ctx(&[])), b(true));
    assert_eq!(run("emit is_bool(false);",  ctx(&[])), b(true));
    assert_eq!(run("emit is_array([]);",    ctx(&[])), b(true));
    assert_eq!(run("emit is_object({});",   ctx(&[])), b(true));
}

#[test]
fn types_coercions() {
    assert_eq!(run(r#"emit number("42");"#, ctx(&[])), num(dec!(42)));
    assert_eq!(run("emit int(3.9);",        ctx(&[])), num(dec!(3)));
    assert_eq!(run("emit bool(0);",         ctx(&[])), b(false));
    assert_eq!(run("emit bool(1);",         ctx(&[])), b(true));
    assert_eq!(run("emit bool(null);",      ctx(&[])), b(false));
}

// ---------------------------------------------------------------------------
// let inmutabilidad
// ---------------------------------------------------------------------------

#[test]
fn let_object_field_immutable() {
    let src = r#"let o = { x: 1 }; o.x = 2; emit o.x;"#;
    assert!(run_ok(src, ctx(&[])).is_err());
}

#[test]
fn var_object_field_mutable() {
    let src = r#"var o = { x: 1 }; o.x = 2; emit o.x;"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(2)));
}

// ---------------------------------------------------------------------------
// stdlib/json
// ---------------------------------------------------------------------------

#[test]
fn json_encode_decode_roundtrip() {
    let src = r#"
        let original = { nombre: "Juan", monto: 850000, activo: true };
        let encoded  = json_encode(original);
        let decoded  = json_decode(encoded);
        emit decoded.monto;
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(850000)));
}

#[test]
fn json_encode_scalar() {
    assert_eq!(run("emit json_encode(42);",     ctx(&[])), s("42"));
    assert_eq!(run("emit json_encode(true);",   ctx(&[])), s("true"));
    assert_eq!(run("emit json_encode(null);",   ctx(&[])), s("null"));
    assert_eq!(run(r#"emit json_encode("hi");"#, ctx(&[])), s("\"hi\""));
}

#[test]
fn json_decode_array() {
    let src = r#"let arr = json_decode("[1,2,3]"); emit arr[1];"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(2)));
}

#[test]
fn json_valid() {
    assert_eq!(run(r#"emit json_valid("{\"a\":1}");"#, ctx(&[])), b(true));
    assert_eq!(run(r#"emit json_valid("not json");"#,  ctx(&[])), b(false));
}

#[test]
fn json_decode_context() {
    // Patrón común: el valor de contexto complejo llega como un JSON.
    let src = r#"
        let config = json_decode(CONFIG_JSON ?? "{}");
        emit config.tasa ?? 0;
    "#;
    let context = ctx(&[("CONFIG_JSON", s(r#"{"tasa": 0.08}"#))]);
    assert_eq!(run(src, context), num(dec!(0.08)));
}

// ---------------------------------------------------------------------------
// use imports — validación
// ---------------------------------------------------------------------------

#[test]
fn use_known_module_ok() {
    let src = "use std.math.round; emit round(3.5, 0);";
    assert_eq!(run(src, ctx(&[])), num(dec!(4)));
}

#[test]
fn use_unknown_module_errors() {
    let src = "use std.nonexistent; emit 1;";
    assert!(run_ok(src, ctx(&[])).is_err());
}

#[test]
fn fixture_hello() {
    let source = include_str!("fixtures/hello.caper");
    let result = run(source, ctx(&[]));
    let expected = obj(&[
        ("mensaje", s("hola AlcaparraLang")),
        ("version", num(dec!(1))),
    ]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// Fixture: liquidacion_basica.caper
// ---------------------------------------------------------------------------

#[test]
fn fixture_liquidacion_basica() {
    let source = include_str!("fixtures/liquidacion_basica.caper");
    let context = ctx(&[
        ("SUELDO_BASE", num(dec!(850000))),
        ("AFP_TASA",    num(dec!(0.1157))),
        ("FONASA_TASA", num(dec!(0.07))),
    ]);
    let result = run(source, context);

    let expected = obj(&[
        ("sueldo_bruto",   num(dec!(850000))),
        ("desc_afp",       num(dec!(98345))),
        ("desc_salud",     num(dec!(59500))),
        ("desc_cesantia",  num(dec!(5100))),
        ("sueldo_liquido", num(dec!(687055))),
    ]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// Fixture: match_guards.caper
// ---------------------------------------------------------------------------

#[test]
fn fixture_match_guards() {
    let source = include_str!("fixtures/match_guards.caper");
    let context = ctx(&[("MONTO", num(dec!(1200000)))]);
    let result = run(source, context);

    let expected = obj(&[
        ("tramo", s("medio-alto")),
        ("tasa",  num(dec!(0.08))),
    ]);
    assert_eq!(result, expected);
}

// ---------------------------------------------------------------------------
// std.dates
// ---------------------------------------------------------------------------

#[test]
fn dates_today_is_string() {
    let v = run(r#"
        use std.dates.{ today };
        let d = today();
        emit len(d);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(10))); // "YYYY-MM-DD"
}

#[test]
fn dates_year_month_day() {
    let v = run(r#"
        use std.dates.{ year, month, day };
        emit { y: year("2024-03-15"), m: month("2024-03-15"), d: day("2024-03-15") };
    "#, ctx(&[]));
    assert_eq!(v, obj(&[
        ("y", num(dec!(2024))),
        ("m", num(dec!(3))),
        ("d", num(dec!(15))),
    ]));
}

#[test]
fn dates_diff_days() {
    let v = run(r#"
        use std.dates.{ date_diff };
        emit date_diff("2024-01-01", "2024-01-31");
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(30)));
}

#[test]
fn dates_diff_months() {
    let v = run(r#"
        use std.dates.{ date_diff };
        emit date_diff("2024-01-01", "2024-06-01", "month");
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(5)));
}

#[test]
fn dates_diff_years() {
    let v = run(r#"
        use std.dates.{ date_diff };
        emit date_diff("2020-03-15", "2024-03-15", "year");
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(4)));
}

#[test]
fn dates_add_days() {
    let v = run(r#"
        use std.dates.{ date_add };
        emit date_add("2024-01-28", 5, "day");
    "#, ctx(&[]));
    assert_eq!(v, s("2024-02-02"));
}

#[test]
fn dates_add_months() {
    let v = run(r#"
        use std.dates.{ date_add };
        emit date_add("2024-01-15", 3, "month");
    "#, ctx(&[]));
    assert_eq!(v, s("2024-04-15"));
}

#[test]
fn dates_add_years() {
    let v = run(r#"
        use std.dates.{ date_add };
        emit date_add("2022-06-30", 2, "year");
    "#, ctx(&[]));
    assert_eq!(v, s("2024-06-30"));
}

#[test]
fn dates_sub_days() {
    let v = run(r#"
        use std.dates.{ date_sub };
        emit date_sub("2024-03-01", 1, "day");
    "#, ctx(&[]));
    assert_eq!(v, s("2024-02-29")); // 2024 is leap
}

#[test]
fn dates_add_month_clamps_end_of_month() {
    // Jan 31 + 1 month = Feb 28 (or 29 in leap year)
    let v = run(r#"
        use std.dates.{ date_add };
        emit date_add("2023-01-31", 1, "month");
    "#, ctx(&[]));
    assert_eq!(v, s("2023-02-28"));
}

#[test]
fn dates_format() {
    let v = run(r#"
        use std.dates.{ date_format };
        emit date_format("2024-09-18", "DD/MM/YYYY");
    "#, ctx(&[]));
    assert_eq!(v, s("18/09/2024"));
}

#[test]
fn dates_parse() {
    let v = run(r#"
        use std.dates.{ date_parse };
        emit date_parse("18/09/2024", "DD/MM/YYYY");
    "#, ctx(&[]));
    assert_eq!(v, s("2024-09-18"));
}

#[test]
fn dates_is_valid_date() {
    let v = run(r#"
        use std.dates.{ is_valid_date };
        emit is_valid_date("2024-02-29");
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn dates_is_valid_date_false() {
    let v = run(r#"
        use std.dates.{ is_valid_date };
        emit is_valid_date("2023-02-29");
    "#, ctx(&[]));
    assert_eq!(v, b(false));
}

#[test]
fn dates_working_days_january_2024() {
    // Jan 2024: 31 days. Weekends: 6,7,13,14,20,21,27,28 = 8 days → 23 weekdays.
    // Holiday on weekday: Jan 1 (Mon = Año Nuevo) → 22 working days.
    let v = run(r#"
        use std.dates.{ working_days };
        emit working_days(2024, 1);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(22)));
}

#[test]
fn dates_working_days_september_2024() {
    // Sep 2024: 30 days. Sep 1 = Sunday.
    // Weekends: 1,7,8,14,15,21,22,28,29 = 9 days → 21 weekdays.
    // Holidays on weekdays: Sep 18 (Wed), Sep 19 (Thu) → 19 working days.
    let v = run(r#"
        use std.dates.{ working_days };
        emit working_days(2024, 9);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(19)));
}

// ---------------------------------------------------------------------------
// std.time
// ---------------------------------------------------------------------------

#[test]
fn time_timestamp_positive() {
    let v = run(r#"
        use std.time.{ timestamp };
        emit timestamp() > 0;
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn time_from_timestamp_roundtrip() {
    let v = run(r#"
        use std.time.{ to_timestamp, from_timestamp };
        let ts = to_timestamp("2024-09-18");
        emit from_timestamp(ts);
    "#, ctx(&[]));
    assert_eq!(v, s("2024-09-18"));
}

#[test]
fn time_time_now_format() {
    // HH:MM:SS = 8 chars
    let v = run(r#"
        use std.time.{ time_now };
        emit len(time_now());
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(8)));
}

#[test]
fn time_elapsed_ms_non_negative() {
    let v = run(r#"
        use std.time.{ elapsed_ms };
        emit elapsed_ms() >= 0;
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

// ---------------------------------------------------------------------------
// std.crypto
// ---------------------------------------------------------------------------

#[test]
fn crypto_md5_known() {
    let v = run(r#"
        use std.crypto.{ md5 };
        emit md5("hello");
    "#, ctx(&[]));
    assert_eq!(v, s("5d41402abc4b2a76b9719d911017c592"));
}

#[test]
fn crypto_sha256_known() {
    let v = run(r#"
        use std.crypto.{ sha256 };
        emit sha256("abc");
    "#, ctx(&[]));
    assert_eq!(v, s("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"));
}

#[test]
fn crypto_base64_roundtrip() {
    let v = run(r#"
        use std.crypto.{ base64_encode, base64_decode };
        let enc = base64_encode("hola mundo");
        emit base64_decode(enc);
    "#, ctx(&[]));
    assert_eq!(v, s("hola mundo"));
}

#[test]
fn crypto_hex_roundtrip() {
    let v = run(r#"
        use std.crypto.{ hex_encode, hex_decode };
        let enc = hex_encode("caper");
        emit hex_decode(enc);
    "#, ctx(&[]));
    assert_eq!(v, s("caper"));
}

#[test]
fn crypto_hmac_sha256_known() {
    let v = run(r#"
        use std.crypto.{ hmac_sha256 };
        emit len(hmac_sha256("message", "secret")) == 64;
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

// ---------------------------------------------------------------------------
// std.sort
// ---------------------------------------------------------------------------

#[test]
fn sort_desc_numbers() {
    let v = run(r#"
        use std.sort.{ sort_desc };
        emit sort_desc([3, 1, 4, 1, 5, 9]);
    "#, ctx(&[]));
    assert_eq!(v, Value::Array(vec![
        num(dec!(9)), num(dec!(5)), num(dec!(4)),
        num(dec!(3)), num(dec!(1)), num(dec!(1)),
    ]));
}

#[test]
fn sort_by_desc_field() {
    let v = run(r#"
        use std.sort.{ sort_by_desc };
        let arr = [{ n: "b", v: 2 }, { n: "a", v: 3 }, { n: "c", v: 1 }];
        let sorted = sort_by_desc(arr, "v");
        emit map(sorted, |x| => x.n);
    "#, ctx(&[]));
    assert_eq!(v, Value::Array(vec![s("a"), s("b"), s("c")]));
}

#[test]
fn sort_order_by_multi() {
    let v = run(r#"
        use std.sort.{ order_by };
        let arr = [
            { dep: "TI",   sueldo: 900000 },
            { dep: "RRHH", sueldo: 800000 },
            { dep: "TI",   sueldo: 700000 }
        ];
        let r = order_by(arr, [["dep", "asc"], ["sueldo", "desc"]]);
        emit map(r, |x| => x.sueldo);
    "#, ctx(&[]));
    assert_eq!(v, Value::Array(vec![num(dec!(800000)), num(dec!(900000)), num(dec!(700000))]));
}

#[test]
fn sort_group_by_key_fn() {
    let v = run(r#"
        use std.sort.{ group_by_key };
        let arr = [
            { dep: "TI",   nombre: "Ana" },
            { dep: "RRHH", nombre: "Bob" },
            { dep: "TI",   nombre: "Carlos" }
        ];
        let g = group_by_key(arr, "dep");
        emit count(g.TI);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(2)));
}

#[test]
fn sort_chunk() {
    let v = run(r#"
        use std.sort.{ chunk };
        let arr = [1, 2, 3, 4, 5];
        let chunks = chunk(arr, 2);
        emit count(chunks);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(3))); // [1,2], [3,4], [5]
}

#[test]
fn sort_group_by_hof() {
    let v = run(r#"
        use std.sort;
        let arr = [1, 2, 3, 4, 5, 6];
        let g = group_by(arr, |x| => if x % 2 == 0 { emit "par"; } else { emit "impar"; });
        emit count(g.par);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(3)));
}

#[test]
fn sort_partition_hof() {
    let v = run(r#"
        let arr = [1, 2, 3, 4, 5];
        let parts = partition(arr, |x| => x > 3);
        emit { mayores: parts[0], menores: parts[1] };
    "#, ctx(&[]));
    assert_eq!(v, obj(&[
        ("mayores", Value::Array(vec![num(dec!(4)), num(dec!(5))])),
        ("menores", Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))])),
    ]));
}

// ---------------------------------------------------------------------------
// std.search
// ---------------------------------------------------------------------------

#[test]
fn search_binary_found() {
    let v = run(r#"
        use std.search.{ binary_search };
        emit binary_search([1, 3, 5, 7, 9], 5);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(2)));
}

#[test]
fn search_binary_not_found() {
    let v = run(r#"
        use std.search.{ binary_search };
        emit binary_search([1, 3, 5, 7, 9], 4);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(-1)));
}

#[test]
fn search_linear() {
    let v = run(r#"
        use std.search.{ search };
        emit search(["a", "b", "c"], "b");
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(1)));
}

#[test]
fn search_in_range() {
    let v = run(r#"
        use std.search.{ in_range };
        emit in_range(500000, 400000, 600000);
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn search_find_all_hof() {
    let v = run(r#"
        let arr = [1, 2, 3, 4, 5, 6];
        let pares = find_all(arr, |x| => x % 2 == 0);
        emit count(pares);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(3)));
}

#[test]
fn search_find_index_hof() {
    let v = run(r#"
        let arr = [10, 20, 30, 40];
        emit find_index(arr, |x| => x > 25);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(2)));
}

#[test]
fn search_fuzzy_match() {
    let v = run(r#"
        use std.search.{ fuzzy };
        emit fuzzy("Mongoose", "Mongose", 0.7);
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn search_fuzzy_no_match() {
    let v = run(r#"
        use std.search.{ fuzzy };
        emit fuzzy("Mongoose", "Dog", 0.9);
    "#, ctx(&[]));
    assert_eq!(v, b(false));
}

// ---------------------------------------------------------------------------
// std.xml
// ---------------------------------------------------------------------------

#[test]
fn xml_encode_object() {
    let v = run(r#"
        use std.xml.{ xml_encode };
        let doc = { nombre: "Juan", monto: 850000 };
        emit xml_encode(doc, "Liquidacion");
    "#, ctx(&[]));
    let xml = match v { Value::Str(s) => s, _ => panic!("expected string") };
    assert!(xml.contains("<Liquidacion>"));
    assert!(xml.contains("<nombre>Juan</nombre>"));
    assert!(xml.contains("<monto>850000</monto>"));
    assert!(xml.contains("</Liquidacion>"));
}

#[test]
fn xml_encode_opts_declaration_namespace() {
    let v = run(r#"
        use std.xml.{ xml_encode_opts };
        let doc = { RUT: "12345678-9" };
        emit xml_encode_opts(doc, "Doc", { declaration: true, indent: true, namespace: "http://example.com" });
    "#, ctx(&[]));
    let xml = match v { Value::Str(s) => s, _ => panic!("expected string") };
    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("xmlns=\"http://example.com\""));
    assert!(xml.contains("<RUT>12345678-9</RUT>"));
}

#[test]
fn xml_decode_simple() {
    let v = run(r#"
        use std.xml.{ xml_decode };
        let xml = "<persona><nombre>Ana</nombre><edad>30</edad></persona>";
        let obj = xml_decode(xml);
        emit { n: obj.nombre, e: obj.edad };
    "#, ctx(&[]));
    assert_eq!(v, obj(&[("n", s("Ana")), ("e", s("30"))]));
}

#[test]
fn xml_decode_repeated_elements_become_array() {
    let v = run(r#"
        use std.xml.{ xml_decode };
        let xml = "<items><item>a</item><item>b</item><item>c</item></items>";
        let obj = xml_decode(xml);
        emit count(obj.item);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(3)));
}

#[test]
fn xml_valid_true() {
    let v = run(r#"
        use std.xml.{ xml_valid };
        emit xml_valid("<root><a>1</a></root>");
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn xml_valid_false() {
    let v = run(r#"
        use std.xml.{ xml_valid };
        emit xml_valid("<root><unclosed>");
    "#, ctx(&[]));
    assert_eq!(v, b(false));
}

#[test]
fn xml_get_path() {
    let v = run(r#"
        use std.xml.{ xml_get };
        let xml = "<empresa><empleado><nombre>Carlos</nombre></empleado></empresa>";
        emit xml_get(xml, "empleado/nombre");
    "#, ctx(&[]));
    assert_eq!(v, s("Carlos"));
}

// ---------------------------------------------------------------------------
// std.yaml
// ---------------------------------------------------------------------------

#[test]
fn yaml_encode_object() {
    let v = run(r#"
        use std.yaml.{ yaml_encode };
        let cfg = { servicio: "payroll", version: "2.0" };
        emit yaml_encode(cfg);
    "#, ctx(&[]));
    let yaml = match v { Value::Str(s) => s, _ => panic!("expected string") };
    assert!(yaml.contains("servicio: payroll"));
    assert!(yaml.contains("version: '2.0'") || yaml.contains("version: \"2.0\"") || yaml.contains("version: 2.0"));
}

#[test]
fn yaml_encode_numbers() {
    let v = run(r#"
        use std.yaml.{ yaml_encode };
        emit yaml_encode({ uf: 38000, tasa: 0.1157 });
    "#, ctx(&[]));
    let yaml = match v { Value::Str(s) => s, _ => panic!("expected string") };
    assert!(yaml.contains("uf: 38000"));
    assert!(yaml.contains("tasa:"));
}

#[test]
fn yaml_decode_object() {
    let v = run(r#"
        use std.yaml.{ yaml_decode };
        let yaml = "nombre: Juan\nmonto: 850000\n";
        let obj = yaml_decode(yaml);
        emit { n: obj.nombre, m: obj.monto };
    "#, ctx(&[]));
    assert_eq!(v, obj(&[("n", s("Juan")), ("m", num(dec!(850000)))]));
}

#[test]
fn yaml_decode_array() {
    let v = run(r#"
        use std.yaml.{ yaml_decode };
        let yaml = "- uno\n- dos\n- tres\n";
        let arr = yaml_decode(yaml);
        emit count(arr);
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(3)));
}

#[test]
fn yaml_roundtrip() {
    let v = run(r#"
        use std.yaml.{ yaml_encode, yaml_decode };
        let original = { a: 1, b: "hola", c: true };
        let encoded = yaml_encode(original);
        let decoded = yaml_decode(encoded);
        emit decoded.a;
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(1)));
}

#[test]
fn yaml_valid_true() {
    let v = run(r#"
        use std.yaml.{ yaml_valid };
        emit yaml_valid("clave: valor\nlista:\n  - a\n  - b\n");
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn yaml_valid_false() {
    let v = run(r#"
        use std.yaml.{ yaml_valid };
        emit yaml_valid("{\ninvalid yaml: [");
    "#, ctx(&[]));
    assert_eq!(v, b(false));
}

// ---------------------------------------------------------------------------
// std.regex
// ---------------------------------------------------------------------------

#[test]
fn regex_match_full() {
    let v = run(r#"
        use std.regex.{ regex_match };
        emit regex_match("12345678-9", "^[0-9]{8}-[0-9]$");
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn regex_match_full_false() {
    let v = run(r#"
        use std.regex.{ regex_match };
        emit regex_match("abc-123", "^[0-9]{8}-[0-9]$");
    "#, ctx(&[]));
    assert_eq!(v, b(false));
}

#[test]
fn regex_test_partial() {
    let v = run(r#"
        use std.regex.{ regex_test };
        emit regex_test("hola mundo", "mundo");
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn regex_find_first() {
    let v = run(r#"
        use std.regex.{ regex_find };
        emit regex_find("precio: $1500 y $2000", "\\$[0-9]+");
    "#, ctx(&[]));
    assert_eq!(v, s("$1500"));
}

#[test]
fn regex_find_null_when_no_match() {
    let v = run(r#"
        use std.regex.{ regex_find };
        emit regex_find("hola", "\\d+") ?? "nada";
    "#, ctx(&[]));
    assert_eq!(v, s("nada"));
}

#[test]
fn regex_find_all_matches() {
    let v = run(r#"
        use std.regex.{ regex_find_all };
        emit regex_find_all("a1b2c3", "[0-9]");
    "#, ctx(&[]));
    assert_eq!(v, Value::Array(vec![s("1"), s("2"), s("3")]));
}

#[test]
fn regex_groups_capture() {
    let v = run(r#"
        use std.regex.{ regex_groups };
        let g = regex_groups("2024-09", "^(\\d{4})-(\\d{2})$");
        emit { anio: g[0], mes: g[1] };
    "#, ctx(&[]));
    assert_eq!(v, obj(&[("anio", s("2024")), ("mes", s("09"))]));
}

#[test]
fn regex_replace_first() {
    let v = run(r#"
        use std.regex.{ regex_replace };
        emit regex_replace("hola hola", "hola", "chao");
    "#, ctx(&[]));
    assert_eq!(v, s("chao hola"));
}

#[test]
fn regex_replace_all_matches() {
    let v = run(r#"
        use std.regex.{ regex_replace_all };
        emit regex_replace_all("1.234.567-8", "[.]", "");
    "#, ctx(&[]));
    assert_eq!(v, s("1234567-8"));
}

#[test]
fn regex_split_by_pattern() {
    let v = run(r#"
        use std.regex.{ regex_split };
        emit regex_split("uno,dos,,tres", ",+");
    "#, ctx(&[]));
    assert_eq!(v, Value::Array(vec![s("uno"), s("dos"), s("tres")]));
}

#[test]
fn regex_invalid_pattern_errors() {
    let result = run_ok(r#"
        use std.regex.{ regex_test };
        emit regex_test("x", "[invalid");
    "#, ctx(&[]));
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "INVALID_REGEX");
}

// ---------------------------------------------------------------------------
// std.rand
// ---------------------------------------------------------------------------

#[test]
fn rand_uuid_is_string_36_chars() {
    let v = run(r#"
        use std.rand.{ uuid };
        emit len(uuid());
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(36)));
}

#[test]
fn rand_int_in_range() {
    let v = run(r#"
        use std.rand.{ seed, rand_int };
        seed(42);
        let n = rand_int(1, 10);
        emit n >= 1 and n <= 10;
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

#[test]
fn rand_seeded_is_reproducible() {
    let src = r#"
        use std.rand.{ seed, rand_int };
        seed(99);
        emit rand_int(0, 1000);
    "#;
    let v1 = run(src, ctx(&[]));
    let v2 = run(src, ctx(&[]));
    assert_eq!(v1, v2);
}

#[test]
fn rand_sample_returns_n_elements() {
    let v = run(r#"
        use std.rand.{ rand_sample };
        let arr = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        emit count(rand_sample(arr, 3));
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(3)));
}

#[test]
fn rand_shuffle_preserves_length() {
    let v = run(r#"
        use std.rand.{ shuffle };
        let arr = [1, 2, 3, 4, 5];
        emit count(shuffle(arr));
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(5)));
}

#[test]
fn rand_item_from_array() {
    let v = run(r#"
        use std.rand.{ seed, rand_item };
        seed(7);
        let item = rand_item([10, 20, 30]);
        emit item == 10 or item == 20 or item == 30;
    "#, ctx(&[]));
    assert_eq!(v, b(true));
}

// ---------------------------------------------------------------------------
// Object spread
// ---------------------------------------------------------------------------

#[test]
fn object_spread_basic() {
    let v = run(r#"
        let base = { a: 1, b: 2 };
        let extended = { ...base, c: 3 };
        emit extended;
    "#, ctx(&[]));
    assert_eq!(v, obj(&[("a", num(dec!(1))), ("b", num(dec!(2))), ("c", num(dec!(3)))]));
}

#[test]
fn object_spread_overrides_earlier_key() {
    let v = run(r#"
        let defaults = { color: "rojo", size: "M" };
        let custom = { ...defaults, color: "azul" };
        emit custom;
    "#, ctx(&[]));
    assert_eq!(v, obj(&[("color", s("azul")), ("size", s("M"))]));
}

#[test]
fn object_spread_multiple() {
    let v = run(r#"
        let a = { x: 1 };
        let b = { y: 2 };
        let c = { ...a, ...b, z: 3 };
        emit c;
    "#, ctx(&[]));
    assert_eq!(v, obj(&[("x", num(dec!(1))), ("y", num(dec!(2))), ("z", num(dec!(3)))]));
}

#[test]
fn object_spread_non_object_is_ignored() {
    // spreading a non-object silently does nothing (same as JS)
    let v = run(r#"
        let r = { a: 1, ...null, b: 2 };
        emit r;
    "#, ctx(&[]));
    assert_eq!(v, obj(&[("a", num(dec!(1))), ("b", num(dec!(2)))]));
}

// ---------------------------------------------------------------------------
// throw / try-catch
// ---------------------------------------------------------------------------

#[test]
fn throw_string_uncaught() {
    let result = run_ok(r#"
        throw "algo salió mal";
        emit 1;
    "#, ctx(&[]));
    assert!(result.is_err());
    let e = result.unwrap_err();
    assert_eq!(e.code(), "USER_THROW");
    assert!(e.to_string().contains("algo salió mal"));
}

#[test]
fn throw_object_uncaught() {
    let result = run_ok(r#"
        throw { code: "MI_ERROR", message: "fallo controlado" };
        emit 1;
    "#, ctx(&[]));
    assert!(result.is_err());
    let e = result.unwrap_err();
    assert_eq!(e.code(), "USER_THROW");
    assert!(e.to_string().contains("MI_ERROR"));
    assert!(e.to_string().contains("fallo controlado"));
}

#[test]
fn throw_caught_by_try_catch() {
    let v = run(r#"
        let resultado = try {
            throw "oops";
            emit 99;
        } catch e {
            emit "capturado";
        };
        emit resultado;
    "#, ctx(&[]));
    assert_eq!(v, s("capturado"));
}

#[test]
fn throw_object_caught_has_code_and_message() {
    let v = run(r#"
        let r = try {
            throw { code: "ERR_X", message: "detalle del error" };
            emit "no llega";
        } catch e {
            emit e.code + ":" + e.message;
        };
        emit r;
    "#, ctx(&[]));
    assert_eq!(v, s("ERR_X:detalle del error"));
}

#[test]
fn throw_string_catch_has_message() {
    let v = run(r#"
        let r = try {
            throw "mensaje simple";
        } catch e {
            emit e.message;
        };
        emit r;
    "#, ctx(&[]));
    assert_eq!(v, s("mensaje simple"));
}

#[test]
fn try_without_throw_returns_value() {
    let v = run(r#"
        let r = try {
            emit 42;
        } catch e {
            emit 0;
        };
        emit r;
    "#, ctx(&[]));
    assert_eq!(v, num(dec!(42)));
}

#[test]
fn throw_in_function_caught_outside() {
    let v = run(r#"
        fn puede_fallar(x) {
            if x < 0 {
                throw { code: "NEG", message: "negativo" };
            }
            emit x * 2;
        }

        let r = try {
            emit puede_fallar(-1);
        } catch e {
            emit e.code;
        };
        emit r;
    "#, ctx(&[]));
    assert_eq!(v, s("NEG"));
}

// ---------------------------------------------------------------------------
// Multi-file imports
// ---------------------------------------------------------------------------

#[test]
fn import_function_from_library() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let v = run_with_base(r#"
        use lib.calculos.{ doble };
        emit doble(21);
    "#, ctx(&[]), fixtures);
    assert_eq!(v, num(dec!(42)));
}

#[test]
fn import_multiple_functions_from_same_lib() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let v = run_with_base(r#"
        use lib.calculos.{ suma, porcentaje };
        let total = suma(1000, 500);
        let desc  = porcentaje(total, 0.1);
        emit desc;
    "#, ctx(&[]), fixtures);
    assert_eq!(v, num(dec!(150)));
}

#[test]
fn import_bool_returning_function() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let v = run_with_base(r#"
        use lib.calculos.{ es_positivo };
        emit es_positivo(5);
    "#, ctx(&[]), fixtures);
    assert_eq!(v, b(true));
}

#[test]
fn import_function_throws_and_is_caught() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let v = run_with_base(r#"
        use lib.validaciones.{ validar_positivo };
        let r = try {
            emit validar_positivo(-1, "sueldo");
        } catch e {
            emit e.code + ":" + e.message;
        };
        emit r;
    "#, ctx(&[]), fixtures);
    assert_eq!(v, s("VALIDACION:sueldo debe ser positivo"));
}

#[test]
fn import_function_ok_path() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let v = run_with_base(r#"
        use lib.validaciones.{ validar_rango };
        emit validar_rango(50, 0, 100);
    "#, ctx(&[]), fixtures);
    assert_eq!(v, num(dec!(50)));
}

#[test]
fn import_same_lib_twice_is_ok() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let v = run_with_base(r#"
        use lib.calculos.{ doble };
        use lib.calculos.{ suma };
        emit suma(doble(3), doble(4));
    "#, ctx(&[]), fixtures);
    assert_eq!(v, num(dec!(14)));
}

#[test]
fn import_not_found_returns_error() {
    let fixtures = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let result = run_with_base_ok(r#"
        use lib.no_existe.{ algo };
        emit 1;
    "#, ctx(&[]), fixtures);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().code(), "IMPORT_NOT_FOUND");
}

// ---------------------------------------------------------------------------
// Regresión: `emit` que escapa de un bucle dentro de una función
//
// `do_ret` restauraba el scope y el programa, pero no las pilas de iteradores y
// de objetos. Un `emit` dentro de un `foreach` sale sin ejecutar el `IterDone`
// del bucle, así que su `IterState` quedaba arriba de la pila y el `IterNext`
// de quien llamaba tomaba ese iterador ajeno —ya agotado— y terminaba su propio
// bucle. Sin error: resultado incorrecto en silencio.
// ---------------------------------------------------------------------------

#[test]
fn emit_dentro_de_foreach_no_corta_el_bucle_de_quien_llama() {
    let src = r#"
        fn primero_par(xs) {
            foreach x in xs { if x % 2 == 0 { emit x; } }
            emit 0;
        }
        var a = [];
        foreach lista in [[1, 2], [3, 4], [5, 6]] { a = [...a, primero_par(lista)]; }
        emit a;
    "#;
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![num(dec!(2)), num(dec!(4)), num(dec!(6))]),
    );
}

#[test]
fn emit_dentro_de_foreach_anidado_dos_niveles() {
    let src = r#"
        fn interna(xs) {
            foreach x in xs { if x > 0 { emit x; } }
            emit 0;
        }
        fn externa(xss) {
            var total = 0;
            foreach xs in xss { total = total + interna(xs); }
            emit total;
        }
        var a = [];
        foreach grupo in [[[1], [2]], [[3], [4]]] { a = [...a, externa(grupo)]; }
        emit a;
    "#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(3)), num(dec!(7))]));
}

#[test]
fn emit_dentro_de_foreach_no_corrompe_un_objeto_en_construccion() {
    let src = r#"
        fn primero(xs) {
            foreach x in xs { emit x; }
            emit 0;
        }
        emit { a: primero([1, 2]), b: primero([3, 4]), c: primero([5, 6]) };
    "#;
    assert_eq!(
        run(src, ctx(&[])),
        obj(&[("a", num(dec!(1))), ("b", num(dec!(3))), ("c", num(dec!(5)))]),
    );
}

#[test]
fn emit_dentro_de_foreach_sobre_objeto_tampoco_corta_el_bucle() {
    let src = r#"
        fn primera_clave(mapa) {
            foreach k, v in mapa { emit k; }
            emit "";
        }
        var a = [];
        foreach m in [{ x: 1 }, { y: 2 }, { z: 3 }] { a = [...a, primera_clave(m)]; }
        emit a;
    "#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![s("x"), s("y"), s("z")]));
}

#[test]
fn emit_dentro_de_foreach_desde_un_hof() {
    let src = r#"
        use std.arrays.{ map };
        fn primero(xs) {
            foreach x in xs { emit x; }
            emit 0;
        }
        var a = [];
        foreach lista in [[1, 2], [3, 4]] {
            a = [...a, map([lista], |l| => primero(l))[0]];
        }
        emit a;
    "#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(1)), num(dec!(3))]));
}

#[test]
fn el_contador_de_vueltas_no_se_hereda_entre_bucles() {
    // `IterDone` pone `loop_iters` en cero y un `emit` temprano se lo salta. Si el
    // contador no se restaura al volver, el bucle de afuera hereda las vueltas del
    // de adentro y choca con LOOP_LIMIT antes de tiempo.
    let src = r#"
        fn primero(xs) {
            foreach x in xs { emit x; }
            emit 0;
        }
        var vueltas = 0;
        foreach i in range(1, 2000) { vueltas = vueltas + primero([i]); }
        emit vueltas;
    "#;
    // `range(a, b)` excluye el extremo: 1 + 2 + ... + 1999.
    assert_eq!(run(src, ctx(&[])), num(dec!(1999000)));
}

// ---------------------------------------------------------------------------
// Regresión: declaraciones antes de `main`
//
// La gramática era `main_block | stmt*`, excluyentes: declarar una función
// auxiliar arriba y después abrir `main` fallaba con un error que apuntaba a
// `main`, justo donde no estaba el problema.
// ---------------------------------------------------------------------------

#[test]
fn funciones_declaradas_antes_de_main() {
    let src = r#"
        fn doble(x) { emit x * 2; }
        fn cuadruple(x) { emit doble(doble(x)); }
        main { emit cuadruple(5); }
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(20)));
}

#[test]
fn header_funciones_y_main_conviven() {
    let src = r#"
        header { name: "con_helpers"; const TASA = 2; }
        fn escalar(x) { emit x * TASA; }
        main { emit escalar(BASE); }
    "#;
    assert_eq!(run(src, ctx(&[("BASE", num(dec!(21)))])), num(dec!(42)));
}

#[test]
fn sentencias_antes_de_main_se_ejecutan_antes() {
    let src = r#"
        var contador = 1;
        contador = contador + 1;
        main { emit contador; }
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(2)));
}

// ---------------------------------------------------------------------------
// Regresión: bloques con valor (`if` y `try/catch` como expresión)
//
// El compilador compilaba las ramas como sentencias sueltas, y `ExprStmt` emite
// un `Pop`: la rama no dejaba nada en el stack y la VM moría con «pop on empty
// stack». Ahora la expresión final del bloque es su valor.
// ---------------------------------------------------------------------------

#[test]
fn if_como_expresion_sin_punto_y_coma_final() {
    let src = r#"
        let activo = true;
        let etiqueta = if activo { "activo" } else { "inactivo" };
        emit etiqueta;
    "#;
    assert_eq!(run(src, ctx(&[])), s("activo"));
}

#[test]
fn if_como_expresion_con_punto_y_coma_final() {
    let src = r#"let e = if false { "si"; } else { "no"; }; emit e;"#;
    assert_eq!(run(src, ctx(&[])), s("no"));
}

#[test]
fn if_como_expresion_encadena_else_if() {
    let src = r#"
        fn tramo(m) {
            emit if m >= 20000000 { "alto" } else if m >= 5000000 { "medio" } else { "bajo" };
        }
        emit [tramo(25000000), tramo(7350000), tramo(2000000)];
    "#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![s("alto"), s("medio"), s("bajo")]));
}

#[test]
fn bloque_con_sentencias_antes_de_la_expresion_final() {
    let src = r#"let x = if true { let a = 2; let b = 3; a * b } else { 0 }; emit x;"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(6)));
}

#[test]
fn bloque_sin_expresion_final_vale_null() {
    let src = r#"var y = 0; let x = if true { y = 1; } else { 2 }; emit [x, y];"#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![Value::Null, num(dec!(1))]));
}

#[test]
fn if_como_expresion_dentro_de_un_objeto() {
    let src = r#"let n = 5; emit { par: if n % 2 == 0 { "si" } else { "no" }, doble: n * 2 };"#;
    assert_eq!(run(src, ctx(&[])), obj(&[("par", s("no")), ("doble", num(dec!(10)))]));
}

#[test]
fn try_catch_como_expresion_devuelve_la_rama_catch() {
    let src = r#"let r = try { 1 / 0 } catch e { "cayo: " + e.code }; emit r;"#;
    assert_eq!(run(src, ctx(&[])), s("cayo: DIVISION_BY_ZERO"));
}

#[test]
fn try_catch_como_expresion_devuelve_la_rama_try() {
    let src = r#"let r = try { 6 * 7 } catch e { 0 }; emit r;"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(42)));
}

#[test]
fn el_punto_y_coma_sigue_siendo_obligatorio_entre_sentencias() {
    // La expresión final de un bloque con valor es la única posición donde el
    // `;` se puede omitir. En cualquier otra sigue siendo obligatorio.
    assert!(run_ok("let x = 1 let y = 2; emit x;", ctx(&[])).is_err());
    assert!(run_ok("let x = if true { 1 2 } else { 0 }; emit x;", ctx(&[])).is_err());
}

#[test]
fn if_anidado_como_valor_final_de_un_bloque() {
    // Un `if` en posición de sentencia se parsea como sentencia; en la posición
    // final de un bloque con valor tiene que leerse como expresión.
    let src = r#"
        fn tramo(monto) {
            emit if monto >= 20000000 {
                "alto"
            } else if monto >= 5000000 {
                let base = monto - 5000000;
                if base > 1000000 { "medio-alto" } else { "medio" }
            } else {
                "bajo"
            };
        }
        emit [tramo(25000000), tramo(7350000), tramo(5500000), tramo(2000000)];
    "#;
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![s("alto"), s("medio-alto"), s("medio"), s("bajo")]),
    );
}

#[test]
fn if_sentencia_sin_else_dentro_de_un_bloque_con_valor() {
    // El retroceso no puede romper el `if` sentencia, que no lleva `else`.
    let src = r#"
        let r = if true {
            var n = 0;
            if 1 > 0 { n = 5; }
            n * 2
        } else { 0 };
        emit r;
    "#;
    assert_eq!(run(src, ctx(&[])), num(dec!(10)));
}

#[test]
fn try_como_valor_final_de_un_bloque() {
    let src = r#"let r = if true { try { 1 / 0 } catch e { -1 } } else { 0 }; emit r;"#;
    assert_eq!(run(src, ctx(&[])), num(dec!(-1)));
}

// ---------------------------------------------------------------------------
// Regresión: `round_half_up`
//
// `std.math` la anunciaba en su catálogo y no estaba implementada. `round` usa
// la estrategia por defecto de rust_decimal, que es medio al par, y las
// liquidaciones de sueldo en Chile redondean medio arriba.
// ---------------------------------------------------------------------------

#[test]
fn round_half_up_sube_el_medio() {
    let src = "use std.math.{ round_half_up }; emit [round_half_up(0.5, 0), round_half_up(1.5, 0), round_half_up(2.5, 0), round_half_up(3.5, 0)];";
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3)), num(dec!(4))]),
    );
}

#[test]
fn round_sigue_siendo_medio_al_par() {
    // `round` no cambia: cambiarla seria romper la semantica del lenguaje.
    let src = "use std.math.{ round }; emit [round(0.5, 0), round(1.5, 0), round(2.5, 0), round(3.5, 0)];";
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![num(dec!(0)), num(dec!(2)), num(dec!(2)), num(dec!(4))]),
    );
}

#[test]
fn round_half_up_con_decimales_y_negativos() {
    let src = "use std.math.{ round_half_up }; emit [round_half_up(1.005, 2), round_half_up(-2.5, 0)];";
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(1.01)), num(dec!(-3))]));
}

// ---------------------------------------------------------------------------
// El catálogo de `caper -m` no puede mentir
//
// `MODULES` es una tabla escrita a mano, separada de los `match` que implementan
// cada función, y las dos listas se habían separado: 26 nombres anunciados no
// existían en tiempo de ejecución, y el editor los autocompletaba igual. Esta
// prueba recorre el catálogo y exige que cada nombre sea despachable.
// ---------------------------------------------------------------------------

#[test]
fn todas_las_funciones_del_catalogo_existen() {
    use alcaparra::stdlib::{HOF_NAMES, MODULES};

    // Formas de argumentos con las que se tantea cada nombre. El despacho es
    // polimórfico —`len`, `count`, `min` e `index_of` los reclaman varios
    // módulos según el tipo— así que basta con que UNA forma sea atendida.
    let formas: Vec<Vec<alcaparra::Value>> = vec![
        vec![],
        vec![num(dec!(1))],
        vec![num(dec!(1)), num(dec!(2))],
        vec![num(dec!(1)), num(dec!(2)), num(dec!(3))],
        vec![s("abc")],
        vec![s("abc"), s("b")],
        vec![s("abc"), num(dec!(1))],
        vec![s("abc"), num(dec!(1)), num(dec!(2))],
        vec![Value::Array(vec![num(dec!(1)), num(dec!(2))])],
        vec![Value::Array(vec![num(dec!(1))]), num(dec!(1))],
        vec![Value::Array(vec![num(dec!(1))]), Value::Array(vec![num(dec!(2))])],
        vec![obj(&[("a", num(dec!(1)))])],
        vec![obj(&[("a", num(dec!(1)))]), s("a")],
        vec![obj(&[("a", num(dec!(1)))]), s("a"), num(dec!(2))],
        vec![Value::Array(vec![Value::Array(vec![s("a"), num(dec!(1))])])],
        vec![Value::Bool(true)],
    ];

    let mut faltan: Vec<String> = vec![];

    for modulo in MODULES {
        for funcion in modulo.functions {
            if HOF_NAMES.contains(funcion) {
                continue; // los intercepta la VM, no la stdlib
            }
            let atendida = formas.iter().any(|args| {
                !matches!(
                    alcaparra::stdlib::call(funcion, args.clone(), 1),
                    Ok(None)
                )
            });
            if !atendida {
                faltan.push(format!("{}.{}", modulo.path, funcion));
            }
        }
    }

    assert!(
        faltan.is_empty(),
        "el catálogo de `caper -m` anuncia funciones que ningún módulo despacha:\n  {}",
        faltan.join("\n  "),
    );
}

// ---------------------------------------------------------------------------
// Funciones que el catálogo anunciaba y no existían (§11.4 del contrato ATLAS)
// ---------------------------------------------------------------------------

#[test]
fn math_clamp_acota_el_rango() {
    let src = "use std.math.{ clamp }; emit [clamp(0, 1, 10), clamp(5, 1, 10), clamp(50, 1, 10)];";
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(1)), num(dec!(5)), num(dec!(10))]));
}

#[test]
fn math_clamp_rechaza_un_rango_invertido() {
    let r = run_ok("use std.math.{ clamp }; emit clamp(5, 10, 1);", ctx(&[]));
    assert!(r.is_err());
}

#[test]
fn math_log_y_log10() {
    let src = "use std.math.{ log, log10 }; emit [log(8, 2), log10(100)];";
    match run(src, ctx(&[])) {
        Value::Array(v) => {
            // Son aproximaciones: no hay logaritmo exacto en decimal.
            let tres: rust_decimal::Decimal = dec!(3);
            let dos:  rust_decimal::Decimal = dec!(2);
            let a = match &v[0] { Value::Number(n) => *n, otro => panic!("{otro:?}") };
            let b = match &v[1] { Value::Number(n) => *n, otro => panic!("{otro:?}") };
            assert!((a - tres).abs() < dec!(0.0001), "log(8,2) = {a}");
            assert!((b - dos).abs()  < dec!(0.0001), "log10(100) = {b}");
        }
        otro => panic!("{otro:?}"),
    }
}

#[test]
fn math_log_rechaza_cero_y_base_uno() {
    assert!(run_ok("use std.math.{ log }; emit log(0);", ctx(&[])).is_err());
    assert!(run_ok("use std.math.{ log }; emit log(8, 1);", ctx(&[])).is_err());
}

#[test]
fn math_pi_y_e_son_constantes() {
    let src = "use std.math.{ pi, e }; emit [pi() > 3.14, pi() < 3.15, e() > 2.71, e() < 2.72];";
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![b(true), b(true), b(true), b(true)]),
    );
}

#[test]
fn min_y_max_funcionan_sobre_numeros_y_sobre_arreglos() {
    // `std.math` ve estos nombres antes que `std.arrays`: si no cede cuando el
    // primer argumento no es un número, `min([1, 2])` queda inalcanzable.
    let src = "use std.math.{ min, max }; emit [min(3, 1), max(3, 1), min([3, 1, 2]), max([3, 1, 2])];";
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![num(dec!(1)), num(dec!(3)), num(dec!(1)), num(dec!(3))]),
    );
}

#[test]
fn strings_substring_y_substr_son_funciones_distintas() {
    // `substr` recibe un largo; `substring`, la posición final.
    let src = r#"use std.strings.{ substr, substring }; emit [substr("abcde", 1, 3), substring("abcde", 1, 3)];"#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![s("bcd"), s("bc")]));
}

#[test]
fn strings_substring_acota_y_tolera_rangos_vacios() {
    let src = r#"use std.strings.{ substring }; emit [substring("abc", 1, 99), substring("abc", 2, 1), substring("abc", 1)];"#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![s("bc"), s(""), s("bc")]));
}

#[test]
fn strings_index_of_y_char_at_cuentan_caracteres() {
    // Con tildes los bytes y los caracteres no coinciden: `ñ` ocupa dos bytes.
    let src = r#"use std.strings.{ index_of, char_at }; emit [index_of("mañana", "n"), char_at("mañana", 2), index_of("abc", "z")];"#;
    assert_eq!(run(src, ctx(&[])), Value::Array(vec![num(dec!(4)), s("ñ"), num(dec!(-1))]));
}

#[test]
fn strings_repeat_y_count_occurrences() {
    let src = r#"use std.strings.{ repeat, count_occurrences }; emit [repeat("ab", 3), repeat("a", 0), count_occurrences("abcabc", "bc"), count_occurrences("abc", "")];"#;
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![s("ababab"), s(""), num(dec!(2)), num(dec!(0))]),
    );
}

#[test]
fn strings_alias_del_catalogo() {
    let src = r#"use std.strings.{ trim_start, trim_end, to_string, to_number }; emit [trim_start("  a"), trim_end("a  "), to_string(42), to_number("42")];"#;
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![s("a"), s("a"), s("42"), num(dec!(42))]),
    );
}

#[test]
fn arrays_shift_y_concat_no_mutan() {
    let src = r#"
        use std.arrays.{ shift, concat };
        let base = [1, 2, 3];
        emit [shift(base), concat([1], [2], [3]), shift([]), base];
    "#;
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![
            Value::Array(vec![num(dec!(2)), num(dec!(3))]),
            Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))]),
            Value::Array(vec![]),
            Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))]),
        ]),
    );
}

#[test]
fn arrays_alias_includes_y_flat() {
    let src = "use std.arrays.{ includes, flat }; emit [includes([1,2], 1), includes([1,2], 9), flat([[1],[2]])];";
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![b(true), b(false), Value::Array(vec![num(dec!(1)), num(dec!(2))])]),
    );
}

#[test]
fn objects_set_y_delete_no_mutan() {
    let src = r#"
        use std.objects.{ set, delete, keys };
        let base = { a: 1, b: 2 };
        emit [set(base, "c", 3), delete(base, "a"), delete(base, "zzz"), keys(base)];
    "#;
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![
            obj(&[("a", num(dec!(1))), ("b", num(dec!(2))), ("c", num(dec!(3)))]),
            obj(&[("b", num(dec!(2)))]),
            obj(&[("a", num(dec!(1))), ("b", num(dec!(2)))]),
            Value::Array(vec![s("a"), s("b")]),
        ]),
    );
}

#[test]
fn objects_from_entries_es_la_vuelta_de_entries() {
    let src = r#"
        use std.objects.{ entries, from_entries };
        let original = { a: 1, b: 2 };
        emit from_entries(entries(original));
    "#;
    assert_eq!(run(src, ctx(&[])), obj(&[("a", num(dec!(1))), ("b", num(dec!(2)))]));
}

#[test]
fn types_is_function_y_alias() {
    let src = r#"
        use std.types.{ is_function, to_number, to_bool };
        let f = |x| => x;
        emit [is_function(f), is_function(1), to_number("42"), to_bool(1)];
    "#;
    assert_eq!(
        run(src, ctx(&[])),
        Value::Array(vec![b(true), b(false), num(dec!(42)), b(true)]),
    );
}

#[test]
fn round_admite_decimales_negativos() {
    // Documentado en docs/src/stdlib/math.md y no implementado: fallaba con
    // «argument 2 must be a non-negative integer».
    let src = "use std.math.{ round, round_half_up }; emit [round(1234.567, -2), round_half_up(1235, -1), round(1250, -2)];";
    assert_eq!(
        run(src, ctx(&[])),
        // El último muestra la diferencia de estrategia: 1250 a centenas es 1200
        // medio-al-par y 1300 medio-arriba.
        Value::Array(vec![num(dec!(1200)), num(dec!(1240)), num(dec!(1200))]),
    );
}

// ---------------------------------------------------------------------------
// Regresion de rendimiento: el contexto no se copia en cada llamada
// ---------------------------------------------------------------------------

/// Cada llamada a funcion instalaba el entorno capturado con un `clone()` del
/// mapa completo. Para una funcion de nivel superior ese mapa **es el contexto**,
/// asi que el costo de llamar a una funcion era proporcional al tamano del
/// contexto aunque el script no lo tocara. Una nomina de 100 trabajadores
/// tardaba 14 segundos y una de 500 no terminaba.
///
/// Nada fallaba: solo se arrastraba. Por eso la regresion se mide como una
/// *razon* entre dos contextos y no como un tiempo absoluto, que dependeria de
/// la maquina. Antes del arreglo la razon era de 164x.
#[test]
fn el_contexto_grande_no_encarece_las_llamadas() {
    use std::time::Instant;

    const LLAMADAS: usize = 3000;
    let fuente = r#"
        use std.arrays.{ range };
        header { name: "r"; version: "1.0.0"; }
        fn f(x) { emit x + 1; }
        main { var s = 0; foreach i in range(0, N) { s = s + f(i); } emit s; }
    "#;

    let relleno: Value = Value::Array(
        (0..2000)
            .map(|i| {
                obj(&[
                    ("a", num(rust_decimal::Decimal::from(i))),
                    ("b", s(&"x".repeat(50))),
                    ("c", Value::Array(vec![num(dec!(1)), num(dec!(2)), num(dec!(3))])),
                ])
            })
            .collect(),
    );

    let medir = |con_relleno: bool| -> u128 {
        let mut contexto = HashMap::new();
        contexto.insert("N".to_string(), num(rust_decimal::Decimal::from(LLAMADAS)));
        if con_relleno {
            // Datos que el script no mira jamas.
            contexto.insert("BASURA".to_string(), relleno.clone());
        }
        let t0 = Instant::now();
        let r = run_ok(fuente, contexto);
        assert!(r.is_ok(), "el script debe correr: {:?}", r.err());
        t0.elapsed().as_micros().max(1)
    };

    // Se calienta una vez para no medir el primer acceso a memoria.
    let _ = medir(false);
    let chico  = medir(false);
    let grande = medir(true);

    let razon = grande as f64 / chico as f64;
    assert!(
        razon < 8.0,
        "llamar a una funcion se encarecio {razon:.1}x por un contexto que el \
         script ni siquiera toca ({chico} us sin relleno, {grande} us con \
         relleno). El entorno capturado se esta copiando en cada llamada en vez \
         de compartirse: ver ScopeStack::base."
    );
}

/// La aritmetica no convierte strings. Un contexto JSON trae numeros como
/// string con facilidad —PostgreSQL serializa `numeric` asi— y la conversion
/// silenciosa hacia que `"1850000" * 26 / 30` diera un resultado correcto y la
/// primera suma lo concatenara: el calculo salia mal sin un solo error.
#[test]
fn la_aritmetica_no_convierte_strings() {
    let c = ctx(&[("S", s("1850000")), ("N", num(dec!(100)))]);

    for expr in ["S * 2", "S - N", "S / 2", "S % 7", "S ** 2", "N * S"] {
        let fuente = format!(
            "header {{ name: \"t\"; version: \"1.0.0\"; }} main {{ emit {expr}; }}"
        );
        let r = run_ok(&fuente, c.clone());
        assert!(
            matches!(r, Err(alcaparra::CaperError::TypeError { .. })),
            "`{expr}` debe dar TypeError y no convertir el string en silencio, dio {r:?}"
        );
    }

    // Los booleanos tampoco valen 1 y 0.
    let r = run_ok(
        "header { name: \"t\"; version: \"1.0.0\"; } main { emit B * 2; }",
        ctx(&[("B", b(true))]),
    );
    assert!(matches!(r, Err(alcaparra::CaperError::TypeError { .. })), "true * 2 debe fallar, dio {r:?}");

    // Y las agregaciones de arreglos, que suman plata, tampoco.
    for expr in ["sum(A)", "avg(A)", "min(A)", "max(A)"] {
        let fuente = format!(
            "use std.arrays.{{ sum, avg, min, max }};\n\
             header {{ name: \"t\"; version: \"1.0.0\"; }} main {{ emit {expr}; }}"
        );
        let r = run_ok(&fuente, ctx(&[("A", Value::Array(vec![s("1"), s("2")]))]));
        assert!(r.is_err(), "`{expr}` sobre strings debe fallar, dio {r:?}");
    }

    // `to_number` sigue siendo la via explicita, y esa si convierte.
    let v = run(
        "use std.types.{ to_number };\n\
         header { name: \"t\"; version: \"1.0.0\"; } main { emit to_number(S) * 2; }",
        c.clone(),
    );
    assert_eq!(v, num(dec!(3700000)));
}
