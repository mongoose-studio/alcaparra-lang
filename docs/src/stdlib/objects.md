# std.objects

```caper
use std.objects.{ keys, values, has, merge, pick, omit };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `keys(obj)` | Array de claves string |
| `values(obj)` | Array de valores |
| `entries(obj)` | Array de `[clave, valor]` |
| `has(obj, key)` | `true` si la clave existe |
| `get(obj, key, default)` | Obtiene valor o `default` si no existe |
| `merge(obj1, obj2)` | Une dos objetos (obj2 prevalece en conflicto) |
| `omit(obj, keys)` | Retorna objeto sin las claves indicadas |
| `pick(obj, keys)` | Retorna objeto solo con las claves indicadas |
| `count(obj)` | Número de claves |

## Ejemplos

```caper
use std.objects.{ keys, has, get, merge, pick, omit };

let empleado = {
    nombre: "Juan",
    rut:    "12345678-9",
    sueldo: 850000,
    activo: true
};

let campos = keys(empleado);         // ["nombre", "rut", "sueldo", "activo"]
let existe  = has(empleado, "rut");  // true

// Acceso seguro con default
let cargo  = get(empleado, "cargo", "Sin cargo");   // "Sin cargo"

// Fusionar objetos (útil para enriquecer resultados)
let base   = { nombre: "Juan", sueldo: 850000 };
let extras = { cargo: "Analista", depto: "TI" };
let completo = merge(base, extras);
// { nombre: "Juan", sueldo: 850000, cargo: "Analista", depto: "TI" }

// Seleccionar solo algunos campos
let publico  = pick(empleado, ["nombre", "cargo"]);
let sin_rut  = omit(empleado, ["rut"]);
```
