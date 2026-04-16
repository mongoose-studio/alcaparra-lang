# Arrays y objetos

## Arrays

```caper
let haberes = [SUELDO_BASE, BONO_ASISTENCIA, HORAS_EXTRA_MONTO];
let vacio   = [];

// Tipos mixtos (permitido)
let mixto = [1, "dos", true, null, 3.14];

// Arrays de arrays (matrices)
let tramos = [
    [0,       800_000,   0.0],
    [800_001, 1_700_000, 0.04],
    [1_700_001, null,    0.08]
];
```

### Acceso

```caper
let primero  = haberes[0];       // índice base 0
let ultimo   = haberes[-1];      // índice negativo: desde el final
let subtotal = haberes[1..3];    // rango: elementos 1 y 2 (3 excluido)
```

### Spread

```caper
let base      = [100, 200];
let extendido = [...base, 300];   // [100, 200, 300]
```

## Objetos

```caper
let empleado = {
    nombre:  NOMBRE,
    rut:     RUT,
    jornada: HORAS_CONTRATO,
    activo:  true
};
```

### Acceso

```caper
let nombre = empleado.nombre;      // dot notation
let rut    = empleado["rut"];      // equivalente con string dinámico
```

### Objetos anidados

```caper
let contrato = {
    empleado: empleado,
    tipo:     TIPO_CONTRATO,
    inicio:   FECHA_INICIO
};

let nombre_en_contrato = contrato.empleado.nombre;
```

### Spread en objeto

```caper
let base_obj  = { nombre: "Juan", activo: true };
let extendido = { ...base_obj, cargo: "Analista" };
// { nombre: "Juan", activo: true, cargo: "Analista" }
// Si hay clave duplicada, la más a la derecha prevalece.
```

### Mutabilidad

Los objetos `let` son completamente inmutables — ningún campo puede modificarse:

```caper
let empleado = { nombre: "Juan" };
empleado.nombre = "Pedro";    // ERROR: ImmutableReassignment
```

Los objetos `var` permiten mutación de campos individuales:

```caper
var resultado = {};
resultado.sueldo_bruto     = sum(haberes);
resultado.total_descuentos = sum(descuentos);
resultado.liquido          = resultado.sueldo_bruto - resultado.total_descuentos;
```
