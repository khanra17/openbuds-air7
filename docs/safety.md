# Safety

## Sound

Never start finding while an earbud is in an ear. The signal can be loud. Remove both buds physically and leave them out of the case; sensor flags alone are not reliable proof of removal.

Fit tests intentionally play a sound with the buds worn. Use a quiet environment and stop if uncomfortable. Do not enable volume enhancement during automated wearing tests.

## State and connection changes

- Preserve starting values and verify writes through readback.
- A write can take effect even if its acknowledgement is lost. Never blindly retry.
- Codec/dual-device changes can disconnect a host. Reconnect and inspect state.
- Pairing removal loses that host's pairing; it is not an ordinary disconnect.
- Missing/offline data is not an Off value or an empty battery.

## Cleanup limits

Finding/fit helpers are bounded and attempt stop cleanup on cancellation, rejection and timeout. The GUI cancels an active sound operation before allowing a normal close. Forced termination, power loss, disconnection or failed stop acknowledgement can prevent cleanup.

## Excluded actions

No firmware flashing, factory reset or destructive earbud unpairing workflow is implemented or performed by tests. Raw APIs are research escape hatches, not permission to replay unknown opcodes.

Read the [state audit](state-model.md) for unverified branches before using a setting as a safety-critical signal.
