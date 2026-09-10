# 2026-09-03 — Encode Substrate accounts once at the keyring boundary

Chosen: configure the shared sr25519 `Keyring` with SS58 prefix `0`, Polkadot's unified account
format. Every Simplex path derives its pair through this service, so wizard responses, review and
operator UI, clipboard values, balance snapshots, and runtime logs all use one canonical display
without changing the underlying public key or signing identity.

Alternative rejected: converting addresses separately in React components would leave server-side
responses and logs inconsistent, duplicate formatting logic, and add Polkadot crypto code to the
browser bundle.
