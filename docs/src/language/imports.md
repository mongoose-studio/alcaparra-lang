# Imports y proyectos

## Stdlib

```caper
use std.math;                              // importa todo el módulo
use std.arrays.{ sum, avg, filter };       // funciones específicas
use std.strings.upper;                     // una sola función
use std.dates;
```

Las funciones importadas están disponibles directamente por nombre en el resto del script.

## Librerías propias (multi-archivo)

Los scripts `.caper` pueden importar funciones de otros archivos `.caper`. El archivo importado opera en **library mode**: solo se exportan las funciones declaradas con `fn`. Las sentencias `emit` en librerías son ignoradas.

```caper
// Importar funciones específicas
use formulas.gratificacion.{ calcular, tope };
use lib.validaciones.{ validar_positivo, validar_rango };

// Usar con normalidad
let grat = calcular(SUELDO_BASE, DIAS_TRABAJADOS);
```

**Resolución de rutas:** `formulas.gratificacion` → `formulas/gratificacion.caper` relativo al directorio base del script principal.

## Path aliases

Define aliases en `.capercfg` para rutas largas o compartidas (similar a `paths` en `tsconfig.json`):

```json
{
    "paths": {
        "@lib":      "./lib",
        "@formulas": "/opt/formulas-nacion"
    }
}
```

```caper
use @lib.validaciones.{ validar_positivo };
use @formulas.afp.{ calcular_descuento };
```

## Paquetes de terceros (`vendors/`)

Los paquetes instalados con `caper install` se resuelven desde `vendors/`:

```caper
use @mongoose/nomina-cl.afp.{ calcular_descuento };
use mongoose/utils.fechas.{ es_feriado };
```

```bash
# Instalar un paquete local o git
caper install /ruta/local/mi-paquete
caper install git+https://github.com/org/paquete --tag v1.0.0
```

## Proyecto — `.capercfg`

Un directorio con `.capercfg` es un proyecto. Permite correr `caper run` sin especificar script:

```json
{
    "name":        "liquidacion-mensual",
    "version":     "1.0.0",
    "description": "Fórmulas de liquidación estándar",
    "author":      "Marcel Rojas",
    "entry":       "main.caper",
    "context": {
        "UF":  36000,
        "UTM": 65000
    },
    "paths": {
        "@lib":      "./lib",
        "@formulas": "./formulas"
    }
}
```

| Campo | Default | Descripción |
|-------|---------|-------------|
| `entry` | `"main.caper"` | Script de entrada |
| `context` | `{}` | Variables de contexto base |
| `paths` | `{}` | Aliases de rutas para imports |

**Precedencia de contexto:** `--context` CLI > `context` en `.capercfg`.

**Búsqueda del config:** el CLI sube por el árbol de directorios hasta encontrar `.capercfg`. Si no existe, termina con error.

## Comportamiento de imports

- **Cache:** un archivo importado por múltiples scripts en la misma ejecución se carga una sola vez.
- **Circular imports:** `A → B → A` lanza `CIRCULAR_IMPORT` inmediatamente.
- **Library mode:** el `header` de un archivo importado es privado para ese archivo. Solo sus `fn` son exportadas.
