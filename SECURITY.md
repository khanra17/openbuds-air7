# Security reporting

This is an early-development Bluetooth controller, not safety-critical software.

Report a security issue through the repository's **Security → Report a vulnerability** page when private reporting is available. Do not post device addresses, credentials, raw captures or an exploit affecting nearby devices publicly. If private reporting is unavailable, open an issue asking for a private reporting channel without technical exploit details.

Include affected version/firmware, platform, impact and a minimal sanitized reproduction. Avoid hardware actions that generate unsafe audio, erase pairings or write firmware.

No response time or long-term support guarantee is promised. Only the current main branch is maintained until a release support policy is established.

Potential report areas include malformed-frame handling, unbounded buffering, unsafe sound/retry behavior, identifier exposure and native transport resource handling.
