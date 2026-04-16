# std.types

```caper
use std.types.{ type_of, is_null, number, bool };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `type_of(v)` | `"number"`, `"string"`, `"bool"`, `"array"`, `"object"`, `"null"` |
| `is_null(v)` | `v == null` |
| `is_number(v)` | `true` si es número |
| `is_string(v)` | `true` si es string |
| `is_bool(v)` | `true` si es booleano |
| `is_array(v)` | `true` si es array |
| `is_object(v)` | `true` si es objeto |
| `number(v)` | Coerce a número (error si no parseable) |
| `int(v)` | Coerce a entero (trunca decimales) |
| `bool(v)` | Coerce a booleano |

## Valores falsy

`bool(v)` retorna `false` para: `0`, `""`, `null`, `[]`, `{}`. Todo lo demás es truthy.

## Ejemplos

```caper
use std.types.{ type_of, is_null, number, int };

let t1 = type_of(42);          // "number"
let t2 = type_of("hola");      // "string"
let t3 = type_of([1, 2]);      // "array"
let t4 = type_of(null);        // "null"

// Coerce seguro desde contexto inyectado
let sueldo = number(SUELDO_RAW ?? "0");     // string → Decimal
let dias   = int(DIAS_TRABAJADOS ?? "30");  // string → entero

// Validar antes de operar
if not is_number(TASA) {
    throw { code: "TIPO_INVALIDO", message: "TASA debe ser un número" };
}
```
