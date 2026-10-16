# Adding a device

Only `realme.buds-air7` is implemented. A Bluetooth name or related brand is not evidence that another model shares its protocol. Do not connect an unknown device through the Air7 driver to see whether writes work.

1. Add a module under `crates/realme-buds/src/devices/<model>`. Keep its discovery evidence, framing, opcodes, capability interpretation, records and safety rules there.
2. Choose an explicit stable driver ID and register it in `devices::DRIVER_IDS`. Do not add friendly-name heuristics or an Air7 fallback.
3. Implement against an appropriate transport. Reuse the timed byte-stream trait only if the protocol is actually a reliable stream. A GATT protocol needs its own justified adapter/contract, not fake Air7 packets.
4. Add synthetic/sanitized decoding, malformed-input, range, transaction, partial-state and timeout tests. Preserve unknown values. Do not claim a missing property is Off, zero or unsupported.
5. If exposing the model through FFI, implement the private driver interface and add its factory branch in `crates/realme-buds-ffi/src/driver.rs`. Its request schema belongs in the device module and its responses may differ from Air7. Add an explicit native service mapping only when verified. Leave the C ABI transport/lifetime machinery unchanged.
6. Add model-specific typed C functions only when justified; name them for that driver. Never reinterpret `ob_v1_air7_*` values for a new model. Incompatible foreign layouts need a new ABI version.
7. Record each capability's evidence, host/UI dependencies, untested states and physical limits. Establish safe read baselines before any reversible hardware mutation, read back writes and restore the baseline. Never automatically replay an uncertain write.

Start independent. Extract a genuinely shared codec or helper only after multiple drivers demonstrate the same behavior, including error and malformed-packet cases. Do not parameterize the Air7 client with a growing table of guessed vendor/model exceptions.

The current applications are intentionally Air7-specific. Registering a driver does not automatically add it to their UI. The future frontend should select its model and render that driver's capabilities without inventing unsupported controls.
