# Contributing

Small, reviewable changes are welcome. Read the [development guide](docs/development.md), [architecture](docs/architecture.md) and [state audit](docs/state-model.md) first.

## Before opening a pull request

1. Describe the problem and the scope of the change.
2. Keep protocol/semantic logic in the library; frontends should delegate.
3. Add a regression test for the changed behavior.
4. Run fmt, tests, strict clippy and the publication/privacy gate.
5. Distinguish mocked coverage from live hardware evidence.
6. Check dependency/asset licenses and include required notices.

No automatic hardware tests: CI must not connect to physical earbuds, play audio, remove pairings or write firmware. Do not expand raw opcode probing without evidence and safety review.

## Protocol contributions

Share sanitized minimal packets, direction, command, firmware and before/after state. Preserve framing lengths when replacing bytes. State uncertainty explicitly: an advertised bit or stored readback is not proof of physical behavior.

Never submit proprietary APK/decompiled code, firmware images, raw HCI logs, device addresses, personal host/preset names or session artifacts.

## UI contributions

Handle missing/unknown/partial observations without inventing defaults. Identify UI-only state and persistence ownership. Use the offline demo for screenshots; do not photograph a live device list.

## Licensing and conduct

Contributions must be yours to license under GPL-3.0-or-later, with compatible third-party material clearly identified. No copyright assignment is required. Discuss respectfully, focus on reproducible technical evidence, and do not post anyone's personal information.

Use [security reporting](SECURITY.md) for sensitive vulnerabilities, not public issues.
