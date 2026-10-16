# Development

Use stable Rust (minimum supported version 1.88). Native Windows applications need MSVC C++ tools and a Windows SDK. Cargo.lock pins dependencies.

## SDK only

The workspace's default members are the library crates (core, platform adapter and FFI), not the frontends. These tests use synthetic streams on Windows, Linux and macOS; they do not operate Bluetooth hardware.

```sh
cargo fmt --all -- --check
cargo test --all-features --locked
cargo clippy --all-targets --all-features --locked -- -D warnings
cargo doc --no-deps --all-features --locked
cargo build -p realme-buds-ffi --release --locked
```

To exercise the Rust SDK without its optional JSON feature:

```sh
cargo test -p realme-buds --no-default-features --locked
```

Windows RFCOMM lives in `crates/realme-buds-windows`, separate from `crates/realme-buds`. Linux/macOS hosts supply stream callbacks; successful builds do not establish native Bluetooth support. See [architecture](architecture.md) and [adding devices](adding-devices.md).

## Existing applications

```powershell
cargo test --workspace --all-features --locked
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo build -p buds-cli -p buds-desktop --locked
cargo doc --workspace --no-deps --all-features --locked
```

`apps/desktop/worker.rs` owns the client outside the UI thread; `app.rs` owns presentation/local preferences; `demo.rs` contains synthetic state. CLI arguments/output live under `apps/cli`. Neither frontend owns a second codec. Applications remain Air7-specific.

## Native consumer tests and packages

The [C example](../examples/c/abi_smoke.c) compiles against the public header and shared library without Cargo. It tests synthetic callback I/O, partial reads/writes, reentrant-call rejection, typed noise, structured requests, ownership and errors. The [Python example](../examples/python/openbuds.py) loads the binary using only ctypes. SDK CI builds, packages and runs both consumers on each native host.

For a maintainer-built SDK archive, build an explicit target and generate notices:

```sh
cargo build -p realme-buds-ffi --release --locked --target YOUR_NATIVE_TARGET
cargo install cargo-about --version 0.9.2 --locked --features cli
cargo about generate about.hbs --output-file target/THIRD_PARTY_LICENSES.html
python scripts/package_sdk.py --target YOUR_NATIVE_TARGET --notices target/THIRD_PARTY_LICENSES.html
```

Replace `YOUR_NATIVE_TARGET` with the `host` reported by `rustc -vV`. Packages carry that target, version and exact public Git revision. Do not distribute an archive from a dirty/private tree as if it matched the recorded revision. Cross-compiling an Android callback library is possible with a suitable NDK toolchain, but no Android binaries/JNI bridge are currently released.

## Testing and evidence

Tests cover framing/chunking, transactions, malformed data, protocol quirks, partial state, named semantics, safety guards, strict JSON requests, FFI null/length/version checks, callback count validation, stale handles, thread-local errors, busy/cancel/close behavior and panic containment/cleanup. Demo tests refuse hardware actions.

Hardware results belong separately in the [coverage ledger](coverage.md). New ABI tests are not new physical validation. Do not enable sound, remove pairings or flash firmware in CI. Never automatically replay an uncertain setter.

## Publication and releases

```sh
python -m unittest discover -s scripts/tests -v
python scripts/check_repository.py
```

The publication gate checks staged/tracked files for forbidden artifacts, addresses, local user paths, credentials, image metadata and broken local Markdown links. Review screenshots and dependency licenses separately.

CI uploads candidate native SDK archives. A matching version tag runs the same SDK pipeline and publishes separate native SDKs, Windows applications, checksums and audited corresponding source. It does not bundle private research or require consumers to install Rust. See [FFI contract](ffi.md) and [licensing](legal.md).
