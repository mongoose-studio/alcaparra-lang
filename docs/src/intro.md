# AlcaparraLang

**AlcaparraLang** es un lenguaje de scripting interpretado, propósito general, con fuerte foco en cálculos numéricos. Su caso de uso principal es la evaluación de fórmulas financieras y de nómina, aunque está diseñado para ser completamente independiente de ese dominio.

```caper
use std.math.{ round, min };

header {
    name: "liquidacion_standard";
    version: "1.0.0";

    const TASA_AFP      = 0.1157;
    const TOPE_IMPONIBLE = 81.0;
}

main {
    let bruto     = SUELDO_BASE ?? 0;
    let desc_afp  = round(bruto * TASA_AFP, 0);
    let liquido   = bruto - desc_afp;

    emit {
        bruto:    bruto,
        desc_afp: desc_afp,
        liquido:  liquido
    };
}
```

## Características principales

- **Números exactos** — usa `Decimal` de precisión arbitraria internamente. Sin errores de punto flotante en cálculos monetarios.
- **Mutabilidad explícita** — `let` (inmutable), `var` (mutable), `const` (constante de script en header).
- **Inyección de contexto** — las variables del entorno están disponibles directamente en el scope, sin declaración.
- **`emit` como salida** — equivalente a `return` pero orientado a producir el resultado final del script.
- **Determinista** — sin acceso a red, sistema de archivos ni estado global entre ejecuciones.
- **Embebible en Rust** — el crate `alcaparra` expone `Interpreter::run()` para integración directa.

## Instalación del CLI

```bash
cargo install alcaparra-lang
```

```bash
caper run mi_script.caper --context '{"SUELDO_BASE": 850000}'
```

## Licencia

MIT — [github.com/mongoose-studio/alcaparra-lang](https://github.com/mongoose-studio/alcaparra-lang)
