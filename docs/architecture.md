# Architecture

![Architecture](assets/architecture.svg)

## Core library

`realme-buds` accepts a timed reliable byte stream through the `Transport` trait.

| Module | Responsibility |
| --- | --- |
| `codec` | Bounded AA framing, ULEB lengths, fragments and inner packets |
| `commands` | Known opcodes and capability mappings |
| `model` | Raw-preserving typed records and event decoding |
| `semantics` | Named components, derived noise state, availability and local ANC preference type |
| `semantic_diagnostics` | Named diagnostic/firmware/event views without guessed meanings |
| `client` | Serialized requests, transaction matching, reads/setters, queued events and guarded helpers |
| `advanced` | Enumerated SDK read probes and explicit result/error records |
| `transport/windows` | Native WinSock Bluetooth Classic RFCOMM and remembered-device enumeration |
| `transport/trace` | Opt-in byte tracing; no redaction guarantee |

The client does not automatically replay a setter after a lost acknowledgement. Successful transport delivery, advertised support, readback and observed physical behavior remain distinct.

## Desktop

The background actor receives actions and sends state/events/results to the UI. Bluetooth I/O must not block rendering. Potentially disruptive writes trigger reconnect/readback rather than write replay. Sound actions use cancellation with best-effort stop cleanup.

Local ANC preferences and unsaved EQ editing are UI state, not fabricated earbud properties. Device selection is locked during an active connection or pending action.

Offline demo mode does not create the actor or enumerate devices. It uses synthetic fixtures with no host addresses/names. The send path also refuses hardware actions as a second guard.

## CLI

Arguments map to library calls. Named JSON is the default; `--raw` preserves the lower-level schema. Explicit confirmation flags distinguish ordinary reads from sound, pairing and loudness operations.

## Out of scope

No A2DP audio driver, LHDC codec installer, cloud account client, factory-reset workflow or firmware-image writer is included. Host media/ringing/call integration is not the control protocol itself.
