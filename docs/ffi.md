# Native SDK and C ABI v1

The SDK shared library is independent of the desktop/CLI and can be loaded by C, C++, Python ctypes, C# P/Invoke, Go cgo or a language's native bridge. Consumers do not need Rust. The typed Rust SDK remains a separate option.

Download a matching `openbuds-sdk-*` archive from [releases](https://github.com/khanra17/openbuds-air7/releases). CI also uploads candidate SDK archives. Each package has a header, shared library, Windows import library where applicable, examples, license report, checksum and source-revision manifest. No Android binary or JNI wrapper is shipped yet. Linux/macOS libraries accept callbacks but do not implement native Bluetooth.

## Lifecycle

1. Load the binary matching your process architecture; require `ob_v1_abi_version() == 1`.
2. Select a driver explicitly. `ob_v1_driver_catalog` is offline metadata, not Bluetooth discovery.
3. Open `realme.buds-air7` using `ob_v1_open_windows`, or provide a timed reliable byte stream through `ob_v1_open` callbacks.
4. Initialize with `ob_v1_initialize` or the driver's `initialize` request.
5. Execute commands on a worker thread. Read back mutations; never retry an uncertain write automatically.
6. Release every output using `ob_v1_buffer_release`. Close the session before destroying callbacks or unloading the library.

See the authoritative [C header](../include/openbuds.h), [standalone C/CMake consumer](../examples/c/abi_smoke.c) and [Python consumer](../examples/python/openbuds.py). The Python example defaults to an offline catalog read; `--address` opts into Windows hardware access.

Pointers must identify valid, aligned storage for their declared regions. The library checks nulls, lengths, ABI versions and handles, but cannot make an arbitrary forged C pointer safe. Strings/JSON are UTF-8 bytes with explicit lengths, **not NUL-terminated strings**. Outputs must not already own an allocation. IDs are opaque and non-reused; copied buffer descriptors must not be released or accessed twice. There is no caller-side `free()` for Rust allocations.

Calls are synchronous and serialize per handle. Concurrent/reentrant operations return `OB_BUSY`, not a queued retry. Context/callback code must remain live until successful close. Callbacks may run on whichever thread invoked the SDK; either support that or keep the session on one owning thread. Never retain callback byte pointers, throw exceptions or unwind across C. A successful close invokes release exactly once and finishes it before returning. Model/table validation failures do not take the context lease; after validation, construction failure also releases the lease.

`ob_v1_cancel` signals an active bounded fit/finding helper without taking the session lock. It does not abort every read or act as a hardware watchdog. A busy close leaves the handle intact: cancel/wait, then close again. A contained Rust panic makes the session unusable but still closable. Process termination, memory corruption, allocation aborts and a foreign callback's unwinding cannot be recovered by the ABI.

## Errors

Every fallible function returns a fixed-width status from the header. Retrieve `ob_v1_last_error` immediately on the same thread before another SDK call. It copies a UTF-8 message and structured status/command/device-status fields; release the message buffer. Successful calls clear that thread's previous error. The getter returns the copied error even though its own success clears the stored slot.

Timeout, transport failure, rejected command, unsupported capability, malformed protocol, bad arguments and cancellation remain distinct. Callback timeout maps to timed I/O; other callback failures are transport errors. The original callback's arbitrary failure integer is included in its message, not misinterpreted as an earbud status.

## Air7 request schema 1

This is the `realme.buds-air7` schema, **not a Realme-wide contract**. Rust declarations are in [the Air7 API module](../crates/realme-buds/src/devices/air7/api.rs). The SDK `json` feature is optional for Rust; the FFI crate enables it.

```json
{"op":"noise"}
{"op":"set_anc","enabled":true,"level":"Moderate"}
{"op":"set_feature","feature":"GameMode","enabled":true}
{"op":"snapshot"}
```

Each line above is a separate request. `op` uses snake_case; enum values use the Rust enum spelling. Unknown operations and top-level request fields are rejected, including extra fields on parameterless reads. Nested raw records follow their SDK struct definitions. Responses use `{"schema_version":1,"driver":"realme.buds-air7","data":...}`. Ordinary acknowledged setters return `data: null`: this is **not** automatic stored readback or proof of physical effect. Snapshot/diagnostic records can contain per-property failures despite successful overall dispatch; inspect their availability/error fields.

| Operations | Additional request fields |
| --- | --- |
| `initialize`, `snapshot`, `battery`, `versions`, `wear`, `features` | none |
| `feature_ids`, `status_features`, `stored_data` | `ids`: byte array |
| `set_feature` | `feature`: named Feature enum; `enabled`: boolean |
| `set_feature_id` | `id`: byte; `enabled`; `confirm_experimental`: boolean, must be true |
| `noise`, `noise_cycle`, `touch`, `eq_preset`, `custom_eq`, `dynamic_bands`, `volume_enhancer`, `multipoint` | none |
| `set_noise` | `mode`: `Off`, `Transparency`, `Mild`, `Moderate`, `Maximum`, `Adaptive` |
| `set_anc` | `enabled`; `level`: `Mild`, `Moderate`, `Maximum`, `Adaptive` or null; enabling requires a level |
| `set_transparency` | `enabled` |
| `set_noise_cycle` | `cycle`: `OffTransparency`, `OffCancellation`, `CancellationTransparency`, `All` |
| `set_touch` | `bindings`: array of raw-preserving TouchBinding records |
| `set_eq_preset` | `id`: byte |
| `set_custom_eq` | `eq`: CustomEq record; `action`: EqAction enum |
| `set_dynamic_band` | `band`: DynamicBand record |
| `set_volume_enhancer` | `enabled`; `confirm_louder`: boolean, true required when enabling |
| `multipoint_action` | `address`: six-byte array; `action`: byte; `confirm`: boolean, must be true |
| `subscribe`, `stop_fit_test`, `stop_finding`, `firmware`, `diagnostics`, `use_time` | none |
| `next_event` | `timeout_ms`: nonzero u32 |
| `run_fit_test` | `timeout_ms`: 1–30000; `confirm_sound`: true |
| `find_for` | `duration_ms`: 1–10000; `confirm_sound`: true; both earbuds must be physically removed |
| `read_property` | `property`: enumerated ReadProperty |
| `set_auto_power_off` | `minutes`: byte; `confirm_experimental`: true |
| `raw_read` | `command`: u16 from an explicit read-only allowlist; `payload`: byte array |

Raw writes, firmware writes, factory reset and standalone unbounded sound-start commands are not exposed by this schema. Rust's lower-level raw API is intentionally a different trust boundary. Unknown reported wire values remain raw instead of being assigned invented names. Direct non-JSON POD functions currently cover Air7 noise reads, noise writes and named feature writes; their names and codes are explicitly model-scoped.

## Future hosts and drivers

An Android frontend can implement Bluetooth ownership, permissions and a JNI/NDK shim, then supply stream callbacks. Java thread attachment and JNI lifetimes belong to that shim; never retain one thread's JNI environment for another. This is an integration path, not a claim of a completed Android client. Other native Bluetooth transports can be supplied without changing Air7 framing.

A new model gets its own driver, wire semantics and request/response schema. It must not inherit Air7 codes through these ABI convenience functions. See [adding devices](adding-devices.md).

GPL-3.0-or-later covers the bridge and SDK, not just the GUI. FFI does not create a permissive licensing exception. See [legal guidance](legal.md) and the package license notices.
