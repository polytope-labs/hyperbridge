# 2026-08-26 — `decimals()` read failures fall back to the asset registry instead of guessing 18

`ContractInteractionService.getTokenDecimals` previously swallowed a failed on-chain `decimals()`
read and returned a hardcoded 18. That value flows into `computeLegPolicyOutput`, which scales
`policyMaxOutput` by `10 ** decimals` — so a 6-decimal token (USDC/USDT/cNGN) misread as 18
inflates the computed payout by 10^12. Since the overfill clamp is disabled, nothing bounds the
result back to the user's requested output, and the filler would size the leg against its whole
wallet balance.

The correct values were already in the tree: `chain.ts` carries a per-chain `tokenDecimals` table
and `ChainConfigService.getAssetMetadataByAddress` resolves it by address. Nothing in simplex
consulted it — `CacheService.tokenDecimals` starts empty and its only writer is the success path
of the very read that just failed. The catch branch now consults the registry through a new
`FillerConfigService.getAssetDecimalsByAddress` delegator, and raises a hard error when neither
the RPC nor the registry can supply a value — there is no safe guess, so the order is skipped
instead. Every caller was audited to confirm a throw skips one order rather than escaping:
`initCache` runs unawaited from the constructor and now swallows its own failures, so a boot-time
RPC hiccup cannot become an unhandled rejection.

Found by the scheduled IntentGateway/Simplex security audit.

Files: `src/services/ContractInteractionService.ts`, `src/services/FillerConfigService.ts`,
`src/tests/services/ContractInteractionService.decimals.test.ts`.
