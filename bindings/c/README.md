# C SDK

Use `openbuds.dll`, `libopenbuds.so` or `libopenbuds.dylib` with [include/openbuds.h](include/openbuds.h). [Prebuilt SDKs](https://github.com/khanra17/openbuds-air7/releases/tag/v0.2.0) require no Rust installation. Match the library to the process architecture.

To build the library itself:

```sh
cargo build -p realme-buds-ffi --release --locked
```

Select `realme.buds-air7`, open a session, initialize it, then issue calls. Windows can enumerate paired devices and open RFCOMM directly; other hosts supply stream callbacks. The catalog includes the service UUID in network byte order. Unknown models have no fallback.

The header defines buffer ownership, callback lifetimes, threading and error rules. [tests/smoke.c](tests/smoke.c) is a standalone C consumer using a synthetic stream, not Bluetooth hardware. No Rust types cross the ABI.

## Air7 JSON requests

`ob_v1_call` accepts UTF-8 JSON. Operations use snake_case; enum values use the spelling below. Unknown operations and top-level fields are rejected. Nested records follow the Rust [model types](../../crates/realme-buds/src/devices/air7/model.rs).

```json
{"op":"noise"}
{"op":"set_anc","enabled":true,"level":"Moderate"}
{"op":"set_feature","feature":"GameMode","enabled":true}
```

Each line is a separate request. Responses contain `schema_version: 1`, `driver` and `data`. A setter's `data: null` means acknowledgement, not automatic readback. Snapshots and diagnostics can report individual read failures.

| Operation | Fields after `op` |
| --- | --- |
| `initialize`, `snapshot`, `battery`, `versions`, `wear`, `features` | none |
| `feature_ids`, `status_features`, `stored_data` | `ids`: byte array |
| `set_feature` | `feature`: Feature enum; `enabled`: boolean |
| `set_feature_id` | `id`: byte; `enabled`; `confirm_experimental: true` |
| `noise`, `noise_cycle`, `touch`, `eq_preset`, `custom_eq`, `dynamic_bands`, `volume_enhancer`, `multipoint` | none |
| `set_noise` | `mode`: Off / Transparency / Mild / Moderate / Maximum / Adaptive |
| `set_anc` | `enabled`; `level`: Mild / Moderate / Maximum / Adaptive, required when enabling |
| `set_transparency` | `enabled` |
| `set_noise_cycle` | `cycle`: OffTransparency / OffCancellation / CancellationTransparency / All |
| `set_touch` | `bindings`: TouchBinding array |
| `set_eq_preset` | `id`: byte |
| `set_custom_eq` | `eq`: CustomEq; `action`: Add / Update / Delete |
| `set_dynamic_band` | `band`: DynamicBand |
| `set_volume_enhancer` | `enabled`; `confirm_louder`: true required when enabling |
| `multipoint_action` | `address`: six bytes; `action`: byte; `confirm: true` |
| `subscribe`, `stop_fit_test`, `stop_finding`, `firmware`, `diagnostics`, `use_time` | none |
| `next_event` | `timeout_ms`: nonzero u32 |
| `run_fit_test` | `timeout_ms`: 1–30000; `confirm_sound: true` |
| `find_for` | `duration_ms`: 1–10000; `confirm_sound: true`; both earbuds physically removed |
| `read_property` | `property`: ReadProperty enum |
| `set_auto_power_off` | `minutes`: byte; `confirm_experimental: true` |
| `raw_read` | `command`: allowlisted read opcode; `payload`: bytes |

No raw writes or unbounded sound-start requests are exposed. Cancellation applies to bounded fit/finding helpers; it is not a hardware watchdog. See [Air7 limits](../../docs/air7.md) before changing settings.
