# Estructura de un script

Un script `.caper` puede tener dos formas: con `header` y `main`, o directo (inline).

## Con header y main

```caper
header {
    name:        "liquidacion_standard";
    version:     "1.2.0";
    description: "Liquidacion mensual con gratificacion garantizada";
    author:      "equipo-nomina";

    // Constantes: disponibles en main, nunca sobreescribibles por el contexto
    const TOPE_IMPONIBLE_UF          = 81.0;
    const TASA_CESANTIA_INDEFINIDO   = 0.006;
    const TASA_CESANTIA_PLAZO_FIJO   = 0.011;
    const TOPE_GRATIFICACION_MENSUAL = 4.75;
}

main {
    let bruto = SUELDO_BASE + (BONO_ASISTENCIA ?? 0);
    emit { resultado: bruto };
}
```

El bloque `header` es opcional pero recomendado para scripts nombrados y versionados.

## Inline (sin header)

```caper
// El intérprete ejecuta el contenido directamente, como si fuera main
let sueldo_bruto = SUELDO_BASE + BONO_ASISTENCIA;
emit { sueldo_bruto: sueldo_bruto };
```

Útil para scripts cortos o evaluaciones directas desde API.

## Constantes del header (`const`)

Las constantes declaradas en `header` tienen **máxima precedencia**:

- No se pueden reasignar desde `main`
- No pueden ser sobreescritas por el contexto inyectado
- Están disponibles en todo el scope de `main` y en las funciones del script

```caper
header {
    name: "calculos_legales";
    version: "2.0.0";

    const UF_TOPE_IMPONIBLE = 81.0;
}

main {
    let tope = UF_TOPE_IMPONIBLE * UF;   // UF viene del contexto, la const es del script

    UF_TOPE_IMPONIBLE = 90.0;            // ERROR: ConstReassignment
}
```

> Las constantes son ideales para valores legales que el operador **no debe poder cambiar** desde configuración externa — tasas y topes que emanan de la ley.

## Comentarios

```caper
// Comentario de línea

/*
   Comentario de bloque.
   Puede ocupar múltiples líneas.
*/

let x = 10; // comentario al final de una línea
```

## Punto y coma

Toda **sentencia** termina obligatoriamente en `;`. Los **bloques** `{ }` no llevan `;` después de la llave de cierre, excepto cuando el bloque es parte de una expresión asignada.

```caper
// Sentencias simples — siempre ;
let x = 5;
var total = 0;
use std.math.round;
emit { valor: x };

// Bloques de control — sin ; después de }
if x > 0 {
    total = total + x;
}

foreach item in lista {
    total = total + item;
}

// Bloque como expresión asignada — el ; va al final
let categoria = match sueldo {
    n if n < 500_000 => "basico",
    _                => "alto"
};

let mensaje = if activo { "vigente" } else { "inactivo" };
```
