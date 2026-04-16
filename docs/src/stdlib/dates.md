# std.dates

```caper
use std.dates.{ today, date_diff, date_add, working_days };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `today()` | Fecha actual como string `"YYYY-MM-DD"` |
| `now()` | Datetime actual como string ISO 8601 |
| `year(d)` | Año como número |
| `month(d)` | Mes como número (1–12) |
| `day(d)` | Día del mes como número |
| `date_diff(d1, d2)` | Diferencia en días (`d2 - d1`) |
| `date_diff(d1, d2, unit)` | Diferencia en `"day"`, `"month"`, `"year"` |
| `date_add(d, n, unit)` | Suma `n` unidades a la fecha |
| `date_sub(d, n, unit)` | Resta `n` unidades |
| `date_format(d, fmt)` | Formatea fecha (`"DD/MM/YYYY"`, `"MM-YYYY"`, etc.) |
| `date_parse(s, fmt)` | Parsea string a fecha interna |
| `is_valid_date(s)` | `true` si el string es una fecha válida |
| `working_days(year, month)` | Días hábiles del mes (lunes–viernes, sin festivos CL) |

## Ejemplos

```caper
use std.dates.{ today, year, month, date_diff, date_add, date_format, working_days };

let hoy       = today();                           // "2026-04-16"
let anio      = year(hoy);                         // 2026
let mes       = month(hoy);                        // 4

// Proporcional por días trabajados
let dias_mes  = working_days(anio, mes);           // días hábiles del mes actual
let sueldo_prop = SUELDO_BASE * DIAS_TRABAJADOS / dias_mes;

// Antigüedad en años
let antiguedad = date_diff(FECHA_INGRESO, hoy, "year");

// Próximo vencimiento
let vence = date_add(hoy, 30, "day");              // "2026-05-16"

// Formato para documentos
let fecha_doc = date_format(hoy, "DD/MM/YYYY");    // "16/04/2026"
```

> `working_days` usa el calendario de festivos de Chile. El dataset se actualiza anualmente en cada release del runtime.
