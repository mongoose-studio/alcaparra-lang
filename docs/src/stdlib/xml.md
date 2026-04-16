# std.xml

```caper
use std.xml.{ xml_encode, xml_encode_opts, xml_decode, xml_valid };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `xml_encode(v, root)` | Serializa un objeto/array a XML con el elemento raíz indicado |
| `xml_encode_opts(v, root, opts)` | Igual con opciones: `indent`, `namespace`, `declaration` |
| `xml_decode(s)` | Parsea XML → objeto AlcaparraLang |
| `xml_valid(s)` | `true` si el string es XML bien formado |
| `xml_get(s, xpath)` | Extrae un valor usando XPath simple |
| `xml_get_all(s, xpath)` | Extrae array de coincidencias XPath |

## Ejemplos

```caper
use std.xml.{ xml_encode_opts, xml_decode, xml_valid };

// Generar XML para integración con SII o Previred
let documento = {
    RUT:     RUT,
    Monto:   TOTAL_LIQUIDO,
    Periodo: PERIODO
};

let xml = xml_encode_opts(documento, "Liquidacion", {
    indent:      true,
    declaration: true,
    namespace:   "http://sii.cl/schema/dt"
});

emit xml;
/*
<?xml version="1.0" encoding="UTF-8"?>
<Liquidacion xmlns="http://sii.cl/schema/dt">
  <RUT>12.345.678-9</RUT>
  <Monto>850000</Monto>
  <Periodo>2026-04</Periodo>
</Liquidacion>
*/

// Leer XML recibido
let obj = xml_decode(XML_INPUT);
let rut = obj.RUT;
```

> El elemento raíz (`root`) es obligatorio — a diferencia de JSON, XML siempre necesita un elemento contenedor. `xml_encode` lanzará `TYPE_ERROR` si no se especifica.
