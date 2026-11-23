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
| `set(obj, key, val)` | Copia del objeto con la clave puesta en ese valor |
| `delete(obj, key)` | Copia del objeto sin esa clave |
| `from_entries(pares)` | Arma un objeto desde pares `[clave, valor]` — la vuelta de `entries` |
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

## Nada muta

`set` y `delete` devuelven una **copia** y dejan el original intacto, igual que `push` sobre
arreglos. En un lenguaje determinista conviene que nada cambie a espaldas de quien llama.

```caper
use std.objects.{ set, delete, keys };

let base = { a: 1, b: 2 };
let con_c = set(base, "c", 3);      // { a: 1, b: 2, c: 3 }
let sin_a = delete(base, "a");      // { b: 2 }
keys(base);                          // ["a", "b"]  — intacto
```

Borrar una clave que no existe no es un error: devuelve la copia igual. Y el orden de las demás
claves se conserva.

## `entries` y `from_entries` son ida y vuelta

```caper
use std.objects.{ entries, from_entries };

let o = { a: 1, b: 2 };
from_entries(entries(o));   // { a: 1, b: 2 }
```
