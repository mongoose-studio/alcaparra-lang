# Funciones y closures

## Declaración

```caper
fn calcular_gratificacion(sueldo_base, meses_trabajados) {
    let base_anual   = sueldo_base * 12;
    let proporcional = base_anual * 0.25 * meses_trabajados / 12;
    let tope         = TOPE_GRATIFICACION_MENSUAL * UTM / 12;

    emit min(proporcional, tope);
}
```

`emit` dentro de una función es el equivalente a `return`. Si no hay `emit`, la función retorna `null`.

## Llamada

```caper
let grat = calcular_gratificacion(SUELDO_BASE, MESES_TRABAJADOS);
```

## Recursión

```caper
fn factorial(n) {
    if n <= 1 {
        emit 1;
    } else {
        emit n * factorial(n - 1);
    }
}

let resultado = factorial(5);   // 120
```

## Closures (funciones anónimas)

```caper
// Closure de expresión única
let calcular_afp = |sueldo| => sueldo * AFP_TASA;

// Closure con cuerpo
let aplicar_descuento = |monto, porcentaje| => {
    let descuento = monto * porcentaje;
    emit monto - descuento;
};

// Como argumento directo (HOF)
let montos_netos = map(haberes, |h| => h * 0.9);

let mayores_a_cien = filter(valores, |v| => v > 100);

let total = reduce(haberes, |acc, h| => acc + h, 0);
```

## emit como return de función

A diferencia de otros lenguajes, `emit` no solo produce la salida del script — también funciona como `return` dentro de funciones:

```caper
fn clasificar(n) {
    if n < 0     { emit "negativo"; }
    if n == 0    { emit "cero"; }
    emit "positivo";
}

let clase = clasificar(-5);   // "negativo"
```

El primer `emit` ejecutado dentro de la función termina su ejecución.
