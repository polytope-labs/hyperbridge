# 2026-10-16 — `decimals()` is cross-checked and overfill protection is removed

## `getTokenDecimals` cross-checks the asset registry

`ContractInteractionService.getTokenDecimals` reads `decimals()` through the plain
fallback client, not the event monitor's quorum, so the first endpoint to answer wins.
Cross-checking matters because one endpoint reporting 18 for a 6-decimal token inflates
every amount scaled by it by 10^12.

A successful read is cross-checked against the asset registry
(`configService.getAssetDecimalsByAddress`), which is verified on-chain when a token is
curated. Where the registry knows the token and the values disagree, the read is a hard
error — `Token … on … reports … decimals but the asset registry says …; refusing to price
it` — and nothing is cached, so the next call re-reads. Callers on the fill path treat a
throw as "skip this order" (`calculateProfitability` runs inside the caller's try/catch),
which is the right outcome for a value that cannot be trusted.

The cross-check covers tokens the registry knows. A token it does not know, such as a
custom `[assets]` entry, still takes the chain's `decimals()` answer and caches it.

The read-failure fallback to the registry is unchanged and still not cached: it covers a
transient RPC failure without pinning a registry value for the process. The native token
still short-circuits to 18 without a read. A cached value was cross-checked when it was
cached, and `setTokenDecimals` has no other caller.

## Overfill protection removed

The overfill clamp was disabled for limit-order pricing and the self-halt it fed could
never trip, so both are gone along with everything that configured or reported them:

- **Config:** `[simplex.overfillProtection]` (`maxOverfillBps`, `maxConsecutiveClamps`).
  `validateConfig` rejects a config that still carries the section, in any form including
  an empty table: `[simplex.overfillProtection] has been removed: Simplex no longer clamps
  bids or halts on overfill. Delete the section from the config and restart.` That check
  runs on `simplex run`, `Simplex.start`, the desktop, the setup API and `simplex init`,
  which reports the same message when it loads an existing config and offers to start
  fresh or exit so the operator can delete the section.
- **Library:** `Simplex.isHalted()`, `Simplex.resetHalt()` and `SimplexStatus.halted`;
  `FillerConfigService.getMaxOverfillBps()` / `getMaxConsecutiveClamps()`;
  `FXFiller.isHalted()` / `resetHalt()`; the `HaltControl` type and
  `FillerRuntime.haltControls` / `OperatorContext.haltControls`.
- **HTTP:** `POST /api/reset-halt` (now an unknown route, `404`) and `halted` in
  `GET /api/status`.
- **Dashboard and wizard:** the "Overfill protection needs attention" banner with its
  "Reset halt" button, and the wizard's "Overfill protection" tuning step.

Bids are unchanged: each still offers the matched limit order's whole offer, capped by
what the order has left and by the wallet, and FXFiller never refuses orders on its own.
