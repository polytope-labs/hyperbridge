# 2026-09-03 — The relayer is a separate storage variable, not a `Params` field

Chosen: `address _relayer` appended after `_paused` in `IntentsBase`, set through its own
`setRelayer` call.

`Params` occupies slots 4 to 8 and `_orders` starts at slot 9. Adding a field to the struct would
push every mapping behind it and corrupt escrow on the live proxy. Reusing `UpdateParams` was
therefore never available, quite apart from it being a Hyperbridge-relayed message: the whole point
is to hold even if Hyperbridge's consensus is compromised, so the setter must not depend on it.

Alternative rejected — a new `RequestKind` for governance to set the relayer. It needs the
`intents-coprocessor` pallet mirrored, and it is still a cross-chain message. Governance can already
rotate the relayer through `UpgradeContract` migration calldata when it wants to; the owner path
covers the case where it cannot.
