# std.rand

```caper
use std.rand.{ seed, rand_int, rand_sample, uuid };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `rand()` | Número aleatorio entre 0.0 y 1.0 |
| `rand_int(min, max)` | Entero aleatorio en `[min, max]` |
| `rand_float(min, max)` | Decimal aleatorio en `[min, max]` |
| `rand_item(arr)` | Elemento aleatorio de un array |
| `rand_sample(arr, n)` | `n` elementos aleatorios sin repetición |
| `shuffle(arr)` | Retorna copia del array con orden aleatorio |
| `seed(n)` | Fija la semilla aleatoria — para reproducibilidad |
| `uuid()` | Genera un UUID v4 como string |

## Ejemplos

```caper
use std.rand.{ seed, rand_sample, uuid, rand_int };

// Tests reproducibles — misma semilla, mismo resultado
seed(42);
let muestra = rand_sample(EMPLEADOS, 10);   // siempre los mismos 10

// ID único de proceso
let run_id = uuid();   // "a7f3c821-4e2b-4d91-b3f1-9c2e8f0d5a61"

// Simulación de variación en tests
let variacion = rand_int(-5, 5);   // entero aleatorio entre -5 y 5
```

> En scripts de producción, evita `rand()` sin `seed()` si necesitas reproducibilidad. Los tests de liquidaciones deben ser deterministas.
