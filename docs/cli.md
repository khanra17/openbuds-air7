# CLI guide

Run `buds --help` and each command's `--help` for the full interface.

## Read state

```powershell
buds devices
buds status
buds battery
buds wear
buds versions
buds firmware
buds features
buds touch
buds multipoint
buds --timeout 2 diagnostics
buds watch
```

The default output identifies components, states and known functions by name. Unknown/missing readings stay explicit. `--raw` returns the numeric wire-level schema for diagnostic scripts. `watch` runs until Ctrl+C.

If automatic device selection is ambiguous, obtain the address locally using `buds devices` and pass it through `--address`. Addresses and names in command output are private device data; do not post them.

## Noise categories and ANC level

```powershell
buds anc on --level mild
buds anc on --level adaptive
buds anc off
buds noise anc --level maximum
buds transparency on
buds transparency off
buds cycle all
```

These select one mutually exclusive transport mode. ANC is not an independent enable bit. Disabling ANC/Transparency selects Off. No hidden previous level is guessed. See [state model](state-model.md).

## Stored switches

```powershell
buds toggle game on
buds toggle game off
buds toggle auto-play-pause on
buds toggle auto-answer off
buds toggle lhdc on
buds toggle wind-noise on
buds toggle spatial off
buds toggle dynamic on
buds toggle find-phone-gesture off
```

LHDC/dual-device changes can interrupt the link. The four-tap find-phone flag enables requests; host ringing is separate and not implemented by the desktop.

## EQ

```powershell
buds eq-list
buds eq-preset 0
buds eq-save 0 "Example EQ" 0 1 0 -1 0 1 --add
```

The device assigns an ID on add. Update/select/rename use the same wire update operation and also select the EQ. List EQs to get the assigned ID before using `eq-select`, `eq-rename` or `eq-delete --confirm`. Do not delete an existing preset just to test the API.

Six bands are 62, 250, 1000, 4000, 8000 and 16000 Hz. Preset IDs 0..3 are Original, Serenade, Clear Bass and Deep Bass.

## Tests, finding and diagnostics

Sound-generating actions require `--confirm-sound`. Enabling volume enhancement requires `--confirm-louder`. Connection/pairing changes require `--confirm`.

Finding is bounded to three seconds and guarded by the wear/case observations. Fit collection attempts stop on completion, timeout and cancellation. Cleanup can fail after power loss or process termination.

An optional `--trace` path records wire data. Keep the file private. Do not use unknown raw commands, especially reset/OTA families. A timeout after a write does not prove it failed; read back before retrying.
