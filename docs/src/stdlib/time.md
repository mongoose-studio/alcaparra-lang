# std.time

```caper
use std.time.{ timestamp, elapsed_ms };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `timestamp()` | Unix timestamp actual en segundos |
| `timestamp_ms()` | Unix timestamp actual en milisegundos |
| `from_timestamp(ts)` | Convierte unix timestamp → string fecha ISO |
| `from_timestamp_ms(ts)` | Igual desde milisegundos |
| `to_timestamp(s)` | Convierte string fecha ISO → unix timestamp |
| `time_now()` | Hora actual como string `"HH:MM:SS"` |
| `time_format(ts, fmt)` | Formatea un timestamp con formato personalizado |
| `elapsed_ms(ts_inicio)` | Milisegundos transcurridos desde `ts_inicio` |

## Ejemplos

```caper
use std.time.{ timestamp, elapsed_ms, from_timestamp };

// Medir tiempo de cálculo (útil en scripts complejos)
let inicio  = timestamp();
// ... lógica de cálculo ...
let duracion = elapsed_ms(inicio);

emit {
    resultado:    RESULTADO,
    tiempo_ms:    duracion
};

// Convertir timestamp a fecha legible
let fecha = from_timestamp(1713225600);   // "2024-04-16T00:00:00Z"
```
