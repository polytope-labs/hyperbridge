# 2026-09-01 — Paymaster selection gated on EntryPoint deposit

`buildPaymasterAndData` now skips a candidate paymaster whose EntryPoint deposit cannot cover the
op's max prefund with 150% headroom, falling through Circle to Simplex to `type: "none"` with a
per-candidate reason. Motivated by a live incident: on Base the Circle paymaster's deposit was
drained (5.04e12 wei against a 1.49e13 prefund) and the Simplex paymaster's deposit was zero, so
every bid was signed against a paymaster the bundler was bound to reject
("precheck failed: paymaster deposit is X but must be at least Y") — the paymaster is baked into
the signed bid UserOp, so nothing could re-select at execution. Callers pass the op's gas terms:
`prepareBidUserOp` from the cached estimate, `trySendSponsored` from the caller's fixed limits or
the fallbacks — for which `getGasPrice` moved above paymaster selection (also turning a gas-price
failure into the safe never-submitted null). The Simplex gate runs before its builder, which can
send a bootstrap approve tx. Deposit-read failures fail open. Skips are warn-logged with deposit
and required figures.
Files: src/services/paymaster/types.ts, src/services/paymaster/index.ts,
src/services/UserOpSender.ts, src/services/ContractInteractionService.ts,
src/tests/services/PaymasterSelection.test.ts.
