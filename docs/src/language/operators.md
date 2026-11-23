# Operadores

## Aritméticos

```caper
let suma     = a + b;
let resta    = a - b;
let producto = a * b;
let division = a / b;    // siempre Decimal, nunca división entera
let modulo   = a % b;    // resto de la división
let potencia = a ** b;   // a elevado a b
```

### Los operadores aritméticos no convierten

Un string **no** es un número, aunque contenga dígitos. `"100" * 2` es un error de tipo, no 200:

```caper
let s = "1850000";     // ojo: texto, no número

let mal  = s * 26;              // TYPE_ERROR
let bien = to_number(s) * 26;   // 48100000
```

Esto es a propósito, y es de las decisiones que más protegen un cálculo. Un contexto JSON trae
números como string con facilidad —PostgreSQL serializa `numeric` así, y muchos lenguajes lo
arrastran hasta el JSON—. Cuando la conversión era automática, `"1850000" * 26 / 30` daba un
resultado correcto y la primera suma lo concatenaba: el cálculo salía mal **sin un solo error**.
Mejor que reviente en la primera operación.

Lo mismo vale para los booleanos: `true` no es 1.

Para convertir a propósito están [`to_number`](../stdlib/types.md) y
[`to_string`](../stdlib/types.md), que es justamente lo que hacen.

## Comparación

```caper
a == b    // igual
a != b    // distinto
a > b
a < b
a >= b
a <= b
```

## Lógicos

```caper
a and b    // AND lógico
a or b     // OR lógico
not a      // NOT lógico
```

## String

```caper
let saludo = "Hola " + nombre;    // concatenación con +
```

Cualquier valor puede concatenarse con un string — se convierte automáticamente:

```caper
let msg = "Total: " + 850000;     // "Total: 850000"
```

> **`+` es la excepción.** Es a la vez suma y concatenación, así que cuando **cualquiera** de los
> dos lados es string el resultado es un string, no un error:
>
> ```caper
> let a = 100 + "50";    // "10050", no 150
> ```
>
> Es lo que permite escribir mensajes de error sin convertir a mano, pero significa que un
> acumulador numérico que se sume con un string del contexto se vuelve texto en silencio. Si el
> valor puede venir como string, pásalo por `to_number` antes de acumularlo.

## Acceso a colecciones

```caper
arr[0]         // acceso por índice base 0
arr[-1]        // último elemento (índice negativo)
arr[1..3]      // rango: elementos 1 y 2 (3 excluido)

obj.clave      // acceso a objeto por nombre de campo
obj["clave"]   // acceso por string dinámico (equivalente)
```

## Null coalescing

```caper
valor ?? fallback    // retorna fallback si valor es null
```

## Precedencia

De mayor a menor precedencia:

| Nivel | Operadores |
|-------|-----------|
| 1 (mayor) | `not`, `-` (unario) |
| 2 | `**` |
| 3 | `*`, `/`, `%` |
| 4 | `+`, `-` |
| 5 | `>`, `<`, `>=`, `<=` |
| 6 | `==`, `!=` |
| 7 | `and` |
| 8 | `or` |
| 9 (menor) | `??` |

Usa paréntesis para forzar cualquier orden de evaluación.
