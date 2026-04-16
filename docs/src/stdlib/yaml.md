# std.yaml

```caper
use std.yaml.{ yaml_encode, yaml_decode, yaml_valid };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `yaml_encode(v)` | Serializa cualquier valor a YAML |
| `yaml_encode_opts(v, opts)` | Con opciones: `indent` (default 2), `flow_style` |
| `yaml_decode(s)` | Parsea YAML → valor AlcaparraLang |
| `yaml_valid(s)` | `true` si el string es YAML válido |

## Ejemplos

```caper
use std.yaml.{ yaml_encode, yaml_decode };

let config = {
    servicio:    "alcaparra-service",
    version:     "1.0.0",
    parametros:  { uf: UF, utm: UTM }
};

emit yaml_encode(config);
/*
servicio: alcaparra-service
version: 1.0.0
parametros:
  uf: 38234.56
  utm: 66157
*/

// Leer configuración YAML inyectada como string
let cfg = yaml_decode(CONFIG_YAML ?? "{}");
let tasa = cfg.tasa_bono ?? 0;
```
