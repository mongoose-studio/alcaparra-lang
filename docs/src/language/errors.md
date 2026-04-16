# Manejo de errores

## try / catch

`try/catch` captura cualquier error — tanto los lanzados con `throw` como los errores internos del runtime:

```caper
// Como expresión con valor
let tasa = try {
    tasas_afp[AFP_CODIGO];
} catch e {
    0.1157    // valor por defecto si la clave no existe
};            // ; al final porque es una expresión asignada

// Como sentencia
try {
    let resultado = operacion_riesgosa();
    emit resultado;
} catch e {
    emit { error: true, mensaje: e.message };
}
```

El objeto `e` siempre tiene dos campos:

| Campo | Tipo | Descripción |
|-------|------|-------------|
| `e.code` | string | Código del error. Para `throw` con objeto: el `code` del usuario. Para errores internos: el código de runtime (`"DIVISION_BY_ZERO"`, etc.). |
| `e.message` | string | Mensaje legible. |

## throw

`throw` termina la ejecución del bloque actual lanzando una excepción:

```caper
// Con mensaje simple
throw "El sueldo base no puede ser cero";

// Con objeto estructurado (recomendado)
throw { code: "SUELDO_INVALIDO", message: "El sueldo base debe ser positivo" };
```

### throw en funciones

```caper
fn validar_sueldo(base) {
    if base <= 0 {
        throw { code: "VALIDACION", message: "sueldo debe ser positivo" };
    }
    emit base;
}

let r = try {
    emit validar_sueldo(-1);
} catch e {
    emit e.code + ": " + e.message;   // "VALIDACION: sueldo debe ser positivo"
};
```

### Excepción no capturada

Si un `throw` no está dentro de ningún `try/catch`, el script termina con error:

```json
{
    "status": "error",
    "error": {
        "type":    "USER_THROW",
        "code":    "SUELDO_INVALIDO",
        "message": "El sueldo base debe ser positivo",
        "line":    12
    }
}
```

Para errores internos del runtime (división por cero, tipo incorrecto, etc.):

```json
{
    "status": "error",
    "error": {
        "type":    "DIVISION_BY_ZERO",
        "message": "Division by zero at line 8"
    }
}
```

## Códigos de error internos

| Código | Descripción |
|--------|-------------|
| `UNDEFINED_VARIABLE` | Variable usada sin declarar ni inyectar |
| `TYPE_ERROR` | Operación sobre tipo incorrecto |
| `DIVISION_BY_ZERO` | División entre cero |
| `CONST_REASSIGNMENT` | Intento de reasignar una `const` |
| `IMMUTABLE_REASSIGNMENT` | Intento de reasignar un `let` |
| `VARIABLE_REDECLARATION` | Variable declarada dos veces en el mismo scope |
| `INDEX_OUT_OF_BOUNDS` | Acceso a índice fuera del rango del array |
| `KEY_NOT_FOUND` | Acceso a campo inexistente en un objeto (con operador de acceso directo) |
| `STACK_OVERFLOW` | Recursión que excede el límite de llamadas |
| `LOOP_LIMIT_EXCEEDED` | Loop que excede el límite de iteraciones |
| `CIRCULAR_IMPORT` | Importación circular detectada |
| `IMPORT_NOT_FOUND` | Archivo de import no encontrado |
