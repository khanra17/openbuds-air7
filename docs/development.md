# Development

## Workspace

Use stable Rust (minimum supported version 1.88) and the MSVC C++ tools/Windows SDK for the native applications. Dependencies are pinned in Cargo.lock.

```powershell
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo doc --workspace --no-deps --locked
```

On non-Windows platforms, the transport-independent core tests can run without Bluetooth hardware:

```sh
cargo test -p realme-buds --locked
cargo clippy -p realme-buds --all-targets --locked -- -D warnings
```

Linux/macOS native Bluetooth backends are not implemented. The desktop supports an offline preview; a successful GUI build is not proof of a working native Bluetooth backend.

## Testing policy

Tests use synthetic/sanitized data and mocked transports. CI never enables audio, touches physical devices, deletes pairings or writes firmware.

Regression coverage includes stream chunking/coalescing, fragmentation, transaction wrap/matching, malformed inputs, captured length quirks, partial state, named semantics, range validation and bounded/cancellable helpers. Offline demo tests ensure the preview carries no host identities.

A hardware result is recorded separately in the [coverage ledger](coverage.md). Do not describe a mocked test as a live verification.

## Module boundaries

- Wire bytes and semantic state belong in `realme-buds`.
- GUI `worker.rs` owns the Bluetooth client and runs outside the UI thread.
- GUI `app.rs` owns presentation/local preferences; `demo.rs` uses synthetic state only.
- CLI `args.rs` owns parsing; `commands.rs` delegates operations to the library.
- Never implement a second protocol codec in a frontend.

## Publication checks

```sh
python scripts/check_repository.py
```

This maintained gate checks tracked files for forbidden artifacts, device-address literals, local user paths, credential patterns, image metadata and broken local Markdown references. Synthetic test-address exceptions are narrowly listed. It is a defense in depth, not a guarantee that an arbitrary screenshot/log is safe.

## Releases

The release workflow builds Windows artifacts from a version tag, includes the GPL text and dependency-license information, and publishes checksums. Do not publish a binary built from a private research tree or unreviewed source. Audit licensing for any newly added dependency or asset.
