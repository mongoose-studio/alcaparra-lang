# Ejemplo completo — Liquidación Standard

Script de liquidación mensual básica con gratificación garantizada. Cubre el flujo completo de haberes, descuentos previsionales y cálculo de sueldo líquido.

## Contexto requerido

| Variable | Tipo | Descripción |
|----------|------|-------------|
| `SUELDO_BASE` | número | Sueldo base mensual |
| `DIAS_TRABAJADOS` | número | Días trabajados en el mes |
| `DIAS_HABILES` | número | Días hábiles totales del mes |
| `HORAS_EXTRA_MONTO` | número | Monto calculado de horas extra |
| `AFP_TASA` | número | Tasa AFP del trabajador (ej. `0.1157`) |
| `FONASA_TASA` | número | Tasa de salud (ej. `0.07`) |
| `FONASA_MINIMO` | número | Piso mínimo de cotización salud |
| `TIPO_GRATIFICACION` | string | `"garantizada"` u otro |
| `MESES_ACUMULADOS` | número | Meses trabajados en el año (para gratificación) |
| `TIPO_CONTRATO` | string | `"indefinido"`, `"plazo_fijo"` u otro |
| `UF` | número | Valor UF del período |
| `UTM` | número | Valor UTM del período |

## Script

```caper
// liquidacion_standard.caper
// Liquidación mensual con gratificación garantizada.

use std.math.{ max, min, round };
use std.arrays.{ sum };

header {
    name:    "liquidacion_standard";
    version: "1.0.0";

    // Topes y tasas legales — no sobreescribibles por configuración de tenant
    const TOPE_IMPONIBLE_UF          = 81.0;
    const TOPE_GRATIFICACION_MENSUAL = 4.75;
    const TASA_CESANTIA_INDEFINIDO   = 0.006;
    const TASA_CESANTIA_PLAZO_FIJO   = 0.011;
}

main {
    // -- Validación de contexto mínimo --
    let sueldo_base = SUELDO_BASE ?? 0;
    if sueldo_base <= 0 {
        emit { error: true, codigo: "SUELDO_BASE_INVALIDO" };
    }

    // -- Tope imponible --
    let tope_pesos     = TOPE_IMPONIBLE_UF * UF;
    let base_imponible = min(sueldo_base, tope_pesos);

    // -- Haberes --
    let factor_asistencia = DIAS_TRABAJADOS / DIAS_HABILES;
    let bono_asistencia   = round(sueldo_base * factor_asistencia * 0.05, 0);

    let gratificacion = if TIPO_GRATIFICACION == "garantizada" {
        let proporcional = sueldo_base * 0.25 * MESES_ACUMULADOS / 12;
        round(min(proporcional, TOPE_GRATIFICACION_MENSUAL * UTM / 12), 0)
    } else {
        0
    };

    let horas_extra  = HORAS_EXTRA_MONTO ?? 0;
    let haberes      = [sueldo_base, bono_asistencia, gratificacion, horas_extra];
    let sueldo_bruto = sum(haberes);

    // -- Descuentos previsionales --
    let desc_afp   = round(base_imponible * AFP_TASA, 0);
    let desc_salud = round(max(base_imponible * FONASA_TASA, FONASA_MINIMO), 0);

    let desc_cesantia = match TIPO_CONTRATO {
        "indefinido" => round(base_imponible * TASA_CESANTIA_INDEFINIDO, 0),
        "plazo_fijo" => round(base_imponible * TASA_CESANTIA_PLAZO_FIJO, 0),
        _            => 0
    };

    let descuentos       = [desc_afp, desc_salud, desc_cesantia];
    let total_descuentos = sum(descuentos);

    // -- Resultado --
    emit {
        sueldo_bruto:     sueldo_bruto,
        total_descuentos: total_descuentos,
        sueldo_liquido:   sueldo_bruto - total_descuentos,
        detalle: {
            haberes: {
                sueldo_base:     sueldo_base,
                bono_asistencia: bono_asistencia,
                gratificacion:   gratificacion,
                horas_extra:     horas_extra
            },
            descuentos: {
                afp:      desc_afp,
                salud:    desc_salud,
                cesantia: desc_cesantia
            }
        }
    };
}
```

## Ejecutar

```bash
caper run liquidacion_standard.caper --context '{
    "SUELDO_BASE":      850000,
    "DIAS_TRABAJADOS":  22,
    "DIAS_HABILES":     22,
    "HORAS_EXTRA_MONTO": 0,
    "AFP_TASA":         0.1157,
    "FONASA_TASA":      0.07,
    "FONASA_MINIMO":    30000,
    "TIPO_GRATIFICACION": "garantizada",
    "MESES_ACUMULADOS": 3,
    "TIPO_CONTRATO":    "indefinido",
    "UF":               38234,
    "UTM":              66157
}'
```

## Resultado esperado

```json
{
    "sueldo_bruto": 907125,
    "total_descuentos": 157979,
    "sueldo_liquido": 749146,
    "detalle": {
        "haberes": {
            "sueldo_base": 850000,
            "bono_asistencia": 42500,
            "gratificacion": 14625,
            "horas_extra": 0
        },
        "descuentos": {
            "afp": 98345,
            "salud": 59500,
            "cesantia": 5100
        }
    }
}
```
