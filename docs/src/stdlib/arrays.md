# std.arrays

```caper
use std.arrays.{ sum, avg, filter, map, count };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `count(arr)` | Número de elementos (alias: `len`) |
| `push(arr, item)` | Retorna nuevo array con `item` al final |
| `pop(arr)` | Retorna `[ultimo, resto]` — no muta |
| `shift(arr)` | Retorna el arreglo sin su primer elemento — no muta |
| `unshift(arr, item)` | Retorna nuevo array con `item` al inicio |
| `first(arr)` | Primer elemento |
| `last(arr)` | Último elemento |
| `slice(arr, start, end)` | Subarray de `start` a `end` (excluido) |
| `reverse(arr)` | Retorna array invertido |
| `sort(arr)` | Retorna array ordenado ascendente |
| `unique(arr)` | Elimina duplicados |
| `concat(a, b, ...)` | Une dos o más arreglos en uno |
| `flatten(arr)` · `flat(arr)` | Aplana un nivel de anidamiento |
| `flatten_deep(arr)` | Aplana completamente |
| `contains(arr, val)` · `includes(arr, val)` | `true` si el valor está en el array |
| `index_of(arr, val)` | Índice del valor (o `-1`) |
| `sum(arr)` | Suma de todos los elementos numéricos |
| `avg(arr)` | Promedio de los elementos numéricos |
| `min_of(arr)` · `min(arr)` | El mínimo del array |
| `max_of(arr)` · `max(arr)` | El máximo del array |
| `zip(arr1, arr2)` | `[[arr1[0],arr2[0]], ...]` |
| `range(start, end)` | Array `[start, ..., end-1]` |

Los de orden superior —`map`, `filter`, `reduce`, `find`, `any`, `all`, `sort_by`— se usan sobre
arreglos y los documenta este mismo módulo más abajo. `find_all` y `find_index` viven en
[std.search](search.md); `group_by` y `partition`, en [std.sort](sort.md).
| `range(start, end, step)` | Con paso personalizado |

## HOF (funciones de orden superior)

| Función | Descripción |
|---------|-------------|
| `map(arr, fn)` | Transforma cada elemento |
| `filter(arr, fn)` | Filtra por predicado |
| `reduce(arr, fn, init)` | Acumula en un valor |
| `find(arr, fn)` | Primer elemento que cumple el predicado |
| `any(arr, fn)` | `true` si algún elemento cumple |
| `all(arr, fn)` | `true` si todos cumplen |
| `sort_by(arr, fn)` | Ordena por función comparadora |

## Ejemplos

```caper
use std.arrays.{ sum, avg, filter, map, reduce, count, range };

let haberes    = [850000, 50000, 30000, 20000];
let total      = sum(haberes);                    // 950000
let promedio   = avg(haberes);                    // 237500

let positivos  = filter(haberes, |h| => h > 0);
let con_iva    = map(haberes, |h| => h * 1.19);
let acumulado  = reduce(haberes, |acc, h| => acc + h, 0);

let indices    = range(0, 5);      // [0, 1, 2, 3, 4]
let pares      = range(0, 10, 2);  // [0, 2, 4, 6, 8]

// Caso nómina: calcular descuentos totales de una lista de empleados
let descuentos = map(EMPLEADOS, |e| => e.sueldo * e.tasa_afp);
let total_desc = sum(descuentos);
```
