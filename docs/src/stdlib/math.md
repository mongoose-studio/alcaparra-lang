# std.math

```caper
use std.math.{ round, min, max, abs };
// o todo el módulo:
use std.math;
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `abs(n)` | Valor absoluto |
| `ceil(n)` | Entero superior más cercano |
| `floor(n)` | Entero inferior más cercano |
| `round(n, d)` | Redondea a `d` decimales |
| `trunc(n, d)` | Trunca a `d` decimales sin redondear |
| `max(a, b)` | El mayor de dos valores |
| `min(a, b)` | El menor de dos valores |
| `pow(base, exp)` | Potencia |
| `sqrt(n)` | Raíz cuadrada |
| `sign(n)` | `-1`, `0` o `1` según el signo |

## Ejemplos

```caper
use std.math.{ round, min, abs, ceil, floor };

let desc_afp  = round(850000 * 0.1157, 0);   // 98345
let desc_tope = min(desc_afp, 100000);        // 98345

let diferencia = abs(-5000);                  // 5000
let pagos       = ceil(1500 / 400);           // 4 (cuotas)
let parte_entera = floor(3.99);              // 3
```

### round — el más usado en nómina

```caper
round(1234.567, 2)    // 1234.57
round(1234.567, 0)    // 1235
round(1234.567, -2)   // 1200  (redondea a centenas)
```

> `round` usa redondeo bancario (half-to-even) por defecto, que es el estándar para cálculos financieros.
