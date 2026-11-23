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
| `is_function(v)` | `true` si es una función o un closure |
| `number(v)` · `to_number(v)` | Coerce a número (error si no parseable) |
| `int(v)` | Coerce a entero (trunca decimales) |
| `bool(v)` · `to_bool(v)` | Coerce a booleano |
| `to_string(v)` | Coerce a string — lo resuelve `std.strings.str` |

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

## El contexto no se convierte solo

Los valores que entran por el contexto llegan con el tipo que traen. Un número mandado como texto
**queda como texto**, y compararlo con un número es un error de tipo, no una coerción silenciosa:

```caper
// contexto: { "SUELDO": "850000" }
SUELDO > 500000        // TYPE_ERROR: cannot compare number and string
number(SUELDO) > 500000   // true
```

Es a propósito: en un motor de cálculo, adivinar tipos es peor que fallar temprano. Si quien arma
el contexto manda números como texto, conviene convertirlos ahí y no en cada script.
