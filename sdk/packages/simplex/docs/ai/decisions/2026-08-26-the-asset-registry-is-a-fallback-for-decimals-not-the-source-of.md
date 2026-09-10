# 2026-08-26 — The asset registry is a fallback for `decimals()`, not the source of truth

Chosen: `getTokenDecimals` reads `decimals()` on-chain first, falls back to
`FillerConfigService.getAssetDecimalsByAddress` when that read throws, and **throws** if neither
source can supply a value. The registry value is **not** written into `CacheService`.

The on-chain value is the real one — the registry is curated data that can drift from a
redeployed or migrated token. Keeping the read authoritative means the registry only ever covers a
transient RPC failure. Not caching the fallback is the load-bearing half: `CacheService.tokenDecimals`
has no TTL, so caching a registry value during an outage would pin it for the lifetime of the
process and silently outlive the failure that justified it. Leaving the cache empty means the next
call retries the read and caches the real answer.

Alternatives rejected:

- _Registry first, on-chain only for unregistered tokens._ Fewer RPC calls, and it would make the
  bad path unreachable rather than merely survivable. Rejected because it inverts which value is
  authoritative: a stale registry entry would then silently override the live token on every fill,
  and the registry is edited by hand.
- _Return 18 when the registry has no entry either._ What this replaced. `decimals` scales
  `policyMaxOutput` by `10 ** decimals`, so a wrong value does not degrade a fill — it changes its
  size by orders of magnitude. There is no safe guess, so the function now throws and the order is
  skipped.

Making it throw required auditing every caller, since `getTokenDecimals` had never thrown:

- `calculateProfitability` and `sizeOrder` reach `IntentFiller.evaluateOrder`, inside the
  `try/catch` opened at `core/filler.ts:604` — the order is logged and skipped.
- `quotePhantomFill` is wrapped at `core/filler.ts:1134` — the leg goes unquoted.
- `getOrderUsdValue` is wrapped at `core/filler.ts:688` — sizing falls back to `baseInputUsd`.
- `initCache` was the one real hazard: it runs **unawaited** from the constructor, so a rejection
  would have been an unhandled rejection and killed the process rather than skipping an order. It
  now swallows its own failures (and the call site adds a `.catch`), so strictness applies where a
  single order can be dropped, not at boot. Note this hazard pre-existed via
  `getFeeTokenWithDecimals`, which could already throw there.
