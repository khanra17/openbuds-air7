# OpenBuds Air7

Direct Bluetooth Classic control for realme Buds Air7. No phone bridge or cloud account.

```text
crates/realme-buds           Rust library: device protocols, state and commands
crates/realme-buds-windows   Windows RFCOMM transport
bindings/c                  C-compatible SDK consuming the Rust crates
apps/cli                    CLI consuming the Rust crates
apps/desktop                Desktop app consuming the Rust crates
```

Only Air7 is implemented. Other models need their own drivers; Air7 opcodes and modes are not vendor-wide defaults. Android is not included. Native Bluetooth is Windows-only; other hosts can supply a timed stream.

## Use

[Prebuilt downloads](https://github.com/khanra17/openbuds-air7/releases/tag/v0.2.0) include separate applications and native SDKs. SDK consumers need a matching shared library and header, not Rust. See [the C binding](bindings/c/README.md).

To build from source, use Rust 1.88+. Windows builds need MSVC C++ tools and a Windows SDK.

```sh
cargo build -p buds-cli -p buds-desktop --release --locked
```

Pair the Air7 in Windows Settings, then run `target/release/buds-desktop.exe` or the CLI:

```sh
buds status
buds anc on --level moderate
buds anc off
buds --help
```

Rust applications can depend directly on `realme-buds` and, for native Windows transport, `realme-buds-windows`:

```rust
use realme_buds::devices::air7::{Client, CONTROL_SERVICE};
use realme_buds_windows::RfcommStream;

let stream = RfcommStream::connect_service(&address, CONTROL_SERVICE)?;
let mut buds = Client::new(stream);
buds.initialize()?;
let state = buds.snapshot().describe();
```

## Safety and state

- Never play finding tones while either earbud is worn. Check physical removal; sensors can be stale.
- A timed-out write may have succeeded. Reconnect and read before retrying; setters are not replayed automatically.
- Acknowledgement, stored readback and physical behavior are different evidence. See [Air7 behavior and limits](docs/air7.md).
- Status exports and wire logs can contain device addresses, host names and preset names.
- Firmware writing and factory reset are not implemented.

## Check

```sh
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo fmt --all -- --check
```

## License

[GPL-3.0-or-later](LICENSE), including the C SDK. Dependencies retain their own [licenses](THIRD_PARTY_NOTICES.md). This is an unofficial project; product names identify compatibility. GPL grants no trademark rights or legal immunity.
