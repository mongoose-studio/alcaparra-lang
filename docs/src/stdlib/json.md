# std.json

```caper
use std.json.{ json_encode, json_decode, json_valid };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `json_encode(v)` | Serializa cualquier valor a string JSON |
| `json_decode(s)` | Parsea un string JSON → valor |
| `json_valid(s)` | `true` si el string es JSON válido, sin lanzar error |
| `json_pretty(v)` | `json_encode` con indentación legible |

## Ejemplos

```caper
use std.json.{ json_encode, json_decode, json_valid, json_pretty };

let datos = { nombre: "Juan", monto: 850000 };

let serializado = json_encode(datos);
// '{"nombre":"Juan","monto":850000}'

let de_vuelta   = json_decode(serializado);
// objeto de vuelta, accesible con de_vuelta.nombre

// Leer configuración compleja inyectada como string JSON
let config_extra = json_decode(CONFIG_JSON ?? "{}");
let tasa_extra   = config_extra.tasa_bono ?? 0;

// Validar sin lanzar error
if not json_valid(INPUT_RAW) {
    throw { code: "JSON_INVALIDO", message: "El input no es JSON válido" };
}

// Debug legible
emit json_pretty(datos);
/*
{
  "nombre": "Juan",
  "monto": 850000
}
*/
```
