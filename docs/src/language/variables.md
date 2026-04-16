# Variables y tipos

## Declaración

```caper
let nombre   = "Juan";    // inmutable — no se puede reasignar
var contador = 0;         // mutable — puede reasignarse
```

```caper
var total = 0.0;
total = total + 1000;     // OK

nombre = "Pedro";         // ERROR: ImmutableReassignment
```

Redeclarar una variable en el mismo scope también es un error:

```caper
let x = 1;
let x = 2;    // ERROR: VariableRedeclaration
```

El shadowing en scopes anidados sí está permitido:

```caper
let x = 1;
if condicion {
    let x = 99;   // OK — scope distinto
    emit x;       // 99
}
emit x;           // 1
```

## Tipos de datos

| Tipo | Ejemplo | Notas |
|------|---------|-------|
| Número | `42`, `3.14`, `-100`, `1_000_000` | Siempre `Decimal` — sin float64 |
| String | `"texto"`, `'texto'` | Comillas simples o dobles equivalentes |
| Booleano | `true`, `false` | |
| Null | `null` | Ausencia de valor |
| Array | `[1, 2, 3]` | Indexado desde 0 |
| Objeto | `{ clave: valor }` | Claves string, orden de inserción preservado |

### Números exactos

El tipo `Decimal` usa precisión arbitraria — nunca hay errores de punto flotante en cálculos monetarios:

```caper
let a = 0.1 + 0.2;   // 0.3 (exacto) — no 0.30000000000000004
```

Los guiones bajos en literales numéricos son ignorados (separador visual):

```caper
let tope = 1_000_000;   // equivalente a 1000000
```

## Null coalescing (`??`)

El operador `??` retorna el lado derecho si el izquierdo es `null`:

```caper
let bono = BONO_ESPECIAL ?? 0;

// Encadenado
let config = A ?? B ?? C ?? "default";
```

Es la forma idiomática de dar valores por defecto a variables de contexto que podrían no estar presentes.

## Inyección de contexto

Las variables externas se inyectan automáticamente en el scope base del script. No hay que declararlas:

```caper
// SUELDO_BASE, DIAS_TRABAJADOS, AFP_TASA, UF, UTM, etc.
// están disponibles directamente si el caller los inyectó

let bruto = SUELDO_BASE + (BONO_ASISTENCIA ?? 0);
```

Si una variable del contexto no fue inyectada, su valor es `null`. Usa `??` para el caso seguro.

**Prioridad del scope:**

```
const del header  >  let/var del script  >  contexto inyectado
```

Una `const` nunca puede ser sobreescrita por el contexto externo.
