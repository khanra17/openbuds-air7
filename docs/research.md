# Interoperability research

OpenBuds uses an independently written implementation informed by Bluetooth captures, reversible readback checks and headset SDK interoperability analysis. Only protocol facts and sanitized fixtures are distributed—not APKs, decompiled vendor source, firmware images or raw capture files.

## Evidence rules

- Scope each finding to a model and firmware version.
- Distinguish an advertised opcode from a successful query, readback, and physical behavior.
- Record starting settings, perform one reversible change, verify it and restore it.
- Keep unknown IDs/payloads and failed reads explicit. Never invent defaults to complete a screen.
- An acknowledgement can be lost after a successful write. Do not replay setters automatically.
- Document UI-only choices and host requirements separately from transport data.

Verified Air7 findings and open branches are in [coverage](coverage.md), [state model](state-model.md) and [protocol](protocol.md).

## Capturing responsibly

Use equipment and software you are authorized to inspect. A dedicated Bluetooth capture can include unrelated device identifiers and audio traffic. Use the smallest useful capture and keep it private.

If using Android HCI logging, record the prior logging configuration, enable it only for the bounded investigation, save the necessary material locally, restore logging settings and verify capture has stopped. Restarting Bluetooth can interrupt audio. A full bugreport usually exposes more unrelated data than a targeted capture.

Private experiments belong outside the repository's tracked tree. Do not commit device-specific JSON, screenshots of paired-host lists, traces, APKs, firmware or local build helpers.

## Sanitizing fixtures

Replace addresses, host/preset names and other identities with clearly synthetic data. Keep length fields and payload schemas consistent after substitution. Explain the transformation. Unit tests must operate on a mocked transport and must not discover/connect devices or generate sound.

Hex payloads can hide identifiers even when text searches find none. Review decoded payloads and byte order as well as printable strings. Publication checks are defense in depth, not proof of anonymization.

## Hardware safety

Never play finding while an earbud is worn. Do not enable volume enhancement during automated wearing tests. Avoid pairing deletion, factory reset, calibration/profile writes or undocumented opcodes when restoration is uncertain. Firmware flashing is excluded.

Share facts and minimal sanitized reproductions. Do not submit proprietary code excerpts, credentials or raw captures in public issues. Seek appropriate legal advice if the scope of reverse engineering or redistribution is uncertain in your jurisdiction.
