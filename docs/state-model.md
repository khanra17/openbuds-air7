# State model and UI/transport boundaries

**This is an audit, not a certificate of exhaustive parity.** Tested setters, interpreted states, app-only presentation, host prerequisites and unresolved protocol fields are different kinds of evidence. The current known-state surface is described below; remaining gaps are listed explicitly. No firmware writes are part of this project.

## Noise control: category versus ANC level

The phone presents **Noise cancellation / Off / Transparency** and, when cancellation is selected, an ANC level. The current noise-control transport field is **one mutually exclusive mode**, not an enable boolean plus an independent intensity register.

| Wire value, action 1/type 1 | Category | ANC enabled (derived) | Active ANC level |
| --- | --- | --- | --- |
| 1 | Off | false | not applicable |
| 2 | Transparency | false | not applicable |
| 4 | ANC | true | Mild |
| 16 | ANC | true | Moderate |
| 8 | ANC | true | Maximum |
| 32 | ANC | true | Adaptive |
| Other value/action/type | Unidentified | unknown | unknown |

Read `010c [01 01]`; set `0404 [01 01 VALUE]`. Selecting any of the four ANC values both enables ANC and selects its level. There is no observed independent ANC-enable packet. Selecting Off also disables Transparency; selecting Transparency also disables ANC. Separate boolean-looking controls are **UI/API conveniences over the same field**.

The Android app's intermediate setting codes are different: settings 2=Off, 4=Transparency, 8=Mild, 1=Maximum, 16=Moderate, 32=Adaptive. These are converted before transmission. Confusing these domains can silently reverse behavior.

### Remembered intensity is not a current earbud observation

The SDK's `NoiseLevelStatusUtils` stores last choices in Android preferences (`HEADSET_LAST_NOISE_LEVEL_` / `HEADSET_LAST_NOISE_LIST_VALUE_`, keyed by device name). `TLHeadsetConnectionImp.O5` eventually emits the same action/type/value setter. The top-level ANC choice can therefore restore a **host-remembered** level.

- `NoiseState::describe()` reports an active level only when the transport currently reports ANC.
- `AncPreference` is explicitly host/UI memory. Choosing a level locally sends no packet.
- The desktop scopes that memory by Bluetooth address for the current session. It is not synchronized with realme Link and is not persisted across app restarts.
- When noise is Off/Transparency and no level has been observed/chosen locally, the desktop requires a choice before enabling ANC. It does not pretend the earbuds reported Maximum.
- While ANC is active, changing its level emits a real setter. While Off/Transparency, choosing the level only stages the UI preference; **click ANC to activate it**.
- `010c [04 01]` is a separate adaptive-level report. Its reported value must not be substituted for a stored user preference or used to infer that ANC is enabled.

The GUI now uses the three top-level categories with a separate level row. CLI examples:

```powershell
buds anc                         # Named current state, not a guessed preference
buds anc on --level moderate
buds anc off
buds noise anc --level adaptive
buds transparency on
buds transparency off            # Selects Off; does not restore a hidden previous mode
buds --raw noise                 # Original action/type/value output
```

All six active modes were acknowledged/read back/restored in right-only, left-worn/right-in-case, and both-worn setups. Neither-worn attempts returned firmware status 14. The GUI warning is an explanation, not a fabricated device state.

## Components, availability and freshness

IDs **1=Left earbud, 2=Right earbud, 3=Charging case** are confirmed by the SDK and the user's physical wearing transitions. `Identifier` retains the numeric ID alongside its name. Unknown IDs are labelled unidentified, never defaulted to Right/High/etc.

Battery, charging flag, wear, in-case flag and connection flag are independent observations:

- **Reported 0%** is distinct from **no reading returned** and **an unavailable percentage code**.
- Missing battery data has no known charging state. Do not fabricate 0% or `charging=false`.
- In the left-worn/right-in-closed-case capture, the right battery entry was absent even though a right wear/connection entry was present.
- Closed/unreachable case data reported 0%. That is not proof of an empty case or a fresh measurement.
- The SDK uses a generic flag structure for the case too. Its wear/in-case bits have no identified physical lid meaning. Semantic output deliberately does not label them as lid-open/lid-closed.
- Missing/offline ear observations make the combined wearing scenario unknown; known observations remain accessible.
- Firmware caches have no identified per-field measurement timestamp. A successful query proves the firmware returned data, not that every sensor value was freshly measured.

`Snapshot::describe()` distinguishes `Reported`, `NotReported`, and `ReadFailed` while retaining the original snapshot. The CLI uses these named descriptions by default. `--raw` preserves the original numeric schema for existing diagnostic scripts.

## Feature-by-feature boundary ledger

| Feature | Transport state/control | UI/host state and gates | Remaining limits |
| --- | --- | --- | --- |
| Noise category and ANC level | Single mode field above | Category grouping; remembered level is local | No independent enable bit or remotely identified saved level while Off |
| Touch-and-hold noise cycle | Separate action 2 field; values 3/9/10/11 | Ordered list of named categories | Physical cycling and its ANC restore level not yet captured |
| Wind reduction | Feature 26 boolean | Phone can hide it outside certain noise views | Stored flag verified; effective DSP behavior across modes not measured |
| Auto play/pause | Feature 4 boolean; wear observations remain separate | Phone calls it smart playback/wear detection in different screens | Real media removal/resume and all single-ear rules need playback tests |
| Auto-answer | Feature 8 boolean | OS call routing/incoming call required | Stored states verified; no real incoming-call test |
| Game mode | Feature 6 boolean | Latency explanation, gestures and host audio route | Flag verified; end-to-end latency not measured |
| LHDC/high-definition sound | Feature 24 boolean | Host codec support/negotiation and reconnect | Windows codec is not installed by this switch; negotiated audio codec not read from this control service |
| Spatial audio | Feature 27 boolean | Named phone subpage, optional host/device behavior | Flag verified; rendering/head-tracking behavior not proven |
| Dynamic audio | Feature 29 boolean | Phone hides/expands sliders based on toggle | Stored bands remain available when processing is Off |
| Dynamic bands | `012c` / `041b`; per-band signed min/max/value/type | Low/Mid/High names for IDs 1/2/3 | No invented dB units; unknown band types labelled unidentified |
| Volume enhancer | `0110` / `0407` boolean | Loudness confirmation is host UI | Not system media volume; on/off verified only with buds not worn |
| Stock EQ | Preset IDs 0..3 | Friendly preset names | Unknown ID stays unknown, not Original |
| Custom EQ | List, selection flag, gains, bounds, name, ID; add/update/delete | Unsaved editor, Add/Edit mode and delete confirmation are local | Rename/select are updates; multiple selected entries are represented as a conflict |
| Touch bindings | Per-component/button/gesture/function records | Phone model config determines visible rows/choices | Software readback is not proof a physical gesture fires an action |
| Dual-device toggle | Feature 17 boolean | Warning/reconnect workflow | Flag readback is not proof of audio-focus ownership |
| Multipoint hosts | Name/address/state/flag/extensions; connect/disconnect/remove | Confirmation; display names | States 0/1/2 named; unknown values retained; SDK my-phone flag is not control-socket ownership; pairing removal not executed |
| Fit test | Start/stop + event with two component/result pairs | Consent, quiet-room instructions, timeout/cancel and result presentation | Both-worn good results captured; physical failure/interruption result variants not all exercised |
| Find earbuds | Start/stop boolean | Human removal confirmation, bounded duration and conservative wear/case guard | Positive tone untested while worn; guard intentionally refuses missing/offline/in-case ears, rather than claiming every one-ear finding setup is supported |
| Find my phone four-tap enable | **Feature 38 boolean**, exposed as `FindPhoneGesture` | Phone shows a first-use consent dialog; four taps request host ringing | Setter/readback/restore verified. Desktop does not implement ringing; no four-tap or ring test performed |
| Find-phone host requests | `0502`, operations 5/6 | Host app/service must receive and act on request | Not the same as an earbud tone or touch function 26; background permissions/OS policies can prevent ringing |
| Battery and charging | Component/value pairs | Icons, hiding offline components, freshness caution | No identified case-lid sensor or per-reading age |
| Firmware/hardware info | Component/version-kind/string; upgrade flags; IDs | UI aggregate version, update badge and cloud latest-version check | Upgrade information read only; kind 3 meaning partly unresolved; **no updater/flashing** |
| Color / language | Raw codes (observed 6/3) | Physical appearance / app interpretation | Code-to-name mapping unresolved; no guessed color/language or setter |
| Personal ANC/profile, hearing, ear scan, use time, auto-off | SDK reads/raw probes | Generic SDK/model configuration and guided workflows | Some queries time out; not silently classified as off/unsupported. Calibration/profile writes not run |
| Extra SDK feature flags | Returned ID/boolean pairs | Generic SDK labels where traceable | Power-saving/party/voice/HearID/virtual-bass names are not proof of Air7 effects; unverified writes are not promoted as normal Air7 controls |
| MindFlow | Host/cloud media content | Playback, content/downloads/account/model gates | Not an observed earbud setting; not reproduced by the Bluetooth library |
| Privacy, scan/Bluetooth/location and notification permissions | No identified earbud permission bit | Android/Windows policies, first-use consent, app background behavior | These are host prerequisites, not measurements on the earbuds; no automatic grant/consent |
| Pairing/device discovery | Windows paired-device enumeration + vendor SDP/RFCOMM | Device picker; choosing another target while connected is disabled | ACL/A2DP/RFCOMM states are distinct; stored pairing is not live control connection |

## Touch audit corrections

A successful numeric write/readback can still have the **wrong user-facing meaning**. The previous labels inverted track and volume directions. The audited path `ItemFactory.f` → `ParamsConverter.A` and the reverse `PodsUtils.b` establishes:

| Wire function | Meaning |
| --- | --- |
| 5 | Previous track |
| 6 | Next track |
| 11 | Volume up |
| 12 | Volume down |
| 13 | Switch connected device (generic SDK; physical Air7 effect unverified) |
| 3/4 | Voice assistant, alternative SDK encodings |
| 26/27 | Context-dependent host actions; newer model configuration uses conversation/live translation |
| 31 | AI assistant in newer model configuration; compatible host required |

The shared enum and both frontends are corrected without rewriting existing device bindings. Function 26 must **not** be unconditionally called find-phone. Air7's documented four-tap find-phone enable is instead settings type 42 → feature 38. Its host request operations 5/6 are a different numeric namespace.

Air7's current mobile Button settings page exposes double tap, triple tap and hold per ear. The transport also returned single tap and long hold records. The desktop now shows **all returned rows** instead of silently filtering them away. That does not prove every reserved row is physically assignable. Generic SDK choices are labelled as such; unknown component/button/gesture/function values remain diagnosable and unsupported writes are refused.

## Evidence and completion criteria

Evidence sources: captured Air7 traffic and readback, current Air7 mobile pages, and the app's headset SDK (noise preference/converter, key-action converter, find-phone settings activity and feature handler). Decompiled code and private captures are not distributed.

A feature is not “fully done” merely because its happy-path setter works. Each feature needs:

1. Identified raw schema and named semantic states, including missing/unknown/error results.
2. A documented UI/host/transport boundary and persistence owner.
3. Reads, notifications and setters where the device actually supports them.
4. Range, rejection, timeout, cancellation, disconnect and readback/restoration handling.
5. Hardware evidence for the relevant physical/setup branches, not only synthetic tests.

Still not exhaustive: combinations of every feature, connection handover during writes, all charging/offline/case-lid transitions, firmware-cache aging, physical tap/media/call outcomes, fit failure/interruption conditions, safely removed-earbud finding, host ringing, and unanswered SDK properties. Those are **open validation/research items**, not hidden completed states. No undocumented/destructive/OTA opcode is blindly replayed to inflate coverage.
