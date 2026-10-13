# 2026-10-13 — The passkey profile check accepts Apple's wildcard domain

Signed macOS builds check the Developer ID provisioning profile before packaging.
`validateMacPasskeyProfile` in `scripts/mac-passkey-signing.cjs` requires the profile to
authorize the passkey domain through `com.apple.developer.associated-domains`.

Apple's "Simplex Developer ID Passkeys" profile grants every domain as the plain string `"*"`.
The check only accepted a list (`["*"]` or `["webcredentials:<rpId>"]`), so every signed macOS
build failed with "Apple profile does not authorize the passkey associated domain". The check now
also accepts the string `"*"`.

Desktop and `@hyperbridge/simplex` move to 0.17.2 together, as `verify-versions` requires.
