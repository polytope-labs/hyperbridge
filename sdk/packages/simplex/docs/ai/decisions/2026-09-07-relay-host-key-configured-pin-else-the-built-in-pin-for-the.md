# 2026-09-07 — Relay host key: configured pin, else the built-in pin for the hosted relay, else trust on first use

`relayHostKey` pins explicitly; for the hosted relay the deployed key's fingerprint is compiled in;
otherwise the key seen on first contact with that relay address is stored in `tunnel/known_relay`
and enforced afterwards. A mismatch is refused and reported in
the UI, not retried silently. A relay is zero-trust by construction (it only sees ciphertext), so
this pin protects availability rather than confidentiality, which is why TOFU is acceptable as
the default.
