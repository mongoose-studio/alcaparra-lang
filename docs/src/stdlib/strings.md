# std.strings

```caper
use std.strings.{ upper, lower, trim, split, join, len };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `len(s)` | Longitud en caracteres |
| `upper(s)` | Mayúsculas |
| `lower(s)` | Minúsculas |
| `trim(s)` | Elimina espacios al inicio y fin |
| `ltrim(s)` · `trim_start(s)` | Elimina espacios al inicio |
| `rtrim(s)` · `trim_end(s)` | Elimina espacios al fin |
| `contains(s, sub)` | `true` si `sub` está en `s` |
| `starts_with(s, prefix)` | `true` si `s` comienza con `prefix` |
| `ends_with(s, suffix)` | `true` si `s` termina con `suffix` |
| `replace(s, from, to)` | Reemplaza primera ocurrencia |
| `replace_all(s, from, to)` | Reemplaza todas las ocurrencias |
| `split(s, sep)` | Divide string en array |
| `join(arr, sep)` | Une array en string |
| `substr(s, start, length)` | Subcadena por **largo** |
| `substring(s, start, end)` | Subcadena hasta la posición `end`, excluida |
| `char_at(s, i)` | El carácter en la posición `i` (vacío fuera de rango) |
| `index_of(s, sub)` | Posición de la primera aparición, o `-1` |
| `count_occurrences(s, sub)` | Cuántas veces aparece `sub`, sin superponerse |
| `repeat(s, n)` | Repite el texto `n` veces |
| `pad_left(s, n, char)` | Rellena por la izquierda hasta longitud `n` |
| `pad_right(s, n, char)` | Rellena por la derecha |
| `str(v)` · `to_string(v)` | Convierte cualquier valor a string |
| `to_number(s)` | Convierte el texto a número (error si no se puede) |

## Ejemplos

```caper
use std.strings.{ upper, lower, trim, split, join, pad_left, str, replace_all };

let nombre = trim("  Juan Pérez  ");              // "Juan Pérez"
let codigo = upper("afp-habitat");                 // "AFP-HABITAT"

// Formatear RUT
let rut_limpio = replace_all("12.345.678-9", ".", "");  // "12345678-9"

// Formatear número como string con padding
let folio = pad_left(str(42), 8, "0");            // "00000042"

// Parsear CSV simple
let fila    = "Juan;850000;AFP-HABITAT";
let campos  = split(fila, ";");
let nombre  = campos[0];   // "Juan"
let sueldo  = campos[1];   // "850000" (string)

// Construir un código
let partes  = ["CL", "2026", "03"];
let periodo = join(partes, "-");                   // "CL-2026-03"
```

## `substr` y `substring` no son lo mismo

Los dos nombres existen y cada uno tiene la semántica que se le conoce por convención. El tercer
argumento es lo que cambia: `substr` recibe un **largo**, `substring` una **posición final**.

```caper
substr("abcde", 1, 3)      // "bcd"  — tres caracteres desde la posición 1
substring("abcde", 1, 3)   // "bc"   — desde la posición 1 hasta la 3, excluida
```

## Las posiciones se cuentan en caracteres, no en bytes

Importa en español, donde las tildes y la `ñ` ocupan más de un byte.

```caper
index_of("mañana", "n")    // 4   — no 5
char_at("mañana", 2)       // "ñ"
```
