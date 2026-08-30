# FFI Error Codes (`FfiStatus`)

> **SSOT** — auto-generated from `ffi/c-binding/src/lib.rs`.
> Do not edit manually. Regenerate: `python scripts/gen_ssot_docs.py --generate`

Stable ABI error codes returned at the C FFI boundary (`FfiStatus`).

| Code | Name | Description |
|------|------|-------------|
| 0 | `Ok` | Success / no error |
| -1 | `NullPointer` | A required pointer argument was null |
| -2 | `InvalidParameter` | Parameter validation failed |
| -3 | `InsufficientData` | Input series too short for the requested calculation |
| -4 | `InternalError` | Internal error or panic caught at FFI boundary |
| -5 | `InvalidUtf8` | Invalid UTF-8 in a string argument |
| -99 | `Unknown` | Unclassified error |

## Regenerate

```bash
python scripts/gen_ssot_docs.py --generate
python scripts/gen_ssot_docs.py --check
```
