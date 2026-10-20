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
custom `[assets]` entry (which maps symbols to addresses and declares no decimals), still
takes the chain's `decimals()` answer and caches it. Nothing cross-checks it, and no bound
on the bid against the order's output exists any more, so a wrong answer is limited only
by the wallet balance and the funding venues.

The read-failure fallback to the registry is unchanged and still not cached: it covers a
transient RPC failure without pinning a registry value for the process. The native token
still short-circuits to 18 without a read. A cached value was cross-checked when it was
cached, and `setTokenDecimals` has no other caller.

## Overfill protection removed

The overfill clamp was disabled for limit-order pricing and the self-halt it fed could
never trip, so both are gone along with everything that configured or reported them:

- **Config:** `[simplex.overfillProtection]` (`maxOverfillBps`, `maxConsecutiveClamps`)
  is no longer read. `validateConfig` accepts it, because `simplex init` wrote the section
  uncommented for existing operators and refusing it would crash-loop their deployments.
  Boot logs a warning (`[simplex.overfillProtection] is ignored: overfill protection was
  removed …`) when `simplex.overfillProtection !== undefined`, so a library caller passing
  `{ overfillProtection: undefined }` is not warned. An update run of `simplex init` does
  not write the section into the new file, and logs `Dropped [simplex.overfillProtection]`
  once that file is written; "Start the filler with it as-is" leaves the file untouched, so
  boot's warning is what the operator sees.
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
