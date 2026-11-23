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
| `round(n, d)` | Redondea a `d` decimales, **medio al par** |
| `round_half_up(n, d)` | Redondea a `d` decimales, **medio hacia arriba** |
| `trunc(n, d)` | Trunca a `d` decimales sin redondear |
| `max(a, b)` · `max(arr)` | El mayor de dos valores, o de un arreglo |
| `min(a, b)` · `min(arr)` | El menor de dos valores, o de un arreglo |
| `pow(base, exp)` | Potencia |
| `sqrt(n)` | Raíz cuadrada |
| `sign(n)` | `-1`, `0` o `1` según el signo |
| `clamp(n, min, max)` | Acota `n` al rango `[min, max]` |
| `log(n)` · `log(n, base)` | Logaritmo natural, o en la base dada |
| `log10(n)` | Logaritmo en base 10 |
| `exp(n)` | `e` elevado a `n` |
| `pi()` | La constante π |
| `e()` | La constante e |

## Ejemplos

```caper
use std.math.{ round, min, abs, ceil, floor };

let desc_afp  = round(850000 * 0.1157, 0);   // 98345
let desc_tope = min(desc_afp, 100000);        // 98345

let diferencia = abs(-5000);                  // 5000
let pagos       = ceil(1500 / 400);           // 4 (cuotas)
let parte_entera = floor(3.99);              // 3
```

### Redondeo — lo más importante del módulo

```caper
round(1234.567, 2)    // 1234.57
round(1234.567, 0)    // 1235
round(1234.567, -2)   // 1200   — los decimales negativos redondean a decenas, centenas y así
```

**Hay dos estrategias y la diferencia importa.** `round` redondea **medio al par**: el 0,5 va al
entero par más cercano.

```caper
round(0.5, 0)   // 0     ← no 1
round(1.5, 0)   // 2
round(2.5, 0)   // 2     ← no 3
round(3.5, 0)   // 4
```

Es la estrategia correcta para estadística y para promediar: no sesga hacia arriba. Pero **no es la
que usa la contabilidad en Chile**, que redondea medio hacia arriba. Para eso está
`round_half_up`:

```caper
round_half_up(0.5, 0)   // 1
round_half_up(2.5, 0)   // 3
round_half_up(-2.5, 0)  // -3    — se aleja del cero
```

> **Para plata, `round_half_up`.** Con un imponible de $1.005.000 la AFP sale 115.474,5: `round`
> da 115.474 y `round_half_up` da 115.475. Un peso de diferencia en una liquidación es un reclamo.

### Logaritmos — la excepción a la exactitud

Todo el resto del módulo trabaja en decimal exacto. `log`, `log10` y `exp` **no**: no existe un
logaritmo exacto en decimal, así que devuelven una aproximación. Siguen siendo deterministas
—misma entrada, misma salida, siempre— pero no se usan para plata.

```caper
log(8, 2)      // 3 (aprox)
log10(100)     // 2 (aprox)
exp(1)         // 2.7182818... (aprox)
```

### clamp

```caper
clamp(5, 1, 10)    // 5
clamp(50, 1, 10)   // 10
clamp(0, 1, 10)    // 1
```

Un rango invertido (`min > max`) es un error, no un resultado silencioso.
