# std.search

```caper
use std.search.{ find_all, find_index, in_range, fuzzy };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `binary_search(arr, val)` | Búsqueda binaria en array ordenado → índice o `-1` |
| `find_all(arr, fn)` | Todos los elementos que cumplen el predicado |
| `find_index(arr, fn)` | Índice del primer elemento que cumple |
| `in_range(val, min, max)` | `true` si `min <= val <= max` |
| `search(arr, val)` | Búsqueda lineal → índice o `-1` |
| `fuzzy(s, pattern, threshold)` | Similitud entre strings (0.0 a 1.0) |

## Ejemplos

```caper
use std.search.{ find_all, find_index, in_range, fuzzy };

let empleados_con_bono = find_all(EMPLEADOS, |e| => e.bono > 0);

let tramo_medio = find_all(EMPLEADOS, |e| =>
    in_range(e.sueldo, 500_000, 1_500_000)
);

// Índice del primer empleado con antigüedad > 5 años
let idx = find_index(EMPLEADOS, |e| => e.antiguedad > 5);

// Búsqueda aproximada de nombre (para tolerancia a errores ortográficos)
let similitud = fuzzy("Juan Perez", "Juan Pérez", 0.8);   // true si similitud >= 0.8
```
