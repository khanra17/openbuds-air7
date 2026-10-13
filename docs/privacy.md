# Privacy

The normal application connects locally over Bluetooth. It does not require an account, upload device state, contact a cloud service or enable Android debug logging.

## Data you should not publish

Runtime output and traces can contain Bluetooth addresses, device/host names, user-defined EQ names, timestamps and pairing information. HCI logs may also include unrelated traffic or audio. Do not attach full exports, screenshots of live host lists, APKs, decompiled code or firmware images to issues.

Share the smallest sanitized reproduction: firmware/model, OS version, operation, expected/actual state and a minimal packet with identities replaced by synthetic values. Explain which bytes were replaced. Never redact lengths inconsistently.

## Repository policy

- Private research, builds, machine/session data, logs and experiments are excluded from Git.
- Tests contain only synthetic/sanitized fixtures. A small set of obvious sequence addresses exists solely to test byte order.
- Documentation images come from an offline synthetic demo or original SVG artwork.
- No developer home path, real hardware address, phone serial, personal account email or live host/preset name belongs in source.
- Do not put tokens or credentials in a clone, issue or screenshot.

Run the [publication gate](development.md) before contributing. It checks known artifact/identifier patterns, but manual review is required, especially for binary assets.

The public GitHub repository's owner handle is necessarily visible in repository URLs. It is not embedded as a device identity or personal commit author.
