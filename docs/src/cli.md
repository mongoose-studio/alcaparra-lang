# CLI — caper

## Instalación

```bash
cargo install alcaparra-lang
```

## Comandos

### run

Ejecuta un script `.caper`:

```bash
caper run script.caper
caper run script.caper --context '{"SUELDO": 850000, "TASA": 0.1157}'
caper run script.caper --context-file contexto.json
```

Modo proyecto (lee `entry` y `context` desde `.capercfg`):

```bash
caper run
```

### validate

Valida sintaxis sin ejecutar. No requiere contexto:

```bash
caper validate script.caper
```

### repl

REPL interactivo:

```bash
caper repl
```

Con historial persistente y autocompletado (requiere feature `repl`):

```bash
caper repl   # usa ~/.caper_history
```

### new

Crea un nuevo proyecto o paquete:

```bash
caper new mi-proyecto          # proyecto con .capercfg + main.caper
caper new mi-lib --package     # paquete con pkg.capercfg + lib.caper
```

### install

Instala dependencias declaradas en `.capercfg` o una dependencia específica:

```bash
caper install                                          # instala todas las deps del .capercfg
caper install /ruta/local/mi-paquete                  # dep local
caper install git+https://github.com/org/pkg           # desde git
caper install git+https://github.com/org/pkg --tag v1  # tag específico
```

### help

```bash
caper help
caper help run
caper help install
```

### version

```bash
caper version
# AlcaparraLang 0.1.0
```

## Opciones globales

| Opción | Descripción |
|--------|-------------|
| `--context '{"K": V}'` | Contexto JSON inline |
| `--context-file ruta.json` | Contexto desde archivo |

## Formato de salida

La salida de `caper run` es JSON por stdout:

```json
{ "status": "ok",    "result": { "liquido": 751655 } }
{ "status": "error", "error": { "type": "TYPE_ERROR", "message": "...", "line": 5 } }
```

Los errores de sintaxis muestran diagnóstico estilo Rust:

```
error[SYNTAX_ERROR]: expected ';' at 5:12
  --> script.caper:5:12
   |
 5 |     let x = foo(bar
   |                 ^
   |
```
