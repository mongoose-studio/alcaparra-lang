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
| `ltrim(s)` | Elimina espacios al inicio |
| `rtrim(s)` | Elimina espacios al fin |
| `contains(s, sub)` | `true` si `sub` está en `s` |
| `starts_with(s, prefix)` | `true` si `s` comienza con `prefix` |
| `ends_with(s, suffix)` | `true` si `s` termina con `suffix` |
| `replace(s, from, to)` | Reemplaza primera ocurrencia |
| `replace_all(s, from, to)` | Reemplaza todas las ocurrencias |
| `split(s, sep)` | Divide string en array |
| `join(arr, sep)` | Une array en string |
| `substr(s, start, length)` | Subcadena |
| `pad_left(s, n, char)` | Rellena por la izquierda hasta longitud `n` |
| `pad_right(s, n, char)` | Rellena por la derecha |
| `str(v)` | Convierte cualquier valor a string |

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
