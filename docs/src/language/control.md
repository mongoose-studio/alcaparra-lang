# Control de flujo

## if / else if / else

```caper
if sueldo_base > 1_000_000 {
    var categoria = "alto";
} else if sueldo_base > 500_000 {
    var categoria = "medio";
} else {
    var categoria = "basico";
}
```

### if como expresión

```caper
let jornada = if horas_contrato >= 45 { "completa" } else { "parcial" };
```

## match

```caper
// Sobre valor exacto
let desc = match TIPO_CONTRATO {
    "indefinido" => "Contrato a Plazo Indefinido",
    "plazo_fijo" => "Contrato a Plazo Fijo",
    "honorarios" => "Boleta de Honorarios",
    _            => "Tipo no reconocido"
};
```

### match con guards

```caper
let tramo = match sueldo_bruto {
    n if n <= 800_000   => 0.0,
    n if n <= 1_700_000 => 0.04,
    n if n <= 2_500_000 => 0.08,
    n if n <= 3_300_000 => 0.135,
    _                   => 0.23
};
```

Los patrones se evalúan en orden. El primero que coincide gana. `_` es el caso por defecto.

## while

```caper
var i    = 0;
var suma = 0;

while (i < count(haberes)) {
    suma = suma + haberes[i];
    i = i + 1;
}
```

## foreach

```caper
// Iterar sobre un array
foreach haber in haberes {
    total = total + haber;
}

// Con índice numérico
foreach i, haber in haberes {
    // i: 0, 1, 2...  |  haber: el valor
}

// Iterar sobre un objeto
let tasas = { "habitat": 0.1157, "cuprum": 0.1044 };

foreach codigo, tasa in tasas {
    // codigo: "habitat", "cuprum"...  |  tasa: el valor numérico
}
```

## break y continue

Disponibles en `while` y `foreach`:

```caper
foreach item in lista {
    if item < 0 { continue; }    // salta al siguiente
    if item > 1000 { break; }    // termina el loop
    total = total + item;
}
```
