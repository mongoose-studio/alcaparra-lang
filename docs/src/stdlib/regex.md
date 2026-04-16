# std.regex

```caper
use std.regex.{ regex_test, regex_match, regex_find, regex_replace_all };
```

Los patrones usan sintaxis de expresiones regulares estándar (Rust regex). Las barras invertidas en los patrones van **duplicadas** (`\\d`, `\\.`) porque el string de AlcaparraLang las interpreta antes que el motor de regex.

## Funciones

| Función | Descripción |
|---------|-------------|
| `regex_test(s, pattern)` | `true` si el patrón aparece en alguna parte del string |
| `regex_match(s, pattern)` | `true` si el string **completo** cumple el patrón (anchored) |
| `regex_find(s, pattern)` | Primera coincidencia como string (o `null`) |
| `regex_find_all(s, pattern)` | Array con todas las coincidencias |
| `regex_groups(s, pattern)` | Array con grupos de captura `()` de la primera coincidencia |
| `regex_replace(s, pattern, repl)` | Reemplaza primera coincidencia |
| `regex_replace_all(s, pattern, repl)` | Reemplaza todas las coincidencias |
| `regex_split(s, pattern)` | Divide el string por el patrón |

## Ejemplos

```caper
use std.regex.{ regex_test, regex_match, regex_find, regex_find_all,
                regex_groups, regex_replace_all, regex_split };

// Validar que existe un patrón en el string
let tiene_numeros = regex_test("ABC-123", "\\d+");          // true

// Validar el string completo
let rut_valido  = regex_match(RUT, "\\d{1,8}-[\\dkK]");    // true/false
let email_ok    = regex_match(EMAIL, "[^@]+@[^@]+\\.[^@]+"); // true/false

// Extraer primera coincidencia
let primer_num  = regex_find("ref AB-1234 / cod 5678", "\\d+");   // "1234"
let sin_match   = regex_find("sin numeros", "\\d+");               // null

// Extraer todas las coincidencias
let todos_nums  = regex_find_all("1, 20, 300", "\\d+");   // ["1", "20", "300"]

// Grupos de captura
let partes = regex_groups("2026-04-16", "(\\d{4})-(\\d{2})-(\\d{2})");
let anio   = partes[0];   // "2026"
let mes    = partes[1];   // "04"
let dia    = partes[2];   // "16"

// Limpieza de strings
let sin_puntos = regex_replace_all("1.250.000", "\\.", "");     // "1250000"
let normalizado = regex_replace_all(NOMBRE, "\\s+", " ");       // espacios múltiples → uno

// Split por patrón
let campos = regex_split("a, b,c ,  d", "\\s*,\\s*");  // ["a", "b", "c", "d"]
```

## Caso real: validar y limpiar RUT

```caper
use std.regex.{ regex_match, regex_replace_all };

fn validar_rut(rut) {
    // Acepta formatos: "12345678-9", "12.345.678-9", "12345678K"
    let rut_limpio = regex_replace_all(rut, "[^0-9kK-]", "");

    if not regex_match(rut_limpio, "\\d{1,8}-?[\\dkK]") {
        throw { code: "RUT_INVALIDO", message: "Formato de RUT incorrecto: " + rut };
    }
    emit rut_limpio;
}
```
