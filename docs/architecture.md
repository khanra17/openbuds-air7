# Architecture

![Architecture](assets/architecture.svg)

## Boundaries

| Path | Owns | Does not own |
| --- | --- | --- |
| `crates/realme-buds` | Platform-neutral SDK, explicit device modules, transport contract | GUI dependencies, native Bluetooth discovery |
| `crates/realme-buds/src/devices/air7` | Air7 framing, opcodes, records, semantics, commands and optional JSON API | Other Realme models' protocols |
| `crates/realme-buds-windows` | Paired-device enumeration and RFCOMM sockets for an explicit service UUID | Air7 service defaults, device modes or packet decoding |
| `crates/realme-buds-ffi` | Versioned C ABI, driver selection, callback adaptation, handles, errors and buffers | GUI behavior, a universal Realme command schema |
| `apps/cli` | Arguments, output formatting and library delegation | Wire protocol implementations |
| `apps/desktop` | Presentation, local preferences, Bluetooth worker and offline demo | Shared device semantics |

The SDK's default feature set has no JSON dispatcher and no native Bluetooth backend. `Transport` is a timed, reliable `Read + Write + Send` stream. `ServiceUuid` is a generic transport parameter, not a model identifier. Optional tracing records bytes and is not a redaction layer.

## Device drivers

Rust callers choose `devices::air7::Client`. The stable driver ID is `realme.buds-air7`; it is not inferred from a friendly Bluetooth name. All current packet quirks, capability mappings, mode codes and named records live below that device module. Root-level Air7 compatibility aliases were deliberately removed in 0.2.

There is no speculative universal headset trait requiring future devices to share Air7's ANC, EQ, wire format or identifiers. A new driver may use different records and framing. Similar-looking opcodes are not evidence of compatibility. See [adding devices](adding-devices.md).

## Foreign boundary

The C ABI exposes opaque integer sessions and owned byte buffers, not Rust layouts. A small private driver interface erases model types only at this boundary: initialize, timeout, structured request and cancellation. It does not standardize wire commands or returned state. Air7's strict optional JSON schema lives in its SDK module, not in the FFI frontend. Direct POD convenience functions are explicitly named `ob_v1_air7_*` and reject another driver.

Caller-provided C transport callbacks support other host runtimes. The Windows convenience constructor selects the service through an explicit model mapping before invoking the platform adapter. Unknown drivers/services have no fallback. Android can supply a stream through a future JNI/NDK bridge; no Android app, JNI wrapper or Android prebuilt is currently shipped.

Calls run on the invoking thread. A per-session lock prevents concurrent/reentrant I/O; cancellation is a separate atomic signal. Close never frees an in-flight session. Buffers and session IDs are checked, non-reused handles. Panics are contained at the ABI boundary; a poisoned session can still be closed. Foreign callbacks themselves must not unwind. See [FFI](ffi.md).

## Applications

The existing applications remain Air7-specific. Future redesigned desktop/Android applications can use either the typed SDK or the native ABI without depending on these frontends.

The desktop worker owns the Bluetooth client outside the UI thread. Reconnect/readback is distinct from replaying writes. Local ANC preferences and unsaved EQ editing are UI state, not earbud properties. Offline demo mode creates no Bluetooth worker and refuses hardware actions.

The CLI delegates device operations to the SDK. Named JSON is its default output; `--raw` preserves the original records. It does not need to route through the C ABI.

## Evidence and exclusions

Transport delivery, advertised support, acknowledgement, readback and physical behavior remain separate evidence levels. No setter is automatically replayed after a lost acknowledgement. Only Air7 has hardware evidence; the restructuring and ABI tests use synthetic streams, not new physical validation.

No audio driver, codec installer, cloud account client, factory-reset workflow or firmware writer is included. Host media/ringing/call integration is not the control protocol itself.
