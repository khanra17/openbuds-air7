# Feature coverage

Verified writes mean a laptop command was acknowledged and read back. Subjective audio quality and physical gesture outcomes are separate tests. Firmware writes are excluded throughout.

## Live validation

Only **Buds Air7**, firmware **1.1.0.62**, was operated. Buds Air 2 was not connected or tested.

| Feature | Library / CLI / GUI | Evidence |
| --- | --- | --- |
| Per-ear battery / case battery | Yes | Live reads; missing right battery preserved when right was in closed case |
| Firmware/hardware versions | Yes | Earbuds 1.1.0.62; case reports 088 |
| Vendor/product/protocol metadata | Yes | Live reads: vendor 0x072f, product 0x064812, protocol bytes 00 00 01 |
| Upgrade capability information | Read only | Live parsed capability; **no OTA writes** |
| Wear / in-case / connection flags | Yes | Neither worn, right-only, left-worn/right-in-closed-case, and both worn captured |
| Off / Transparency / Mild / Moderate / Maximum / Adaptive ANC | Yes | All six modes verified with right-only, left-worn/right-in-case, and both worn; original mode restored |
| ANC with neither worn | Yes, rejection exposed | All six requests returned status 14; no false success |
| Adaptive ANC's reported level | Advanced read | Query action 4/type 1 returned value 16; not confused with current mode/cycle |
| Noise gesture cycle | Yes | All four cycles verified and restored while not worn |
| Dynamic audio toggle | Yes | Both boolean states verified |
| Dynamic low/mid/high bands | Yes | Every band checked at −5, 0, +5 and restored |
| All four EQ presets | Yes | IDs 0..3 verified and restored |
| Custom six-band EQ | Yes | Add, gain update, rename, select, delete/readback/restoration verified |
| LHDC / high-definition flag | Yes | Both states verified; link recovery needed after some changes. Does not add a Windows codec |
| Volume enhancer | Yes, confirmation | Both states verified **before the user wore either bud**; restored |
| Spatial Audio | Yes | Both states verified; rendering not subjectively evaluated |
| Dual-device enable/disable | Yes | Disable/readback/reconnect/restore verified |
| Multipoint list/state/flags | Yes | Read; raw flags, name bytes and extension bytes preserved |
| Multipoint disconnect/reconnect | Library / CLI / GUI | Test phone disconnected to state 0, reconnected to state 2; pairing kept |
| Multipoint remove pairing | Library / CLI / GUI, confirmation | Mapped, not executed: would erase an existing host pairing |
| Game mode | Yes | Both states verified |
| Auto play/pause | Yes | Both stored states verified; actual media-pause behavior needs a playback/removal test |
| Auto-answer | Yes | Both stored states verified; no real incoming call was generated |
| Wind noise reduction | Yes | Both states verified |
| Touch bindings | Yes | Both sides' double-tap changes/readback/restoration verified, including the in-case peer; physical gestures not triggered |
| Earbud fit test | Yes, bounded + cancellable | Both-worn start accepted; both ears returned **1 = good fit** after about nine seconds; stop acknowledged. One-ear starts rejected with status 14. Settings readback matched baseline |
| Find earbuds | Yes, guarded + bounded | Positive tone intentionally not run after user began wearing a bud; stop command available. Human confirmation and wear/case checks required |
| Button/user-interaction events | Library / watch | SDK parser + regression tests; live physical gesture capture still pending |
| Four-tap find-phone gesture enable | Library / CLI / GUI | Newly audited feature **38**; enabled/read back/restored; no four-tap or ring test |
| Host find-phone operations | Library event | Operation requests 5/6 surfaced to caller; separate host permission/consent/ringing flow; no automatic laptop-speaker ringing implemented |
| Color / device language code | Advanced read | Live values 6 / 3; raw codes, no invented meanings |
| Diagnostic feature-status values | Advanced read | Multiple device-counter replies collected; partial list preserved without filling missing IDs |
| SDK profile/usage/power-off queries | Advanced probe | Some are advertised but did not respond; explicit timeout results, not fabricated values |
| Firmware upgrade / firmware flashing | **Excluded** | No image writer or OTA transaction sequence implemented/run |
| Earbud unpair/factory reset | **Not implemented/tested** | No destructive earbud reset/unpair operation run |
| MindFlow / cloud content | **Not implemented** | Host media/content service, not an observed Bluetooth setting |

The non-wearing suite ran 39 checks: 33 acknowledged/readback successes, plus six expected ANC rejections. Every starting value was restored. Separate wearing sweeps verified all six ANC modes in both one-ear setups and with both worn. The both-worn fit test completed with two good-fit verdicts, not just a start acknowledgement.

## Edge-state representation

- Component 1 is left; component 2 is right, verified against the user's physical transitions.
- Wearing, being in the case, connection flags, battery availability and charging are separate fields. One must not be inferred from another.
- An absent battery entry stays absent. An unavailable/non-percentage battery code becomes `percent: None` while retaining `raw` and charging-bit information.
- A case percentage of 0 was reported when the case was closed/unreachable. It remains a **reported** value, not proof the case was actually empty or freshly measured.
- No standalone case-lid bit has been identified. The user supplied the closed-lid context; the protocol independently reported right-in-case. Do not infer lid position merely from missing battery data.
- A missing/offline ear yields `WearingScenario::Unknown`, while the known per-ear fields remain accessible. It is not silently assumed not worn.
- Unknown notification kinds remain raw packets. Known byte-layout quirks are narrowly scoped; arbitrary malformed frames remain errors.

## Semantic audit, not just successful setters

The [state/UI/transport audit](state-model.md) separates active state from local UI preferences, explains all known feature boundaries, and lists unresolved meanings/conditions. ANC now has explicit category and intensity controls; the last chosen level while Off is labelled local rather than an earbud reading. Both frontends use shared named component/state descriptions. Single-tap/long-hold rows are no longer silently hidden. The track/volume direction labels were corrected against both SDK converter directions. This changes interpretation, not the user's stored bindings.

## What remains before claiming full parity

1. Capture physical touch gestures and test real playback pause/resume and call-answer behavior. Stored switch readback is not proof of those OS/audio behaviors.
2. With both buds out of the ears, sound-test the guarded finding path.
3. Decide whether to add host-side ringing and phone-only MindFlow content; these are outside the Bluetooth settings protocol.
4. Resolve SDK queries that the Air7 firmware advertises but leaves unanswered; a timeout does not prove permanent lack of support.
5. Add Linux/macOS native transports if needed. Windows and the transport-independent core are implemented.

Therefore, this is broad tested control coverage, **not a claim that every undocumented opcode or physical condition has been exhausted**. The library exposes the known controls, explicit diagnostic probes, opaque data and raw access needed to continue safely. Never blindly probe destructive or OTA commands.
