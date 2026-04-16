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
