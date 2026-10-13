# Getting started

## Requirements

- realme Buds Air7. Other models are not validated.
- Windows with a working Bluetooth Classic adapter and the earbuds already paired.
- Rust 1.88 or newer; stable is recommended.
- Visual Studio Build Tools: Desktop development with C++, MSVC toolchain and a Windows SDK.

No phone, Android root access, realme account or cloud login is required at runtime.

## Build from source

```powershell
cargo build --release --workspace --locked
.\target\release\buds-desktop.exe
.\target\release\buds.exe --help
```

The command-line binary is named `buds`; the desktop binary is `buds-desktop`. There is currently no promised signed installer. Only run binaries built from source you trust.

## Connect

1. Pair the earbuds through Windows Bluetooth settings.
2. Open the desktop application, select the Air7 and choose Connect.
3. Close competing control clients if the channel is busy.
4. Wait for live readback before editing settings.

Pairing, an audio connection and an RFCOMM control session are separate states. A connected phone app is not a requirement for this desktop program.

## Offline preview

```powershell
cargo run -p buds-desktop -- --demo
```

Demo mode uses deterministic synthetic data and creates no Bluetooth worker or transport. Hardware actions are disabled. It is used for documentation screenshots.

## Before changing sound

Noise-mode writes require a worn ear on the validated firmware. Both ears are required for a successful fit test. Finding requires both earbuds physically removed; never rely solely on sensors.

Read [safety](safety.md). Read [troubleshooting](troubleshooting.md) if Windows pairing works but control does not.
