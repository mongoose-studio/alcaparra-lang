# Primeros pasos

## Instalación

```bash
cargo install alcaparra-lang
```

Verifica la instalación:

```bash
caper version
# AlcaparraLang 0.1.0
```

## Tu primer script

Crea un archivo `hola.caper`:

```caper
let nombre = NOMBRE ?? "mundo";
emit "Hola, " + nombre + "!";
```

Ejecútalo:

```bash
caper run hola.caper
# "Hola, mundo!"

caper run hola.caper --context '{"NOMBRE": "Marcel"}'
# "Hola, Marcel!"
```

## Script con cálculo

```caper
// descuento.caper
use std.math.round;

let bruto    = SUELDO ?? 0;
let tasa     = TASA   ?? 0.1157;
let descuento = round(bruto * tasa, 0);

emit {
    bruto:     bruto,
    descuento: descuento,
    liquido:   bruto - descuento
};
```

```bash
caper run descuento.caper --context '{"SUELDO": 850000, "TASA": 0.1157}'
# { "bruto": 850000, "descuento": 98345, "liquido": 751655 }
```

## Modo proyecto

Crea un proyecto con `.capercfg`:

```bash
caper new mi-proyecto
cd mi-proyecto
caper run
```

El archivo `.capercfg` generado:

```json
{
    "name":    "mi-proyecto",
    "version": "1.0.0",
    "entry":   "main.caper",
    "context": {}
}
```

## REPL interactivo

```bash
caper repl
```

```
alcaparra> let x = 10 * 3;
alcaparra> emit x;
30
alcaparra> exit
```

## Validar sintaxis

```bash
caper validate mi_script.caper
# OK — sin errores
```
