# Air7 behavior and limits

Hardware evidence is from Air7 firmware 1.1.0.62; case version 088. Verified settings were read back and restored. That does not establish every physical effect or state combination.

| Control / observation | Device state and evidence | Host behavior / unverified branches |
| --- | --- | --- |
| Noise mode | Off, Transparency, Mild, Moderate, Maximum and Adaptive verified in both one-ear setups and both-worn | One mutually exclusive field, not separate ANC enable/strength registers. Neither-worn writes returned status 14 |
| Remembered ANC level | Not a separately identified device register | Desktop remembers it per address for the session. Off/Transparency do not reveal a saved level. Adaptive-depth query is separate |
| Hold noise cycle | All four stored cycles verified | Physical cycling and restored ANC level untested |
| Battery / charging / wear / connection | Independent observations; missing battery and unknown flags preserved | No identified lid sensor or measurement timestamps. A reported case 0% is not proof of an empty case |
| Wind reduction, feature 26 | Boolean readback verified | DSP effect across modes unmeasured |
| Auto play/pause, feature 4 | Boolean readback verified | Playback removal/resume and single-ear rules untested |
| Auto-answer, feature 8 | Boolean readback verified | Incoming-call behavior and OS routing untested |
| Game mode, feature 6 | Boolean readback verified | End-to-end latency unmeasured |
| LHDC preference, feature 24 | Boolean readback verified; some changes required reconnect | Does not install a host codec or report the negotiated audio codec |
| Spatial audio, feature 27 | Boolean readback verified | Rendering/head tracking unproven |
| Dynamic audio, feature 29 | Toggle and all three bands at minimum/zero/maximum verified | Stored bands remain readable while disabled; band values have no established dB units |
| Volume enhancer | On/off readback verified with neither earbud worn | Not system volume; requires loudness consent |
| Stock / custom EQ | Four presets and custom add/update/select/rename/delete verified | Unsaved edits are local. Conflicting selections stay explicit; device assigns added IDs |
| Touch bindings | Both sides' double-tap writes verified, including an in-case peer; all returned rows exposed | Physical gestures and reserved single-tap/long-hold rows not fully tested; some actions need a host |
| Dual-device, feature 17 | Disable/re-enable verified | Stored preference is not audio-focus ownership |
| Multipoint | Host list and disconnect/reconnect verified; raw names/flags/extensions retained | Pairing removal untested. SDK “my phone” flag is not control-socket ownership |
| Fit test | Both-worn returned good fit for both ears after about nine seconds; stop acknowledged. One-ear starts returned status 14 | Failure/interruption branches not physically exercised. Start acknowledgement is not a verdict |
| Finding tones | Wear/case/connection guard and bounded cleanup implemented | Positive tone untested. Physically remove both earbuds; missing/stale sensors cannot establish safety |
| Find-phone gesture, feature 38 | Four-tap enable readback verified | No physical four-tap/ringing test; receiving host must implement ringing |
| Host events | Fit, interaction, host operations and unknown packets exposed | Host find-phone operations 5/6 are distinct from touch function 26; OS permissions still apply |
| Firmware / diagnostics | Versions, IDs and upgrade capabilities read; color/language returned raw codes | No firmware writer. Some profile/usage/auto-off queries time out; unknown codes are not guessed names |
| Extra feature flags | Returned IDs and values preserved | Generic SDK labels do not prove Air7 effects; experimental writes require separate validation |

`Snapshot::describe()` distinguishes reported, missing and failed reads. Missing data never becomes Off or 0%. Unknown wire values and events retain raw data. Charging/offline/case transitions, cache aging and disconnects during writes are not exhaustively tested. MindFlow/cloud content, host ringing, factory reset and firmware updates are not implemented.

## Protocol details that affect implementation

Service: `0000079a-d102-11e1-9b23-00025b00a5a5`, resolved through SDP rather than a fixed RFCOMM channel. Audio transport is separate.

```text
AA | length:ULEB128 | control:u8 | reserved:u8 | [fragment:u8] | inner
inner = command:u16LE | transaction:u8 | payload_length:u16LE | payload
```

Replies normally use `command | 0x8000`, matching transaction and a leading status byte. Requests are serialized and never automatically replayed.

- Current noise values: Off=1, Transparency=2, Mild=4, Maximum=8, Moderate=16, Adaptive=32. Android UI codes differ. Current read is `010c [01 01]`; setter is `0404 [01 01 VALUE]`; `[04 01]` is adaptive depth, not current mode.
- Touch functions 5/6 mean previous/next track; 11/12 mean volume up/down. Function 26 is context-dependent, not an unconditional find-phone action. Feature 38 and host operations 5/6 are different namespaces.
- Custom EQ action 2 updates/selects/renames; preset selection is separate. Gains are −6..6 at 62/250/1000/4000/8000/16000 Hz. Name lengths count UTF-8 bytes.
- Fit result codes: 0 general, 1 good, 3 failed, 4 wear interruption, 5 app interruption; other codes remain unknown.
- Captured length exceptions are narrow: `841b` adds opaque metadata, `8120` declares 256 for two bytes, `8129` omits status/count and uses device counters, `812a` declares five for six bytes, and cycle notifications add a reserved zero. Other malformed frames remain errors.

[Driver source](../crates/realme-buds/src/devices/air7) and [protocol tests](../crates/realme-buds/tests/protocol.rs) contain the layouts and regression packets. A new model must not inherit these quirks without evidence.
