// SPDX-License-Identifier: MIT
// AlcaparraLang — deterministic scripting language
// Copyright (c) 2026-present Marcel Rojas

//! Topes de **forma** del programa: hasta dónde puede anidar y encadenar un script
//! antes de que el motor se niegue a seguir.
//!
//! No son topes de rendimiento ni de gusto. Son lo único que impide que un script
//! de un kilobyte y medio mate el proceso entero: el parser, el compilador y el
//! ejecutor recorren el árbol **recursivamente**, y un desbordamiento de pila en
//! Rust no es un `panic` sino un `SIGABRT`. No hay `catch_unwind` que lo contenga,
//! no hay error que devolver, y se lleva por delante todo lo que el proceso
//! estuviera haciendo en paralelo.
//!
//! # Por qué los valores por omisión son estos
//!
//! Cada etapa tiene su propio techo, y el techo depende de dos cosas que el motor
//! **no controla**: el tamaño de pila del hilo y el perfil de compilación. Medido
//! sobre este mismo crate:
//!
//! | caso | dónde muere | 2 MiB release | 2 MiB debug | 64 MiB release | 64 MiB debug |
//! |---|---|---|---|---|---|
//! | `[[[…1…]]]` | parser     | 464   | **105** | 14.924  | 3.433 |
//! | `1 + 1 + …`  | compilador | 5.453 | **259** | 174.755 | 8.304 |
//!
//! Los dos números en negrita son el peor caso realista: un hilo de tokio o de
//! prueba trae 2 MiB, y `cargo test` compila en `debug`. Los valores por omisión
//! se eligen debajo de esos: 64 niveles de anidamiento contra un techo de 105, y
//! 192 términos encadenados contra uno de 259.
//!
//! # Por qué son configurables y no constantes
//!
//! Porque la respuesta correcta depende del empotrador. Un servicio que le da
//! 64 MiB de pila a sus trabajadores puede permitir mucho más que un hilo de
//! prueba, y un script **generado** —un reporte que arma la fórmula pegando
//! columnas— encadena bastante más que uno escrito a mano. Un número fijo tenía
//! que ser inseguro para uno o una regresión para el otro, así que no hay número
//! fijo: hay un valor por omisión conservador y un `Limits` que se puede subir
//! **sabiendo lo que se está moviendo**, que es la pila del hilo donde corre.

/// Topes de forma con los que se parsea y se compila un script.
///
/// `Default` trae los valores seguros en la peor configuración realista. Subirlos
/// es legítimo, pero hay que hacerlo junto con la pila del hilo — ver la tabla de
/// medición al principio de este módulo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Niveles de anidamiento sintáctico: `[[[…]]]`, `{a:{b:{c:…}}}`, `if` dentro
    /// de `if`, closures dentro de closures. Lo cuenta el parser, que es la etapa
    /// con el techo más bajo.
    pub max_nesting_depth: usize,

    /// Términos encadenados en una misma expresión: `a + b + c + …`. El parser la
    /// arma **iterativamente**, así que no la ve; el que recursa un marco por
    /// término es el compilador, y por eso tiene su propio tope.
    pub max_operator_chain: usize,

    /// Niveles de anidamiento de un **valor**: el contexto que inyecta el host y
    /// el valor que el script emite. No lo produce el texto del script, así que
    /// no lo cubren los dos de arriba.
    pub max_value_depth: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_nesting_depth:  64,
            max_operator_chain: 192,
            max_value_depth:    64,
        }
    }
}

impl Limits {
    /// Topes por omisión. Equivale a `Limits::default()`, pero se puede usar en
    /// contexto `const`.
    pub const DEFAULT: Self = Self {
        max_nesting_depth:  64,
        max_operator_chain: 192,
        max_value_depth:    64,
    };
}

/// Profundidad de anidamiento de un valor, acotada: deja de contar apenas pasa
/// `limit`, así que no recorre de más.
///
/// **Sin recursión, a propósito.** Es la única forma de poder preguntar «¿esto es
/// demasiado hondo?» sin que la pregunta misma desborde la pila — que es
/// exactamente lo que pasaría con un recorrido recursivo sobre un valor que viene
/// de afuera y que nadie revisó todavía.
///
/// Devuelve `Err(depth)` con la profundidad que se alcanzó al pasarse, y `Ok(())`
/// si el valor cabe.
pub fn check_value_depth(value: &crate::vm::value::Value, limit: usize) -> Result<(), usize> {
    use crate::vm::value::Value;

    // (valor, profundidad a la que está). Pila explícita en el heap.
    let mut pending: Vec<(&Value, usize)> = vec![(value, 1)];

    while let Some((v, depth)) = pending.pop() {
        if depth > limit {
            return Err(depth);
        }
        match v {
            Value::Array(items) => {
                for item in items {
                    pending.push((item, depth + 1));
                }
            }
            Value::Object(fields) => {
                for item in fields.values() {
                    pending.push((item, depth + 1));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::value::Value;
    use rust_decimal::Decimal;

    fn nested(levels: usize) -> Value {
        let mut v = Value::Number(Decimal::ONE);
        for _ in 0..levels {
            v = Value::Array(vec![v]);
        }
        v
    }

    #[test]
    fn a_flat_value_has_depth_one() {
        assert!(check_value_depth(&Value::Number(Decimal::ONE), 1).is_ok());
    }

    #[test]
    fn nesting_is_counted_level_by_level() {
        // `nested(9)` son 9 arreglos más el número de adentro: 10 niveles.
        assert!(check_value_depth(&nested(9), 10).is_ok());
        assert_eq!(check_value_depth(&nested(9), 9), Err(10));
    }

    /// Un valor que desbordaría la pila de cualquier recorrido recursivo tiene que
    /// poder revisarse igual. Si esta prueba aborta en vez de fallar, el recorrido
    /// volvió a ser recursivo.
    #[test]
    fn a_value_far_deeper_than_the_stack_can_still_be_checked() {
        let v = nested(200_000);
        assert!(check_value_depth(&v, 64).is_err());

        // `forget` y no `drop`, y esto es un agujero conocido, no una comodidad:
        // el `Drop` de `Value` es el que **deriva** el compilador, o sea recursivo,
        // así que soltar un valor así aborta el proceso con `SIGABRT`. Dentro del
        // motor ya no puede existir uno —`push_built` lo impide— pero un host que
        // arme el valor a mano sí puede, y para eso existe esta función: para que
        // pueda preguntar antes de entregarlo. Arreglarlo de raíz pide un `Drop`
        // iterativo, y eso no se puede implementar sobre `Value` sin romper todo
        // patrón que lo desarme por valor.
        std::mem::forget(v);
    }
}
