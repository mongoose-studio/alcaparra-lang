# std.sort

```caper
use std.sort.{ order_by, group_by_key, sort_desc };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `sort_asc(arr)` | Ordena ascendente |
| `sort_desc(arr)` | Ordena descendente |
| `sort_by(arr, key)` | Ordena array de objetos por un campo string |
| `sort_by_desc(arr, key)` | Igual pero descendente |
| `order_by(arr, criterios)` | Multi-campo: `[["campo", "asc"], ["otro", "desc"]]` |
| `group_by(arr, fn)` | Agrupa en objeto `{ clave: [items] }` |
| `group_by_key(arr, key)` | Atajo cuando el criterio es un campo del objeto |
| `partition(arr, fn)` | Divide en `[los_que_cumplen, los_que_no]` |
| `chunk(arr, n)` | Divide en sub-arrays de tamaño `n` |

## Ejemplos

```caper
use std.sort.{ order_by, group_by_key, partition, chunk };

// Ordenar por múltiples campos
let ordenadas = order_by(LIQUIDACIONES, [
    ["sueldo_liquido", "desc"],
    ["apellido",       "asc"]
]);

// Agrupar empleados por departamento
let por_depto = group_by_key(EMPLEADOS, "departamento");
// { "RRHH": [...], "TI": [...], "Ventas": [...] }

// Separar los que tienen bono de los que no
let separados = partition(EMPLEADOS, |e| => e.bono > 0);
let con_bono  = separados[0];
let sin_bono  = separados[1];

// Procesar en lotes de 50
let lotes = chunk(EMPLEADOS, 50);
foreach lote in lotes {
    // procesar lote...
}
```

> `group_by` (con función) también está disponible como HOF — no requiere import explícito cuando se usa con arrays.
