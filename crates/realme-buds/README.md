# realme-buds

The transport-independent protocol core of [OpenBuds Air7](../../README.md), licensed GPL-3.0-or-later.

## API layers

- `codec`: frame/packet encoding, bounded incremental decoding and known firmware quirks.
- `Client<T>`: request matching, capability checks, reads/setters, queued notifications.
- Raw-preserving typed records: battery, wear, noise, EQ, features, bindings and hosts.
- `describe()` helpers: shared named state without guessed readings.
- `advanced`: enumerated SDK read probes, firmware metadata and explicit failures.
- `Transport`: timed `Read + Write + Send`; native Windows RFCOMM is included.

```rust,no_run
use realme_buds::{Client, Component, transport::windows::RfcommStream};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let address = std::env::var("BUDS_ADDRESS")?;
    let mut buds = Client::new(RfcommStream::connect(&address)?);
    buds.initialize()?;
    let state = buds.snapshot();
    println!("{:#?}", state.describe());
    if let Some(battery) = state.battery_for(Component::Right) {
        println!("Reported right percentage: {:?}", battery.percent);
    }
    Ok(())
}
```

The Windows backend is conditional. Other platforms may provide their own reliable timed transport; no native Linux/macOS backend is currently supplied.

## Guarantees and limits

No automatic write retry. A timeout may follow a successful mutation. Read state before retrying. Missing data remains absent, unavailable percentages keep raw bytes, unknown events remain packets, and diagnostic failures stay failures.

The library does not implement an audio driver, phone ringing, cloud services or firmware-image writing. A raw API is not a safety guarantee for arbitrary opcodes. See [state semantics](../../docs/state-model.md), [protocol](../../docs/protocol.md) and [safety](../../docs/safety.md).

## Testing

```sh
cargo test -p realme-buds --locked
```

Tests are offline and use synthetic/sanitized data. No physical device is operated.
