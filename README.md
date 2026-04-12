# AlcaparraLang
### by Mongoose Studio

![Rust](https://img.shields.io/badge/Rust-1.94+-orange)
![Alcaparra](https://img.shields.io/badge/Alcaparra-0.1.0-olive)
![License](https://img.shields.io/badge/license-MIT-green)
![Status](https://img.shields.io/badge/status-active-success)

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="alcaparra-banner-dark.png">
  <source media="(prefers-color-scheme: light)" srcset="alcaparra-banner.png">
  <img alt="Alcaparra Framework" height="64px" src="alcaparra-banner-dark.png">
</picture>

**AlcaparraLang** es, en esencia, un **motor de cálculo determinístico embebible**. Básicamente, es un lenguaje de scripting orientado a cálculos numéricos, diseñado con un enfoque minimalista y ejecución rápida. Utiliza archivos `.caper` y se opera mediante su CLI `caper`, permitiendo una experiencia directa y sin sobrecarga.

Su diseño privilegia la simplicidad semántica y la claridad operativa, reduciendo al mínimo la fricción entre escribir código y obtener resultados. Esto lo hace ideal para scripts, procesamiento numérico y automatizaciones donde la velocidad y la legibilidad son clave.

Lejos de ser un lenguaje generalista, **AlcaparraLang** apuesta por hacer bien un problema específico: trabajar con números de forma eficiente, sin ruido innecesario.

En resumen: **AlcaparraLang** es ese lenguaje chico, ágil y sin pretensiones que simplemente hace la pega… y la hace bien 🇨🇱.

---

## Casos de uso

**AlcaparraLang** brilla cuando necesitas:

- **Cálculos de negocio** — sueldos, impuestos, comisiones, reglas financieras
- **Motor de reglas** — lógica configurable sin redeploy
- **Transformaciones de datos** — inputs → outputs JSON
- **Prototipos rápidos** — validar lógica sin levantar backend completo
- **Lógica embebida** — ejecutar scripts seguros dentro de tu app Rust
- **Generación de Datasets** — procesa grandes volúmenes de datos y genera outputs listos para BI
- **Determinismo total** — mismo input → mismo output. Siempre.

Si tu problema es “tomar datos, aplicar reglas y devolver resultados”… estás en casa.

---

Ahora, a modo de ejemplo:

```caper
use std.math.{ round };

header {
    name: "liquidacion";
    version: "1.0.0";
    const TASA_AFP = 0.1157;
}

main {
    let bruto      = SUELDO_BASE ?? 0;
    let desc_afp   = round(bruto * TASA_AFP, 0);
    let desc_salud = round(bruto * FONASA_TASA, 0);

    emit {
        sueldo_bruto:   bruto,
        desc_afp:       desc_afp,
        desc_salud:     desc_salud,
        sueldo_liquido: bruto - desc_afp - desc_salud
    };
}
```

```bash
caper run liquidacion.caper --context '{"SUELDO_BASE": 850000, "FONASA_TASA": 0.07}'
# → { "sueldo_bruto": 850000, "desc_afp": 98345, "desc_salud": 59500, "sueldo_liquido": 692155 }
```

## ¿Por qué AlcaparraLang?

- **Aritmética decimal exacta** — todos los números usan `rust_decimal::Decimal`. Sin errores de punto flotante en cálculos monetarios.
- **Inyección de contexto** — los valores del entorno (`SUELDO_BASE`, tasas, etc.) se inyectan directamente en el scope del script. Los scripts no acceden a bases de datos ni HTTP — solo calculan.
- **Embebible** — usa el crate `alcaparra` directamente en tu aplicación Rust. Sin subprocesos, sin FFI.
- **Seguro por diseño** — sin acceso a sistema de archivos, red ni sistema operativo desde dentro de un script.
- **Lenguaje completo** — funciones, closures, recursión, pattern matching, try/catch, iteradores, imports multi-archivo.

---

## Instalación

### Desde el código fuente

```bash
git clone https://github.com/mongoose-studio/alcaparra-lang
cd alcaparra-lang
cargo install --path .
```

Features opcionales:

```bash
# REPL con soporte readline (historial, edición de línea)
cargo install --path . --features repl

# Carga dinámica de plugins (.so / .dll / .dylib)
cargo install --path . --features dynamic_plugins

# Ambos
cargo install --path . --features repl,dynamic_plugins
```

---

## CLI

```
caper run [script.caper] [--context '{"CLAVE": val}']   Ejecutar un script
caper run [script.caper] [--context-file ctx.json]      Ejecutar con contexto desde archivo
caper run                                               Modo proyecto (lee .capercfg)
caper validate <script.caper>                           Solo verificar sintaxis
caper lint     <script.caper> [...]                     Análisis estático (apto para CI)
caper fmt      <script.caper> [--write]                 Formatear código fuente
caper install                                           Instalar dependencias
caper new <nombre>                                      Nuevo proyecto
caper new --pkg <@author/nombre>                        Nuevo paquete publicable
caper repl                                              REPL interactivo
caper -m                                                Listar módulos de la stdlib
caper --cfg                                             Mostrar archivos de configuración cargados
caper version
```

### `caper run`

```bash
# Contexto inline
caper run script.caper --context '{"SUELDO_BASE": 850000, "TASA": 0.1157}'

# Contexto desde archivo
caper run script.caper --context-file context.json

# Modo proyecto — lee entry + contexto desde .capercfg en el directorio actual
caper run
```

La salida siempre es JSON impreso en stdout.

### `caper lint`

Ejecuta lex → parse → compile → análisis estático. Sale con código 0 si solo hay advertencias, con 1 si hay errores.

```bash
caper lint script.caper            # un archivo
caper lint src/**/*.caper          # múltiples archivos
```

Detecciones:
- `MISSING_EMIT` — el script nunca produce salida
- `DEAD_EMIT` — múltiples sentencias `emit`; las anteriores son inalcanzables
- `NO_RETURN` — una función no tiene `emit`

### `caper fmt`

Formateador basado en tokens. Preserva comentarios y líneas en blanco.

```bash
caper fmt script.caper             # imprime en stdout
caper fmt script.caper --write     # reescribe el archivo
```

Configurable en `.capercfg`:

```json
{
  "fmt": {
    "indent_size": 4,
    "quotes": "double",
    "max_blank_lines": 1
  }
}
```

### `caper -m`

Lista todos los módulos de la stdlib con autor, descripción y nombres de funciones.

### `caper --cfg`

Muestra qué archivos `alcaparra.capercfg` fueron encontrados (similar a `php --ini`), el `.capercfg` de proyecto más cercano y la configuración activa combinada.

---

## Referencia del lenguaje

### Estructura de un script

Un script puede tener un bloque `header` opcional (metadatos + constantes) y un bloque `main`. Los scripts sin `main` ejecutan sus sentencias directamente en el nivel superior.

```caper
header {
    name: "mi-script";
    version: "1.0.0";
    const TASA_IVA = 0.19;   // inmutable, no puede ser sobreescrita por el contexto
}

main {
    let total = MONTO * (1 + TASA_IVA);
    emit { total: total };
}
```

### Variables

```caper
let x = 42;          // inmutable — no se puede reasignar
var contador = 0;    // mutable — puede reasignarse
contador = contador + 1;
```

### Tipos

| Tipo    | Ejemplo                        | Notas                                     |
|---------|--------------------------------|-------------------------------------------|
| Número  | `42`, `3.14`, `1_000_000`      | Siempre `Decimal` internamente — sin f64  |
| String  | `"hola"`, `'mundo'`            | Comillas simples o dobles equivalentes    |
| Bool    | `true`, `false`                |                                           |
| Null    | `null`                         | Ausencia de valor                         |
| Array   | `[1, 2, "x"]`                  | Indexado desde 0, tipos mixtos ok         |
| Objeto  | `{ nombre: "Ana", edad: 30 }` | Orden de inserción preservado             |

La aritmética decimal es exacta:

```caper
let a = 0.1 + 0.2;   // 0.3 — no 0.30000000000000004
```

### Operadores

```caper
// Aritméticos
a + b    a - b    a * b    a / b    a % b    a ** b

// Comparación
a == b   a != b   a > b   a < b   a >= b   a <= b

// Lógicos
a and b   a or b   not a

// Null coalescing
valor ?? valor_por_defecto    // retorna valor_por_defecto cuando valor es null

// Concatenación de strings
"Hola " + nombre

// Rango (produce un array)
1..10    // [1, 2, ..., 10]
```

### Control de flujo

```caper
// if / else if / else — como sentencia o como expresión
if puntaje >= 90 {
    nota = "A";
} else if puntaje >= 70 {
    nota = "B";
} else {
    nota = "C";
}

let etiqueta = if activo { "activo" } else { "inactivo" };

// while
while (i < 10) {
    i = i + 1;
}

// foreach
foreach item in items {
    total = total + item.precio;
}

foreach clave, valor in mapa {
    // clave y valor disponibles
}

// break y continue funcionan dentro de loops
```

### match

```caper
// Match exacto
let descripcion = match tipo_contrato {
    "indefinido" => "Contrato a Plazo Indefinido",
    "plazo_fijo" => "Contrato a Plazo Fijo",
    "honorarios" => "Boleta de Honorarios",
    _            => "Tipo no reconocido"
};

// Con guards
let tramo = match sueldo {
    n if n < 500_000   => "exento",
    n if n < 1_500_000 => "medio",
    _                  => "alto"
};
```

### Funciones y closures

```caper
fn factorial(n) {
    if n <= 1 { emit 1; }
    emit n * factorial(n - 1);
}

// Closures
let doble  = |x| => x * 2;
let sumar  = |a, b| => a + b;

// Closure con cuerpo
let clamp = |val, min, max| => {
    if val < min { emit min; }
    if val > max { emit max; }
    emit val;
};
```

### Arrays y objetos

```caper
let items = [10, 20, 30];
let primero = items[0];    // 10
let ultimo  = items[-1];   // 30

let persona = { nombre: "Ana", edad: 30 };
let nombre  = persona.nombre;
let edad    = persona["edad"];

// Spread
let a = [1, 2];
let b = [...a, 3, 4];            // [1, 2, 3, 4]

let base    = { x: 1 };
let merged  = { ...base, y: 2 }; // { x: 1, y: 2 }
```

### try / catch / throw

```caper
try {
    let resultado = funcion_riesgosa(entrada);
    emit { ok: resultado };
} catch e {
    emit { error: e.message };
}

// throw un valor estructurado
throw { code: "ENTRADA_INVALIDA", message: "El monto debe ser positivo" };
```

### emit

`emit` termina la ejecución y retorna un valor (equivalente al `return` de otros lenguajes). Solo se retorna el primer `emit` que se alcanza.

```caper
fn clasificar(n) {
    if n > 0 { emit "positivo"; }
    if n < 0 { emit "negativo"; }
    emit "cero";
}
```

### Imports

```caper
// Importar funciones específicas de la stdlib
use std.math.{ round, min, max };
use std.arrays.{ sum, filter, map };

// Importar un script de usuario (ruta relativa)
use formulas.gratificacion;

// Importar con alias de ruta (configurado en .capercfg)
use @formulas.liquidacion;

// Importar desde un paquete de vendor (instalado con caper install)
use @author/mi-paquete.lib.{ mi_funcion };
use autor/otro-pkg.modulo.{ helper };
```

---

## Librería estándar

Ejecuta `caper -m` para ver todos los módulos con sus listas de funciones.

| Módulo    | Path            | Descripción                                         |
|-----------|-----------------|-----------------------------------------------------|
| `math`    | `std.math`      | abs, ceil, floor, round, sqrt, log, …               |
| `strings` | `std.strings`   | len, upper, trim, split, join, regex, …             |
| `arrays`  | `std.arrays`    | count, map, filter, reduce, sum, sort, …            |
| `objects` | `std.objects`   | keys, values, merge, pick, omit, …                  |
| `types`   | `std.types`     | type_of, is_null, is_number, to_bool, …             |
| `json`    | `std.json`      | json_encode, json_decode, json_pretty               |
| `dates`   | `std.dates`     | today, date_diff, date_add, working_days (feriados CL) |
| `rand`    | `std.rand`      | rand, rand_int, shuffle, uuid                       |
| `regex`   | `std.regex`     | regex_match, regex_find, regex_replace              |
| `crypto`  | `std.crypto`    | md5, sha256, hmac_sha256, base64_encode             |
| `time`    | `std.time`      | timestamp, elapsed_ms, time_format                  |
| `sort`    | `std.sort`      | sort_asc, order_by, group_by_key, chunk             |
| `search`  | `std.search`    | binary_search, fuzzy, find_all                      |
| `xml`     | `std.xml`       | xml_encode, xml_decode, xml_get                     |
| `yaml`    | `std.yaml`      | yaml_encode, yaml_decode, yaml_valid                |

Las funciones de orden superior (`map`, `filter`, `reduce`, `sort_by`, `group_by`, `find`, `any`, `all`, etc.) aceptan closures:

```caper
use std.arrays.{ map, filter, sum };

let sueldos = [850000, 1200000, 450000, 2100000];

let altos  = filter(sueldos, |s| => s > 1_000_000);
let dobles = map(sueldos, |s| => s * 2);
let total  = sum(sueldos);
```

---

## Gestión de paquetes

**AlcaparraLang** incluye un sistema de paquetes integrado en la CLI, con soporte para
paquetes de terceros versionados, dependencias locales y repositorios git.

### Crear un proyecto nuevo

```bash
caper new mi-proyecto
cd mi-proyecto
caper run
```

Genera:

```
mi-proyecto/
  .capercfg     ← manifiesto del proyecto
  main.caper    ← script principal
  .gitignore
```

### Crear un paquete publicable

```bash
caper new --pkg @mi-usuario/formulas-cl
```

Genera:

```
@mi-usuario/
  formulas-cl/
    pkg.capercfg    ← manifiesto del paquete
    lib.caper       ← módulo principal
    .gitignore
```

### Declarar dependencias

En `.capercfg`, agrega la sección `dependencies`:

```json
{
  "name":    "mi-proyecto",
  "version": "1.0.0",
  "entry":   "main.caper",
  "dependencies": {
    "@autor/formulas-cl":  "^1.2.0",
    "@otro/validaciones":  { "git": "https://github.com/otro/validaciones.git", "tag": "v0.5.1" },
    "mi-lib-interna":      { "path": "../mi-lib" }
  }
}
```

Fuentes soportadas:

| Tipo | Ejemplo | Estado |
|------|---------|--------|
| `"^1.2.0"` (semver) | versión del registry | pendiente registry |
| `{ "git": "url", "tag": "v1.0" }` | repositorio git | operativo |
| `{ "git": "url", "branch": "main" }` | branch git | operativo |
| `{ "path": "../mi-lib" }` | ruta local | operativo |

### Instalar dependencias

```bash
caper install
```

Los paquetes se instalan en `vendors/<author>/<pkg>/` y se genera `caper.lock`:

```
mi-proyecto/
  .capercfg
  caper.lock       ← versiones exactas bloqueadas (commitear)
  vendors/
    @autor/
      formulas-cl/
        pkg.capercfg
        lib.caper
    mi-lib-interna/
      pkg.capercfg
      ...
```

### Importar desde un paquete instalado

```caper
use @autor/formulas-cl.lib.{ calcular_afp, calcular_salud };
use mi-lib-interna.helpers.{ formatear_rut };

main {
    let afp = calcular_afp(SUELDO_BASE);
    emit { afp: afp };
}
```

### `caper.lock`

El lockfile registra las versiones y fuentes exactas para builds reproducibles.
**Commitea `caper.lock` junto al código fuente.**

```json
{
  "version": 1,
  "packages": [
    {
      "name":    "@autor/formulas-cl",
      "version": "1.2.0",
      "source":  "git+https://github.com/autor/formulas-cl#a1b2c3d"
    }
  ]
}
```

### Registry (próximamente)

El registry público estará disponible en `registry.alcaparra.dev`.
La especificación completa de la API se encuentra en [`docs/registry-spec.md`](docs/registry-spec.md).

---

## Integración con Rust

Agrega al `Cargo.toml`:

```toml
[dependencies]
alcaparra = { git = "https://github.com/mongoose-studio/alcaparra-lang" }
```

### Uso básico

```rust
use alcaparra::{Interpreter, Value};
use rust_decimal_macros::dec;
use std::collections::HashMap;

let mut interp = Interpreter::new();

let source = r#"
    let base = SUELDO_BASE ?? 0;
    emit { liquido: base * 0.883 };
"#;

let mut context = HashMap::new();
context.insert("SUELDO_BASE".to_string(), Value::number(dec!(850000)));

let result = interp.run("mi-script", source, context)?;
// result es un Value::Object con los campos del emit
```

### Registrar una función nativa

```rust
interp.register_fn("triple", |args, _line| {
    if let Some(Value::Number(n)) = args.into_iter().next() {
        Ok(Value::number(n * dec!(3)))
    } else {
        Ok(Value::Null)
    }
});
```

La función queda disponible por nombre en todas las llamadas a `run` siguientes.

### Registrar un plugin (funciones agrupadas)

```rust
use alcaparra::{CaperPlugin, CaperError, Value};

struct MiPlugin;

impl CaperPlugin for MiPlugin {
    fn module_name(&self) -> &str { "milib" }

    fn call(&self, name: &str, args: Vec<Value>, _line: usize)
        -> Result<Option<Value>, CaperError>
    {
        match name {
            "saludar" => {
                let quien = args.into_iter().next()
                    .and_then(|v| if let Value::Str(s) = v { Some(s) } else { None })
                    .unwrap_or_else(|| "mundo".to_string());
                Ok(Some(Value::str(format!("Hola, {quien}!"))))
            }
            _ => Ok(None),
        }
    }
}

interp.register_plugin(Box::new(MiPlugin));
```

```caper
// En un script:
let msg = saludar("Ana");   // → "Hola, Ana!"
```

### Plugins dinámicos (feature: `dynamic_plugins`)

Plugins de terceros compilados como librerías compartidas (`.so` / `.dll` / `.dylib`) con ABI C:

```c
// Implementar estas dos funciones
const char* alcaparra_plugin_module();

const char* alcaparra_plugin_call(
    const char* fn_name,
    const char* args_json,   // array JSON de argumentos
    char*       out_buf,     // buffer de salida (1 MB)
    int         buf_len
);   // retorna valor JSON o "ERROR: mensaje"
```

Carga desde código:

```rust
interp.load_plugin(std::path::Path::new("/opt/plugins/milib.so"))?;
```

O declara en `alcaparra.capercfg`:

```json
{ "plugins": { "milib": "/opt/plugins/milib.so" } }
```

---

## Configuración

### Manifiesto de proyecto (`.capercfg`)

Coloca `.capercfg` en la raíz del proyecto. `caper run` (sin argumento de script) lee el entry point y el contexto desde ahí.

```json
{
  "name":        "mis-formulas",
  "version":     "1.0.0",
  "entry":       "main.caper",
  "context": {
    "PAIS":    "CL",
    "MONEDA":  "CLP"
  },
  "paths": {
    "@formulas": "./formulas",
    "@lib":      "./lib"
  },
  "plugins": {
    "milib": "./plugins/milib.so"
  },
  "fmt": {
    "indent_size":     4,
    "quotes":          "double",
    "max_blank_lines": 1
  }
}
```

### Configuración global (`alcaparra.capercfg`)

Se carga en orden de prioridad ascendente (mayor prioridad gana en conflictos):

| Prioridad     | Ubicación                              |
|---------------|----------------------------------------|
| 1 (más baja)  | `/etc/alcaparra/alcaparra.capercfg`    |
| 2             | `~/.alcaparra/alcaparra.capercfg`      |
| 3             | `<directorio_del_binario>/alcaparra.capercfg` |
| 4             | `$ALCAPARRA_CONFIG` (variable de entorno, ruta al archivo) |
| 5             | `.capercfg` de proyecto                |
| 6 (más alta)  | CLI `--context` / `--context-file`     |

`paths` y `plugins` son **aditivos** entre niveles. Las claves de `context` siguen la prioridad (el nivel más alto gana en conflictos).

Ejecuta `caper --cfg` para inspeccionar qué archivos fueron encontrados y cómo quedó la configuración combinada.

---

## REPL

```bash
caper repl   # REPL básico (siempre disponible)
caper repl   # REPL con readline e historial si se compiló con --features repl
```

```
>>> 1 + 2
3
>>> let x = 10; let y = 20; x + y
30
>>> fn doble(n) { emit n * 2; }; doble(21)
42
>>> .help
>>> .exit
```

El historial se guarda en `~/.caper_history` (feature: `repl`).

---

## Licencia

Este proyecto está licenciado bajo la Licencia MIT - ver el archivo [LICENSE](LICENSE.txt) para más detalles.

## Autor

**Marcel Rojas**  
[marcelrojas16@gmail.com](mailto:marcelrojas16@gmail.com)  
__Mongoose Studio__

## Contribuciones

Las contribuciones son bienvenidas. Por favor:

1. Fork el proyecto
2. Crea una rama para tu feature (`git checkout -b feature/amazing-feature`)
3. Commit tus cambios (`git commit -m 'Add amazing feature'`)
4. Push a la rama (`git push origin feature/amazing-feature`)
5. Abre un Pull Request

---

💚**AlcaparraLang** by Mongoose Studio
