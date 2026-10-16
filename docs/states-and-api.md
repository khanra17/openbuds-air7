# States and API use

```rust,no_run
use realme_buds::devices::air7::{Client, Component, wearing_summary, CONTROL_SERVICE};
use realme_buds_windows::RfcommStream;
fn main() -> Result<(),Box<dyn std::error::Error>> {
    let address = std::env::var("BUDS_ADDRESS")?;
    let mut buds=Client::new(RfcommStream::connect_service(&address, CONTROL_SERVICE)?);
    buds.initialize()?;
    let s=buds.snapshot();
    println!("{:?}",s.wearing_summary());
    if let Some(right)=s.battery_for(Component::Right) {
        println!("Right reported percentage: {:?}; raw={:02x}",right.percent,right.raw);
    } else {
        println!("No right battery observation was returned");
    }
    println!("{:?}",wearing_summary(&buds.wear()?));
    Ok(())
}
```

## Observed wearing setups

| Setup supplied by user | Left flags | Right flags | Derived scenario |
| --- | --- | --- | --- |
| Both out, neither worn | 01 | 01 | Neither |
| Right worn, left out | 01 | 03 | RightOnly |
| Left worn, right in closed case | 03 | 00 | LeftOnly |
| Both worn | 03 | 03 | Both |

Raw flags: bit 0 clear → in-case; bit 1 set → worn; bit 2 clear → connected flag. The case component uses the same raw SDK structure, but its `in_case`/`worn` bits should not be interpreted as a physical case-lid sensor.

In the left-worn/right-in-case setup, the battery reply contained left=100 and case=0, **no right entry**. The wear reply still included right-in-case and its connection flag. The library preserves both observations rather than manufacturing a right percentage or deducing battery availability from the connection bit.

## Named semantics versus raw data

`Snapshot::describe()` and the shared reading/event descriptions attach component, gesture, function and state names without losing raw IDs. `NoiseState::describe()` derives the category and ANC-enable meaning from the single current-mode value. It returns no active ANC level while Off/Transparency; `AncPreference` is separate host/UI memory. `set_anc_enabled()` requires an explicit level on enable and emits the existing mode setter, not a guessed new command. `set_transparency_enabled(false)` selects Off.

The CLI defaults to these descriptions; `--raw` retains the original schema for diagnostic scripts. API users can still use the original structs. See [state-model.md](state-model.md) for all known feature boundaries, label corrections, persistence ownership and unresolved branches.

## Advanced reads

- `firmware_info()` reads versions, hardware strings, vendor/product IDs, protocol bytes and upgrade-capability flags. It does not write firmware.
- `read_property(ReadProperty)` probes a fixed, enumerated set of SDK read commands. Some reads bypass bitmap gating because the app itself queries them without a capability bit.
- `property_audit()` returns data or an explicit error for each read. A timeout is not mapped to zero or an “off” setting.
- `status_features(ids)` collects multiple responses that use the earbuds' notification counter rather than the request transaction ID. Returned IDs can be a subset of requested IDs.
- `features_by_ids(ids)` and `set_feature_id(id,bool)` support SDK flags not given a user-facing name. The setter first requires the firmware to return that ID.
- `send_unacknowledged()` is for SDK commands lacking a standard response. Successful delivery is **not** confirmation that a setting took effect; query state afterwards.
- `request()` provides capability-checked raw request/response access. It is a diagnostic escape hatch, not permission to guess destructive/firmware opcodes.

Opaque profile data is intentionally not converted into guessed acoustic or biometric meanings. Firmware caches may report old information; the protocol does not supply measurement freshness for every value.

## Event handling and safety

`next_event()` exposes battery, wear, noise, fit results, EQ, multipoint, diagnostic features, use-time, user-interaction and host-operation events. Unknown messages remain `Event::Raw`.

`run_fit_test()` collects results and sends stop on rejection/timeout. Its cancellable variant checks a callback between bounded reads. Single-ear tests returned status 14 on the actual device. A both-worn hardware test completed after about nine seconds with `[left=1, right=1]`, meaning good fit for both, followed by a successful stop acknowledgement. All settings matched the starting snapshot afterwards.

`FitResult::outcome()` maps raw result 0 to general fit, 1 to good fit, 3 to detection failure, 4 to interruption by wearing changes, and 5 to interruption by the app. Other codes remain `Unknown(code)`; raw `result` is always preserved. Codes 0/1 are final verdicts, not progress. A captured two-good-fit packet is a regression fixture. Early notifications arriving before the start acknowledgement are retained; waiting for that acknowledgement is cancellable too.

`find_buds(true)` checks that both earbuds are reported connected, out of case and not worn. `find_buds_for()` also sends stop after a bounded duration. The wearer must still confirm physical removal: sensors can be stale or incorrect. Forced process termination, disconnection, or power loss can prevent cleanup. Never run finding while an earbud is worn.

No writes are automatically replayed after a timeout or lost acknowledgement. The GUI reconnects after potentially disruptive codec/multipoint changes and reads the result; it does not replay the setter.
