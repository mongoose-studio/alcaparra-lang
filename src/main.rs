// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

use alcaparra::{config, formatter, global_config, package, stdlib, Interpreter, LintDiagnostic, Severity, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    match args[1].as_str() {
        "run"      => cmd_run(&args[2..]),
        "validate" => cmd_validate(&args[2..]),
        "lint"     => cmd_lint(&args[2..]),
        "fmt"      => cmd_fmt(&args[2..]),
        "install"  => cmd_install(&args[2..]),
        "new"      => cmd_new(&args[2..]),
        "repl"     => cmd_repl(),
        "version"  => print_version(),
        "help"     => cmd_help(&args[2..]),
        "-m"       => cmd_modules(),
        "--cfg"    => cmd_cfg(),
        "--help" | "-h" | "-help" => print_usage(),
        "--version" | "-V"        => print_version(),
        other => {
            eprintln!("caper: comando desconocido '{other}'");
            eprintln!("       ejecuta 'caper --help' para ver los comandos disponibles.");
            process::exit(1);
        }
    }
}

fn print_version() {
    println!("caper {} (AlcaparraLang)", env!("CARGO_PKG_VERSION"));
}

fn print_usage() {
    println!("caper {} — intérprete AlcaparraLang\n", env!("CARGO_PKG_VERSION"));
    println!("USO:");
    println!("  caper <comando> [opciones]\n");
    println!("COMANDOS DE EJECUCIÓN:");
    println!("  run [script.caper]   Ejecutar un script o el proyecto actual
                       --context '<json>' | --context-file <ruta.json>");
    println!("  validate <script>    Verificar sintaxis sin ejecutar");
    println!("  lint <script> [...]  Análisis estático (apto para CI)");
    println!("  fmt <script>         Formatear código fuente\n");
    println!("GESTIÓN DE PAQUETES:");
    println!("  new <nombre>         Crear un proyecto nuevo");
    println!("  new --pkg <name>     Crear un paquete publicable (@author/nombre)");
    println!("  install              Instalar dependencias declaradas en .capercfg\n");
    println!("HERRAMIENTAS:");
    println!("  repl                 Iniciar el REPL interactivo");
    println!("  -m                   Listar módulos y funciones de la stdlib");
    println!("  --cfg                Mostrar archivos de configuración cargados\n");
    println!("INFORMACIÓN:");
    println!("  version / --version  Mostrar versión");
    println!("  help [comando]       Mostrar ayuda detallada de un comando");
    println!("  --help / -h          Mostrar esta ayuda\n");
    println!("EJEMPLOS:");
    println!("  caper run liquidacion.caper --context '{{\"SUELDO\": 850000}}'");
    println!("  caper run liquidacion.caper --context-file contexto.json");
    println!("  caper run                   # modo proyecto, lee .capercfg");
    println!("  caper new mi-proyecto");
    println!("  caper new --pkg @mi-usuario/formulas-cl");
    println!("  caper install");
    println!("  caper fmt script.caper --write\n");
    println!("Documentación: https://github.com/mongoose-studio/alcaparra-lang");
}

/// Ayuda detallada por comando: `caper help <comando>`.
fn cmd_help(args: &[String]) {
    match args.first().map(|s| s.as_str()) {
        Some("run") => {
            println!("caper run — ejecutar un script .caper\n");
            println!("USO:");
            println!("  caper run <script.caper> [opciones]");
            println!("  caper run                           # modo proyecto\n");
            println!("OPCIONES:");
            println!("  --context '<json>'       Contexto inline (objeto JSON)");
            println!("  --context-file <archivo> Contexto desde archivo JSON\n");
            println!("MODO PROYECTO:");
            println!("  Sin argumento de script, busca .capercfg en el directorio actual");
            println!("  y usa 'entry' como script y 'context' como variables base.\n");
            println!("EJEMPLOS:");
            println!("  caper run script.caper --context '{{\"SUELDO\": 850000}}'");
            println!("  caper run script.caper --context-file contexto.json");
            println!("  caper run script.caper --context-file ctx.json");
            println!("  caper run   # ejecuta entry definido en .capercfg");
        }
        Some("new") => {
            println!("caper new — crear proyecto o paquete nuevo\n");
            println!("USO:");
            println!("  caper new <nombre>              Nuevo proyecto");
            println!("  caper new --pkg <@author/nombre>  Nuevo paquete publicable\n");
            println!("PROYECTO:");
            println!("  Crea <nombre>/ con .capercfg y main.caper listos para ejecutar.\n");
            println!("  Ejemplo:");
            println!("    caper new mi-liquidacion");
            println!("    cd mi-liquidacion && caper run\n");
            println!("PAQUETE:");
            println!("  Crea <author>/<nombre>/ con pkg.capercfg y lib.caper.");
            println!("  El nombre debe incluir author: @author/nombre o author/nombre.\n");
            println!("  Ejemplo:");
            println!("    caper new --pkg @mongoose/formulas-cl");
            println!("    # Importa con: use @mongoose/formulas-cl.lib.{{ fn }};");
        }
        Some("install") => {
            println!("caper install — instalar dependencias del proyecto\n");
            println!("USO:");
            println!("  caper install [--dir <directorio>]\n");
            println!("DESCRIPCIÓN:");
            println!("  Lee las dependencias de .capercfg y las instala en vendors/.");
            println!("  Genera o actualiza caper.lock con las versiones exactas.\n");
            println!("FUENTES SOPORTADAS:");
            println!("  version semver  — requiere registry (próximamente)");
            println!("  git + tag/branch — clona el repositorio y extrae archivos .caper");
            println!("  path local      — copia desde una ruta relativa al proyecto\n");
            println!("EJEMPLO .capercfg:");
            println!("  {{");
            println!("    \"dependencies\": {{");
            println!("      \"@autor/pkg\":   \"^1.2.0\",");
            println!("      \"@otro/lib\":    {{ \"git\": \"https://github.com/.../lib.git\", \"tag\": \"v1.0\" }},");
            println!("      \"mi-lib\":       {{ \"path\": \"../mi-lib\" }}");
            println!("    }}");
            println!("  }}");
        }
        Some("fmt") => {
            println!("caper fmt — formatear archivos .caper\n");
            println!("USO:");
            println!("  caper fmt <script.caper> [--write]\n");
            println!("OPCIONES:");
            println!("  --write / -w   Reescribir el archivo (sin la opción imprime en stdout)\n");
            println!("CONFIGURACIÓN (en .capercfg):");
            println!("  {{");
            println!("    \"fmt\": {{");
            println!("      \"indent_size\":     4,      # espacios por nivel");
            println!("      \"quotes\":          \"double\", # \"double\" o \"single\"");
            println!("      \"max_blank_lines\": 1       # máx. líneas en blanco consecutivas");
            println!("    }}");
            println!("  }}");
        }
        Some("lint") => {
            println!("caper lint — análisis estático de scripts .caper\n");
            println!("USO:");
            println!("  caper lint <script.caper> [script2.caper ...]\n");
            println!("SALIDA:");
            println!("  Exit 0 — solo advertencias o sin issues");
            println!("  Exit 1 — al menos un error\n");
            println!("DIAGNÓSTICOS:");
            println!("  MISSING_EMIT   El script nunca produce salida con emit");
            println!("  DEAD_EMIT      Múltiples emit; los anteriores son inalcanzables");
            println!("  NO_RETURN      Una función no tiene emit (retornará null)");
        }
        Some("repl") => {
            println!("caper repl — REPL interactivo\n");
            println!("  .help    Mostrar ayuda del REPL");
            println!("  .exit    Salir (también Ctrl-D)\n");
            println!("  Las expresiones se auto-envuelven en main {{}}.");
            println!("  El historial se guarda en ~/.caper_history (feature: repl).");
        }
        Some(other) => {
            eprintln!("caper: no hay ayuda detallada para '{other}'.");
            eprintln!("       ejecuta 'caper --help' para ver todos los comandos.");
            process::exit(1);
        }
        None => print_usage(),
    }
}

// ---------------------------------------------------------------------------
// caper -m  — listar módulos de stdlib
// ---------------------------------------------------------------------------

fn cmd_modules() {
    println!("AlcaparraLang — stdlib modules\n");

    for m in stdlib::MODULES {
        println!("  \x1b[1m{}\x1b[0m  ({})", m.name, m.path);
        println!("  Author:      {}", m.author);
        println!("  Description: {}", m.description);
        // Limitar la lista de funciones a 72 caracteres.
        let funcs_line = m.functions.join(", ");
        if funcs_line.len() <= 72 {
            println!("  Functions:   {funcs_line}");
        } else {
            print!("  Functions:   ");
            let mut col = 13usize;
            for (idx, f) in m.functions.iter().enumerate() {
                let sep = if idx == 0 { "" } else { ", " };
                let chunk = format!("{sep}{f}");
                if col + chunk.len() > 80 && col > 13 {
                    println!();
                    print!("               ");
                    col = 15;
                }
                print!("{chunk}");
                col += chunk.len();
            }
            println!();
        }
        println!();
    }
}

// ---------------------------------------------------------------------------
// caper --cfg  — mostrar configuración cargada
// ---------------------------------------------------------------------------

fn cmd_cfg() {
    println!("AlcaparraLang — configuration\n");

    // Candidatos de configuración global
    println!("Global config files (lowest → highest priority):");
    for (path, found) in global_config::config_paths() {
        let status = if found { "\x1b[32mfound\x1b[0m" } else { "not found" };
        println!("  [{status}]  {}", path.display());
    }

    // Configuración de proyecto
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    println!();
    println!("Project config (.capercfg):");
    match config::find_config(&cwd) {
        Some((cfg, dir)) => {
            println!("  \x1b[32mfound\x1b[0m  {}", dir.join(".capercfg").display());
            if let Some(name) = &cfg.name {
                println!("  name:    {name}");
            }
            if let Some(ver) = &cfg.version {
                println!("  version: {ver}");
            }
            println!("  entry:   {}", cfg.entry_file());
            if !cfg.paths.is_empty() {
                println!("  paths:");
                for (alias, path) in &cfg.paths {
                    println!("    {alias} → {path}");
                }
            }
            if !cfg.plugins.is_empty() {
                println!("  plugins:");
                for (module, path) in &cfg.plugins {
                    println!("    {module} → {path}");
                }
            }
        }
        None => println!("  not found  (searched from {})", cwd.display()),
    }

    // Configuración activa fusionada
    let global = global_config::load_global_config();
    println!();
    println!("Active merged config:");
    let ctx_json = serde_json::to_string_pretty(&global.context)
        .unwrap_or_else(|_| "{}".to_string());
    println!("  context: {ctx_json}");
    if !global.paths.is_empty() {
        println!("  paths:");
        for (alias, path) in &global.paths {
            println!("    {alias} → {path}");
        }
    }
    if !global.plugins.is_empty() {
        println!("  plugins:");
        for (module, path) in &global.plugins {
            println!("    {module} → {path}");
        }
    }
}

// ---------------------------------------------------------------------------
// caper fmt  — formatear archivos fuente
// ---------------------------------------------------------------------------

fn cmd_fmt(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: caper fmt <script.caper> [--write]");
        process::exit(1);
    }

    let write_in_place = args.iter().any(|a| a == "--write" || a == "-w");
    let files: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();

    if files.is_empty() {
        eprintln!("Usage: caper fmt <script.caper> [--write]");
        process::exit(1);
    }

    // Cargar configuración de formato del .capercfg de proyecto si existe
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let fmt_cfg = config::find_config(&cwd)
        .map(|(cfg, _)| formatter::FmtConfig::from_raw(&cfg.fmt))
        .unwrap_or_default();

    let mut had_error = false;

    for path_str in &files {
        let source = match fs::read_to_string(path_str) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading {path_str}: {e}");
                had_error = true;
                continue;
            }
        };

        match formatter::format(&source, &fmt_cfg) {
            Ok(formatted) => {
                if write_in_place {
                    match fs::write(path_str, &formatted) {
                        Ok(_) => {
                            if source != formatted {
                                println!("{path_str}: reformatted");
                            } else {
                                println!("{path_str}: unchanged");
                            }
                        }
                        Err(e) => {
                            eprintln!("Error writing {path_str}: {e}");
                            had_error = true;
                        }
                    }
                } else {
                    print!("{formatted}");
                }
            }
            Err(e) => {
                eprintln!("{}", Interpreter::format_diagnostic(&e, path_str, &source));
                had_error = true;
            }
        }
    }

    if had_error {
        process::exit(1);
    }
}

// ---------------------------------------------------------------------------
// caper install  — instala dependencias de vendors
// ---------------------------------------------------------------------------

/// Instala las dependencias declaradas en el `.capercfg` del directorio de proyecto.
///
/// Uso: `caper install [--dir <directorio>]`
///
/// Descarga o copia cada dependencia a `vendors/<author>/<pkg>/` y genera `caper.lock`.
fn cmd_install(args: &[String]) {
    // Opción --dir para apuntar a otro directorio de proyecto
    let project_dir = if let Some(pos) = args.iter().position(|a| a == "--dir") {
        args.get(pos + 1).map(PathBuf::from).unwrap_or_else(|| {
            eprintln!("--dir requiere un argumento.");
            process::exit(1);
        })
    } else {
        env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    println!("Instalando dependencias en {}...", project_dir.display());

    match package::install(&project_dir) {
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
        Ok(results) => {
            let mut had_error = false;
            for result in results {
                match result {
                    package::InstallResult::Installed { name, version, source } => {
                        println!("  \x1b[32m✓\x1b[0m {name} v{version}  ({source})");
                    }
                    package::InstallResult::AlreadyInstalled { name } => {
                        println!("  \x1b[34m~\x1b[0m {name}  (ya instalado)");
                    }
                    package::InstallResult::Stub { name, reason } => {
                        println!("  \x1b[33m!\x1b[0m {name}  {reason}");
                    }
                    package::InstallResult::Failed { name, error } => {
                        eprintln!("  \x1b[31m✗\x1b[0m {name}  Error: {error}");
                        had_error = true;
                    }
                }
            }
            if had_error {
                process::exit(1);
            } else {
                println!("caper.lock actualizado.");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// caper new  — scaffolding de proyecto o paquete
// ---------------------------------------------------------------------------

/// Crea el scaffold de un proyecto nuevo o de un paquete publicable.
///
/// Uso:
///   `caper new <nombre>`                — crea proyecto en `./<nombre>/`
///   `caper new --pkg <@author/nombre>`  — crea paquete en `./<author>/<nombre>/`
fn cmd_new(args: &[String]) {
    if args.is_empty() {
        eprintln!("Uso:");
        eprintln!("  caper new <nombre>               — nuevo proyecto");
        eprintln!("  caper new --pkg <@author/nombre> — nuevo paquete");
        process::exit(1);
    }

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if args[0] == "--pkg" || args[0] == "--package" {
        // Modo paquete
        let name = args.get(1).unwrap_or_else(|| {
            eprintln!("--pkg requiere el nombre del paquete, ej: caper new --pkg @autor/mi-paquete");
            process::exit(1);
        });
        if let Err(e) = package::new_package(name, &cwd) {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    } else {
        // Modo proyecto
        let name = &args[0];
        if let Err(e) = package::new_project(name, &cwd) {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// caper run  — ejecutar scripts
// ---------------------------------------------------------------------------

fn cmd_run(args: &[String]) {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Cargar config global (sistema → usuario → binario → var de entorno), menor prioridad primero
    let global = global_config::load_global_config();

    let project_mode = args.is_empty() || args.first().map(|a| a.starts_with('-')).unwrap_or(false);

    if project_mode {
        // Modo proyecto: buscar .capercfg
        let (cfg, cfg_dir) = config::find_config(&cwd).unwrap_or_else(|| {
            eprintln!("No script specified and no .capercfg found.");
            process::exit(1);
        });

        let entry = cfg_dir.join(cfg.entry_file());

        // Aliases: global (menor prioridad) ← proyecto (gana)
        let mut aliases = global.resolved_aliases(&cfg_dir);
        aliases.extend(cfg.resolved_aliases(&cfg_dir));

        // Contexto: global ← manifiesto ← archivos del manifiesto ← CLI (gana)
        let mut context = json_value_to_context(&global.context);
        context.extend(json_value_to_context(&project_context(&cfg, &cfg_dir)));
        context.extend(parse_context_args(args));

        // Plugins: global ← proyecto (aditivo, proyecto gana en conflicto)
        let plugins: HashMap<String, String> =
            global.plugins.into_iter().chain(cfg.plugins).collect();

        run_script(entry, context, cfg_dir, aliases, plugins);
    } else {
        // Modo script: el primer argumento es la ruta del script
        let script_path = PathBuf::from(&args[0]);
        let base_dir = script_path.parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| cwd.clone());

        let mut aliases = global.resolved_aliases(&base_dir);
        let mut context = json_value_to_context(&global.context);
        let mut plugins = global.plugins;

        // Buscar .capercfg adyacente para aliases y contexto
        if let Some((cfg, cfg_dir)) = config::find_config(&base_dir) {
            aliases.extend(cfg.resolved_aliases(&cfg_dir));
            context.extend(json_value_to_context(&project_context(&cfg, &cfg_dir)));
            plugins.extend(cfg.plugins);
        }

        // El contexto de CLI tiene máxima prioridad
        context.extend(parse_context_args(&args[1..]));

        run_script(script_path, context, base_dir, aliases, plugins);
    }
}

fn run_script(
    path: PathBuf,
    context: HashMap<String, Value>,
    base_dir: PathBuf,
    aliases: HashMap<String, PathBuf>,
    plugins: HashMap<String, String>,
) {
    let source = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {e}", path.display());
        process::exit(1);
    });

    let filename = path.to_string_lossy().into_owned();
    let mut interpreter = Interpreter::with_loader(base_dir, aliases);

    // Cargar plugins dinámicos declarados en archivos de configuración
    load_dynamic_plugins(&mut interpreter, &plugins);

    match interpreter.run(&filename, &source, context) {
        Ok(value) => {
            let json: serde_json::Value = value.into();
            println!("{}", serde_json::to_string_pretty(&json).unwrap());
        }
        Err(e) => {
            // Salida diagnóstica enriquecida (estilo compilador con contexto fuente)
            eprintln!("{}", Interpreter::format_diagnostic(&e, &filename, &source));
            process::exit(1);
        }
    }
}

fn cmd_lint(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: caper lint <script.caper> [script2.caper ...]");
        process::exit(1);
    }

    let mut any_error = false;
    let total_files   = args.len();

    for script_path in args {
        let source = fs::read_to_string(script_path).unwrap_or_else(|e| {
            eprintln!("Error reading {script_path}: {e}");
            process::exit(1);
        });

        let diags = Interpreter::lint(&source);

        if diags.is_empty() {
            if total_files > 1 {
                println!("{script_path}: ok");
            } else {
                println!("ok — {script_path} has no issues");
            }
            continue;
        }

        let errors   = diags.iter().filter(|d| d.severity == Severity::Error).count();
        let warnings = diags.iter().filter(|d| d.severity == Severity::Warning).count();

        if errors > 0 { any_error = true; }

        for diag in &diags {
            eprintln!("{}", format_lint_diag(diag, script_path, &source));
        }

        let summary = match (errors, warnings) {
            (0, w) => format!("{w} warning(s)"),
            (e, 0) => format!("{e} error(s)"),
            (e, w) => format!("{e} error(s), {w} warning(s)"),
        };
        eprintln!("{script_path}: {summary}\n");
    }

    if any_error {
        process::exit(1);
    }
}

fn format_lint_diag(diag: &LintDiagnostic, filename: &str, source: &str) -> String {
    let src_opt = if diag.line > 0 { Some(source as &str) } else { None };
    diag.format(filename, src_opt)
}

fn cmd_validate(args: &[String]) {
    if args.is_empty() {
        eprintln!("Usage: caper validate <script.caper>");
        process::exit(1);
    }

    let script_path = &args[0];
    let source = fs::read_to_string(script_path).unwrap_or_else(|e| {
        eprintln!("Error reading {script_path}: {e}");
        process::exit(1);
    });

    let interpreter = Interpreter::new();
    match interpreter.validate(&source) {
        Ok(warnings) => {
            if warnings.is_empty() {
                println!("OK — {script_path} is valid");
            } else {
                for w in &warnings {
                    eprintln!("WARNING [{}] line {}: {}", w.code, w.line, w.detail);
                }
                println!("OK with {} warning(s)", warnings.len());
            }
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", Interpreter::format_diagnostic(e, script_path, &source));
            }
            process::exit(1);
        }
    }
}

// ---------------------------------------------------------------------------
// Formateo de errores
// ---------------------------------------------------------------------------

#[allow(dead_code)]
fn error_to_json(e: &alcaparra::CaperError) -> serde_json::Value {
    use alcaparra::CaperError;
    match e {
        CaperError::UserThrow { code, message, line } => serde_json::json!({
            "type":    "USER_THROW",
            "code":    code,
            "message": message,
            "line":    line
        }),
        other => serde_json::json!({
            "type":    other.code(),
            "message": other.to_string()
        }),
    }
}

// ---------------------------------------------------------------------------
// Parseo del contexto
// ---------------------------------------------------------------------------

/// Contexto del proyecto: el inline del manifiesto más sus `context_file`.
///
/// Un `context_file` que no se puede leer **corta la corrida**. No se degrada a
/// una advertencia: si el manifiesto dice que el contexto sale de un archivo y ese
/// archivo no está, lo que se correría no es el caso que se pidió correr.
fn project_context(cfg: &config::CaperConfig, cfg_dir: &std::path::Path) -> serde_json::Value {
    cfg.resolved_context(cfg_dir).unwrap_or_else(|e| {
        eprintln!("Error: {e}");
        process::exit(1);
    })
}

fn parse_context_args(args: &[String]) -> HashMap<String, Value> {
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--context" => {
                if let Some(json_str) = args.get(i + 1) {
                    return json_str_to_context(json_str);
                }
            }
            "--context-file" => {
                if let Some(path) = args.get(i + 1) {
                    let content = fs::read_to_string(path).unwrap_or_else(|e| {
                        eprintln!("Error reading context file {path}: {e}");
                        process::exit(1);
                    });
                    return json_str_to_context(&content);
                }
            }
            _ => {}
        }
        i += 1;
    }
    HashMap::new()
}

fn json_str_to_context(json_str: &str) -> HashMap<String, Value> {
    let v: serde_json::Value = serde_json::from_str(json_str).unwrap_or_else(|e| {
        eprintln!("Invalid context JSON: {e}");
        process::exit(1);
    });
    json_value_to_context(&v)
}

fn json_value_to_context(v: &serde_json::Value) -> HashMap<String, Value> {
    match v {
        serde_json::Value::Object(map) => {
            map.iter()
                .map(|(k, v)| (k.clone(), json_to_value(v.clone())))
                .collect()
        }
        serde_json::Value::Null => HashMap::new(),
        _ => {
            eprintln!("Context must be a JSON object");
            process::exit(1);
        }
    }
}

fn json_to_value(v: serde_json::Value) -> Value {
    match v {
        serde_json::Value::Null       => Value::null(),
        serde_json::Value::Bool(b)    => Value::bool(b),
        serde_json::Value::String(s)  => Value::str(s),
        serde_json::Value::Number(n)  => {
            let d: rust_decimal::Decimal = n.to_string().parse().unwrap_or_default();
            Value::number(d)
        }
        serde_json::Value::Array(arr) => {
            Value::array(arr.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => {
            let m = map.into_iter().map(|(k, v)| (k, json_to_value(v))).collect();
            Value::object(m)
        }
    }
}

// ---------------------------------------------------------------------------
// Carga de plugins
// ---------------------------------------------------------------------------

/// Carga los plugins dinámicos declarados en archivos de configuración en el intérprete.
///
/// Omite silenciosamente las entradas si el feature `dynamic_plugins` no está habilitado
/// (imprime un warning en lugar de fallar, para que el binario siga siendo funcional).
fn load_dynamic_plugins(_interpreter: &mut Interpreter, plugins: &HashMap<String, String>) {
    if plugins.is_empty() { return; }

    #[cfg(feature = "dynamic_plugins")]
    {
        for (module, path_str) in plugins {
            let path = std::path::Path::new(path_str);
            match _interpreter.load_plugin(path) {
                Ok(name) => eprintln!("[alcaparra] Loaded plugin: {name} ({path_str})"),
                Err(e)   => eprintln!("[alcaparra] Warning: could not load plugin '{module}' from {path_str}: {e}"),
            }
        }
    }

    #[cfg(not(feature = "dynamic_plugins"))]
    {
        for (module, path_str) in plugins {
            eprintln!(
                "[alcaparra] Warning: plugin '{module}' ({path_str}) ignored — \
                 recompile with --features dynamic_plugins to enable dynamic loading"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// REPL
// ---------------------------------------------------------------------------

fn cmd_repl() {
    println!("AlcaparraLang REPL 0.1.0");
    println!("Type '.exit' or Ctrl-D to quit, '.help' for help.\n");

    #[cfg(feature = "repl")]
    repl_rustyline();

    #[cfg(not(feature = "repl"))]
    repl_basic();
}

/// REPL completo con readline, historial y soporte multi-línea (feature: repl).
#[cfg(feature = "repl")]
fn repl_rustyline() {
    use rustyline::error::ReadlineError;
    use rustyline::{Config, Editor};

    let config = Config::builder()
        .history_ignore_space(true)
        .completion_type(rustyline::CompletionType::List)
        .build();

    let mut rl: Editor<(), rustyline::history::DefaultHistory> =
        Editor::with_config(config).expect("failed to create editor");

    // Cargar historial
    let history_path = dirs::home_dir()
        .map(|h| h.join(".caper_history"))
        .unwrap_or_else(|| PathBuf::from(".caper_history"));
    let _ = rl.load_history(&history_path);

    let mut interpreter = Interpreter::new();
    let context: HashMap<String, Value> = HashMap::new();

    loop {
        let prompt = ">>> ";
        match rl.readline(prompt) {
            Ok(line) => {
                let line = line.trim().to_string();
                if line.is_empty() { continue; }
                let _ = rl.add_history_entry(&line);

                if let Some(result) = repl_handle_command(&line) {
                    if result == "exit" { break; }
                    continue;
                }

                // Acumular entrada multi-línea hasta que las llaves estén balanceadas
                let mut input = line;
                while !is_balanced(&input) {
                    match rl.readline("... ") {
                        Ok(more) => { input.push('\n'); input.push_str(&more); }
                        Err(_)   => break,
                    }
                }

                let source = repl_wrap(&input);
                match interpreter.run("<repl>", &source, context.clone()) {
                    Ok(value) => {
                        if !matches!(value, Value::Null) {
                            println!("{}", format_repl_value(&value));
                        }
                    }
                    Err(e) => {
                        eprintln!("{}", Interpreter::format_diagnostic(&e, "<repl>", &source));
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("(Ctrl-C — type '.exit' to quit)");
            }
            Err(ReadlineError::Eof) => break,
            Err(e) => {
                eprintln!("REPL error: {e}");
                break;
            }
        }
    }

    let _ = rl.save_history(&history_path);
    println!("Bye!");
}

/// REPL mínimo sin readline (fallback cuando el feature `repl` está deshabilitado).
#[cfg(not(feature = "repl"))]
fn repl_basic() {
    use std::io::{self, BufRead, Write};

    let stdin  = io::stdin();
    let stdout = io::stdout();
    let mut interpreter = Interpreter::new();

    loop {
        print!(">>> ");
        stdout.lock().flush().unwrap();

        let mut line = String::new();
        match stdin.lock().read_line(&mut line) {
            Ok(0) => break,  // fin de stream
            Ok(_) => {}
            Err(e) => { eprintln!("Read error: {e}"); break; }
        }

        let line = line.trim().to_string();
        if line.is_empty() { continue; }

        if let Some(result) = repl_handle_command(&line) {
            if result == "exit" { break; }
            continue;
        }

        let source = repl_wrap(&line);
        match interpreter.run("<repl>", &source, HashMap::new()) {
            Ok(value) => {
                if !matches!(value, Value::Null) {
                    println!("{}", format_repl_value(&value));
                }
            }
            Err(e) => {
                eprintln!("{}", Interpreter::format_diagnostic(&e, "<repl>", &source));
            }
        }
    }
    println!("Bye!");
}

// ---------------------------------------------------------------------------
// Helpers del REPL
// ---------------------------------------------------------------------------

/// Maneja los comandos de punto del REPL. Retorna Some("exit") para salir, Some("") para continuar.
fn repl_handle_command(line: &str) -> Option<&'static str> {
    match line {
        ".exit" | ".quit" | "exit" | "quit" => Some("exit"),
        ".help" => {
            println!("AlcaparraLang REPL commands:");
            println!("  .exit / .quit — exit the REPL");
            println!("  .help         — show this help");
            println!();
            println!("Input rules:");
            println!("  • Single expressions or statements are auto-wrapped in main {{}}");
            println!("  • Type a full main {{}} block for multi-statement sessions");
            println!("  • Multi-line: keep typing; press Enter on empty line when done");
            Some("")
        }
        _ => None,
    }
}

/// Envuelve una entrada del REPL en `main {}` si aún no lo tiene.
/// Heurística:
/// - Ya tiene `main {` → usar tal cual
/// - Contiene la keyword `emit` → envolver en main
/// - Empieza con una keyword de sentencia (`let`, `var`, `fn`, …) → tratar como sentencias
/// - De lo contrario → tratar como expresión pura y emitirla automáticamente
fn repl_wrap(input: &str) -> String {
    let trimmed = input.trim();

    // Ya es un programa completo
    if trimmed.starts_with("main") || trimmed.starts_with("header") {
        return input.to_string();
    }

    // Ya tiene emit — solo envolver en main
    if has_keyword(trimmed, "emit") {
        return format!("main {{\n{trimmed}\n}}");
    }

    // Termina con `;` → sentencia(s) puras, emitir null (suprimido en el REPL)
    if trimmed.ends_with(';') {
        return format!("main {{\n{trimmed}\nemit null;\n}}");
    }

    // Tiene un `;` interno → sentencias + expresión pura al final
    // ej. `let x = 10; let y = 20; x + y`
    if let Some(last_semi) = trimmed.rfind(';') {
        let stmts = &trimmed[..=last_semi];
        let expr  = trimmed[last_semi + 1..].trim();
        if !expr.is_empty() {
            return format!("main {{\n{stmts}\nemit {expr};\n}}");
        }
    }

    // Expresión pura — auto-emit
    format!("main {{\nemit {trimmed};\n}}")
}

/// Verifica si `s` contiene `keyword` como palabra completa (no dentro de un string).
fn has_keyword(s: &str, keyword: &str) -> bool {
    let kw = keyword.as_bytes();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + kw.len() <= bytes.len() {
        if &bytes[i..i + kw.len()] == kw {
            let before_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let after_ok  = i + kw.len() >= bytes.len()
                || !bytes[i + kw.len()].is_ascii_alphanumeric();
            if before_ok && after_ok { return true; }
        }
        i += 1;
    }
    false
}

#[cfg(feature = "repl")]
/// Verifica si las llaves están balanceadas (para decidir si la entrada multi-línea está completa).
fn is_balanced(s: &str) -> bool {
    let mut depth: i32 = 0;
    let mut in_str = false;
    let mut str_ch = '"';
    let mut prev   = ' ';

    for ch in s.chars() {
        if in_str {
            if ch == str_ch && prev != '\\' { in_str = false; }
        } else {
            match ch {
                '"' | '\'' => { in_str = true; str_ch = ch; }
                '{'        => depth += 1,
                '}'        => { depth -= 1; if depth < 0 { return false; } }
                _          => {}
            }
        }
        prev = ch;
    }
    depth == 0
}

/// Formatea un Value para mostrarlo en el REPL.
fn format_repl_value(v: &Value) -> String {
    let json: serde_json::Value = v.clone().into();
    serde_json::to_string_pretty(&json).unwrap_or_else(|_| format!("{v}"))
}
