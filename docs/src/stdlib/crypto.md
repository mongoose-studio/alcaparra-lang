# std.crypto

```caper
use std.crypto.{ sha256, md5, base64_encode, base64_decode };
```

## Funciones

| Función | Descripción |
|---------|-------------|
| `md5(s)` | Hash MD5 → hex string de 32 caracteres |
| `sha1(s)` | Hash SHA-1 → hex string de 40 caracteres |
| `sha256(s)` | Hash SHA-256 → hex string de 64 caracteres |
| `sha512(s)` | Hash SHA-512 → hex string de 128 caracteres |
| `hmac_sha256(data, key)` | HMAC-SHA256 → hex string |
| `base64_encode(s)` | Codifica a Base64 |
| `base64_decode(s)` | Decodifica desde Base64 |
| `base64_url_encode(s)` | Codifica a Base64 URL-safe (sin `+`, `/`, `=`) |
| `hex_encode(s)` | Codifica a hex string |
| `hex_decode(s)` | Decodifica desde hex string |

## Ejemplos

```caper
use std.crypto.{ sha256, md5, base64_encode, hmac_sha256 };
use std.json.json_encode;

// Huella de configuración para auditoría
let formula_hash = sha256(json_encode(FORMULAS_CONFIG));

// Cache key única por empleado y período
let cache_key = md5(RUT + "-" + PERIODO + "-" + str(SUELDO_BASE));

// Payload para integración externa
let payload_b64 = base64_encode(json_encode({
    rut:    RUT,
    monto:  TOTAL_LIQUIDO,
    firma:  sha256(RUT + PERIODO + str(TOTAL_LIQUIDO))
}));

// Firma HMAC para webhook
let firma = hmac_sha256(json_encode(PAYLOAD), API_SECRET);
```

> **Nota de seguridad:** MD5 y SHA-1 están incluidos por utilidad general (checksums, cache keys). Para firmas o autenticación usar SHA-256 o superior.
