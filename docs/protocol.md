# Buds Air7 protocol

## Evidence

Observed on realme Buds Air7 firmware **1.1.0.62**, using a rooted Android 10 phone running realme Link **5.8.811.2814** (version code 530214) and the Windows laptop's own Bluetooth adapter. Command IDs and layouts were cross-checked against the app's headset SDK; setters were verified by readback where listed in [coverage](coverage.md). Decompiled app code is not distributed.

The service's SDP record advertises:

- Bluetooth Classic RFCOMM, L2CAP PSM `0x0003`.
- Vendor service UUID `0000079a-d102-11e1-9b23-00025b00a5a5`.
- Standard SPP `00001101-0000-1000-8000-00805f9b34fb` also occurs in the app, but Air7 uses the vendor UUID. Do not substitute BLE GATT.
- Captured RFCOMM channel: 15. The Windows backend resolves the UUID through SDP rather than hard-coding the channel.

Control traffic is distinct from A2DP audio. Changing a codec flag on the earbuds does not change the codecs supported by the laptop's audio stack.

## Framing

```text
AA | length:ULEB128 | control:u8 | reserved:u8 | [fragment:u8] | inner
```

`length` counts every byte after itself. Captured unfragmented control/reserved bytes are `00 00`. Control's low two bits encode single/first/middle/last as 0/1/2/3. Fragment numbers begin at 0. The decoder retains partial headers and payloads across stream reads, accepts coalesced frames, and bounds buffering/assembly.

```text
inner = command:u16LE | transaction:u8 | payload_length:u16LE | payload
```

The transaction counter wraps 255 → 0. Responses normally use `command | 0x8000`, echo the transaction, and start with a status byte (`0` = success). Requests are serialized. A timeout is **not** proof the write failed, so no setter is automatically retried.

Example captured dynamic-audio enable:

```text
aa09000003042702001d01
         0304 27 0200 1d01
         cmd  tx len  feature29 / enabled1
response: aa080000038427010000
```

### Dynamic acknowledgement quirk

Firmware 1.1.0.62 sent this literal response on the laptop:

```text
aa0900001b840301000001
         1b84 03 0100 00 01
```

The inner header declares one payload byte, but the correct outer length includes **two**: status `00` and opaque metadata `01`. The same `01` was returned for **all three bands**, so it must not be treated as an echoed band ID. The codec accepts only this exact successful `841b` shape. `0508` notifications and `012c` readback identify the actual band.

### Other captured firmware quirks

- Language reply `8120` declares length `0100` (256) but has two bytes: status 0 and language code 3. The correct outer length is used only for this narrowly recognized shape.
- Diagnostic status reply `8129` declares only `5 × count`, omitting its leading status/count bytes. One query yields multiple packets using the **device's event counter**, not the request's transaction. `status_features()` collects that known response stream until quiet and preserves the returned subset of IDs.
- Detection reply `812a` declares 5 but sends 6 bytes: status, feature ID, u32LE value.
- Cycle notification `0204`/event 3 includes `[action,type,value,00]`; the trailing zero is reserved. It is not present in the normal `810c` query reply.

All other length mismatches remain errors. These exceptions are backed by packet captures and regression tests, not a blanket decision to ignore malformed data.

## Commands

Response data below excludes the leading status byte.

| Read | Setter | Data |
| --- | --- | --- |
| `0100` capabilities | — | Bitmap; bit-to-command mapping in `commands.rs` |
| `0101` host MTU negotiation | — | Send `00 02` (512); peer returned `c0 08` (2240) |
| `0102` frame/vendor exchange | — | Advertise receive frame size 1946; vendor ID returned as u16LE |
| `0103` product ID | — | u24LE |
| `0104` protocol version | — | Returned bytes `00 00 01` |
| `0107` upgrade capability | — | Count + component/flag pairs; **read only** |
| `010b` color code | — | Raw byte (observed 6) |
| `0105` firmware/hardware versions | — | Entry count, then comma-separated component/kind/string triples |
| `0106` battery | — | Count followed by component/value pairs; high bit = charging |
| `0108` touch bindings | `0401` | Read: count + function IDs. Reply/set: count + four-byte bindings |
| `0109` wear/case/connection state | — | Count + component/flags pairs |
| `010c` noise control | `0404` | Action, type, value; read current with `01 01`, cycle with `02 01` |
| `010d` feature switches | `0403` | Read: count + feature IDs. Reply: count + ID/boolean pairs. Set: ID/boolean |
| `010f` EQ preset | `0406` | Single preset ID |
| `0110` volume enhancer | `0407` | Boolean |
| `0112` multipoint device list | `040b` | Variable-length entries; set action includes reversed MAC bytes |
| `0120` device language code | — | Raw byte (observed 3); layout quirk above |
| `0122` custom EQ list | `0418` | See below |
| `0129` diagnostic feature status | — | Count + ID/u32LE pairs, often delivered in multiple replies |
| `012c` dynamic audio bands | `041b` | Count + signed min/max/current/type tuples; setter sends one tuple |
| `0200` notification capabilities | `0205` subscribe | Count + notification IDs |
| — | `0400` find earbuds | Boolean; can generate loud audio |
| — | `0405` fit test | Start=1, stop=0; results arrive as notifications |

Capabilities are queried before writes. A capability bit means the firmware advertises support, not that the corresponding behavior has been hardware-tested. Unknown events remain available as raw packets.

### Noise

Wire current-mode values: Off=1, Transparency=2, Mild=4, Maximum=8, Moderate=16, Adaptive=32. Current mode is set with `01 01 VALUE`. Noise-cycle values are Off+Transparency=3, Off+ANC=9, ANC+Transparency=10, All=11, set with `02 01 VALUE`.

`0404` returns status **14** both in the case and out-of-case when neither ear is worn. All six modes succeeded with **right-only wearing**, **left worn/right in a closed case**, and **both worn**. All four cycle values succeeded without wearing. Original values were restored after each check. Query `[04 01]` reads the adaptive algorithm's reported level separately (observed action 4/type 1/value 16).

### Feature IDs

| ID | Feature |
| --- | --- |
| 4 | Auto play/pause |
| 6 | Game mode |
| 8 | Auto-answer |
| 17 | Dual-device connection |
| 24 | High-definition sound / LHDC |
| 26 | Wind noise reduction |
| 27 | Spatial Audio |
| 29 | Dynamic audio |
| 38 | Four-tap find-phone gesture enable (host ringing requires an implementation) |

Other IDs returned by the firmware are preserved. Where a generic SDK name is traceable it is labelled as generic/unverified on Air7; otherwise it remains unidentified. An advertised/returned flag does not prove its effect. The phone's Auto-answer toggle was captured as `0403 0801`, **not** feature 11.

### Touch bindings

Each binding is `[ear, button, gesture, function]`. Air7's button field is 1. Gestures are 1 single, 2 double, 3 triple, 4 hold, 6 long hold. Wire function IDs include 0 disabled, 1 play/pause, 3/4 voice assistant (alternative SDK encodings), **5 previous track, 6 next track, 11 volume up, 12 volume down**, 8 noise control, 13 switch device (generic SDK), and 17 game mode. IDs 26/27/31 are context-dependent host actions; newer SDK model configurations use conversation/live translation/AI assistant. Do not unconditionally label 26 find-phone. Air7's fixed four-tap find-phone enable is feature **38**, separate from these bindings. See [state audit](state-model.md) for the corrected converter evidence and unverified effects.

Component IDs **1 left / 2 right** were confirmed by the user's physical wearing transitions and match the SDK. The GUI shows names and IDs. Both sides' double-tap configuration could be written/read back/restored with the right bud in the closed case. A successful configuration readback still does not prove a physical gesture fired the intended action. Find-phone/voice-assistant behavior depends on the receiving host.

### Custom EQ

Reply list:

```text
count:u8
repeat count:
  selected:u8 | min:i8 | max:i8 | id:u8 | name_bytes:u8 | UTF8_name
  band_count:u8 | repeat { frequency:u16LE | gain:i8 }
```

Setter:

```text
action:u8 | min:i8 | max:i8 | id:u8 | name_bytes:u8 | UTF8_name
band_count:u8 | repeat { frequency:u16LE | gain:i8 }
```

Actions: **1 add, 2 update/select/rename, 3 delete**. The earbuds assign a new ID on add. Update also selects the EQ, even if the old `selected` field was false. Selection/renaming are updates, not separate guessed opcodes. The reply is status + affected ID; query the list for the resulting state.

Air7 exposes six frequencies: **62, 250, 1000, 4000, 8000, 16000 Hz**, with gains **−6..+6 dB**. Name length counts UTF-8 bytes, not characters. The reusable types preserve device-provided bounds/frequencies rather than assuming every model shares Air7's layout.

Preset IDs: 0 Original sound, 1 Serenade, 2 Clear Bass, 3 Deep Bass. Do not pass a custom EQ ID to the preset setter; select it through `0418` action 2.

### Fit results

`0405 01` starts the test. Both buds must be worn on this firmware; either one-ear setup returned status 14. The both-worn test acknowledged start, then emitted `0204` event 4 payload `04 01 01 02 01` after about nine seconds: left=1/right=1, both good fit. `0405 00` was acknowledged afterwards.

Fit event data is two `[component,result]` pairs, without a count byte. Raw result 0 means general fit; 1 means good fit; 3 is detection failure; 4 is interruption by wearing changes; 5 is interruption by the app. Unknown codes remain raw. A start acknowledgement is not a fit verdict; collect the actual event. The helper retains early results arriving before that acknowledgement, handles cancellation while waiting, and always attempts stop cleanup.

### Notifications

- `0204`: event ID + event-specific data. Typed events include battery (1), wear (2), noise (3), fit result (4), multipoint (6), and user interaction (`f1`). User-interaction fields are component/button/gesture/function/scene followed by u16LE options.
- `8129` and `812a`: diagnostic status/detection values, with the framing quirks above.
- `0502`: host-operation request byte (the SDK treats 5/6 as find-phone operations).
- `050e`: use-time entries, count + ID/u32LE pairs.
- `0503`: feature-switch state list.
- `0504`: EQ preset.
- `0506`: custom EQ list.
- `0508`: dynamic band min/max/current/type.
- `0500`: host-time request; answered with `8500`, matching transaction, status 0 and Unix seconds as u32LE.

The client retains at most 256 pending events, dropping the oldest if the caller does not drain them. The GUI shows up to 50 recent typed events. Unknown messages remain raw. None of these are firmware-update commands.

## UI versus transport

Noise mode is one value including its ANC level, not an independent enable bit plus a strength register. The phone stores the last chosen ANC level locally. Its UI setting codes differ from wire mode values. Unsaved EQ editing, menu visibility, first-use consent, Android permissions and codec availability are also host/UI concepts, while enabled features and saved EQs are actual transport state. The complete known-feature boundary ledger and unresolved branches are in [state-model.md](state-model.md).

## Partial and cached state

See [states and API](states-and-api.md). Battery entries can disappear independently of wear/connection entries. Values above 100 retain their raw byte and charging bit but use `percent: None`. Multipoint retains raw flags, raw name bytes and extension bytes. Firmware flag 1 is called “my phone” by the SDK; it is not proof that host owns the laptop's control socket.

The standalone lid position and measurement freshness are not identified. Do not infer them from case reachability, a reported 0% case value, or an absent peer battery. Diagnostic/property timeouts remain explicit rather than becoming default values.
