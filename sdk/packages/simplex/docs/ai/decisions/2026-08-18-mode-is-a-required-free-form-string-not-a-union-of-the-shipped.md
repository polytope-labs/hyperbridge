# 2026-08-18 — `mode` is a required free-form string, not a union of the shipped backends

Chosen: `Signer.mode: string`. Free-form — the union it replaced made every custom signer misreport itself as one of three backends it is not — and required, since the final interface pass made every member required: a label costs an implementer one string and buys every log line a real backend name instead of `"custom"`.

Alternative considered: keeping `mode: "privateKey" | "mpcVault" | "turnkey"`, or leaving it optional with a `"custom"` default.

Why: the field is only ever read for logs (`DelegationService`, and the boot-time "EVM signing strategy" line). A label with no behaviour attached should not be a closed set, and a fleet running several backends wants each one named.
