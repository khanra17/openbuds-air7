# realme-buds

The platform-neutral Rust SDK for [OpenBuds Air7](../../README.md), licensed GPL-3.0-or-later. It does not depend on a desktop UI or native Windows Bluetooth APIs.

## Explicit device modules

The only implemented driver is `realme_buds::devices::air7`, ID `realme.buds-air7`. Its framing, opcodes, capability checks, raw-preserving records, semantics, client, diagnostics and safety helpers are scoped to that module. They are not a vendor-wide contract.

The SDK root contains `devices`, `transport` and `Transport`. Default features are empty; optional `json` adds the Air7 structured-request dispatcher used by FFI. There are no root-level Air7 compatibility aliases in 0.2.

```rust,no_run
use realme_buds::devices::air7::{Client, Component, CONTROL_SERVICE};
use realme_buds_windows::RfcommStream;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::var("BUDS_ADDRESS")?;
    let stream = RfcommStream::connect_service(&address, CONTROL_SERVICE)?;
    let mut buds = Client::new(stream);
    buds.initialize()?;
    let state = buds.snapshot();
    println!("{:#?}", state.describe());
    if let Some(battery) = state.battery_for(Component::Right) {
        println!("Reported right percentage: {:?}", battery.percent);
    }
    Ok(())
}
```

The example additionally requires the separate Windows adapter crate and Windows. Other hosts can implement the generic timed reliable `Read + Write + Send` transport contract. Native Linux/macOS Bluetooth adapters are not included.

For languages without Rust, use the independent [native C SDK](../../docs/ffi.md). Its shared library is distributed separately from applications. Consumers do not need Cargo or a Rust installation.

## Guarantees and limits

No automatic write retry: a timeout may follow a successful mutation. Read state before retrying. Missing data remains absent, unavailable percentages keep raw bytes, unknown events remain packets, and diagnostic failures stay failures. Acknowledgement is not stored readback or proof of physical behavior.

The SDK does not implement an audio driver, phone ringing, cloud services or firmware-image writing. Its lower-level raw API is not a safety guarantee for arbitrary commands. See [state semantics](../../docs/state-model.md), [protocol](../../docs/protocol.md) and [safety](../../docs/safety.md).

## Testing and extension

```sh
cargo test -p realme-buds --all-features --locked
```

Tests are offline and use synthetic/sanitized data. New models need independent, evidenced drivers; see [adding devices](../../docs/adding-devices.md).
