# Troubleshooting

## Build fails with linker/SDK errors

Install Visual Studio Build Tools with Desktop development with C++ and a Windows SDK. Run Cargo from a developer shell with the MSVC environment available. This project does not depend on a particular developer's cached SDK paths.

## Paired, but control will not connect

Pairing, audio profiles and the RFCOMM control channel are different layers. Check Bluetooth availability, use the Air7, and close competing control clients. The vendor service UUID is resolved through SDP; do not guess channels or substitute BLE GATT.

## Firmware status 14

Noise-mode writes returned status 14 when neither ear was worn. Fit start returned it in one-ear setups and succeeded with both worn on firmware 1.1.0.62. This is a firmware/state rejection, not proof of an OS permission failure.

## A battery/feature is missing

Missing means unknown/not returned, not 0% or Off. In-case/offline peers can disappear from battery readings independently of wear flags. Case readings can be cached; a reported 0% is not proof of a fresh measurement.

## Lost acknowledgement or connection

The setting may already have changed. Reconnect and query state before retrying. Do not replay setters as a recovery loop.

## LHDC enabled, but Windows uses another codec

The switch only changes an earbud preference. Actual audio codec negotiation depends on the host/audio stack; this app installs no codec or audio driver.

## Diagnostics time out

Some SDK queries do not respond on the validated firmware despite advertised support. Report a sanitized result; do not replace a timeout with a default value or brute-force other opcodes.

## Demo buttons do not change the device

Expected: demo mode has no Bluetooth worker and refuses hardware actions.
