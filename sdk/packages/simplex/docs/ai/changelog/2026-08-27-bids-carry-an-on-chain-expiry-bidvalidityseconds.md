# 2026-08-27 — Bids carry an on-chain expiry (`bidValiditySeconds`)

Every bid this filler signs now sets `FillOptions.validUntil`, so `fillOrder` reverts `FillExpired` once the quote
has gone stale. Configured as `simplex.bidValiditySeconds`, default 300 (5 minutes).

A bid is a firm price the order placer takes up whenever they choose, and nothing bounded that window:
`order.deadline` is placer-chosen with no ceiling, and `enqueueRetraction` only clears the bid on Hyperbridge, which
has no effect on the destination chain. A bid signed at one rate stayed executable indefinitely and was exercised only
if the rate moved against us — a written option on this filler's inventory, at no premium. Volatile pairs
(USDC/CNGN) are the worst case, since the naira reprices in steps rather than drifting.

Operators configure seconds because that is the unit the risk is in; the contract compares block numbers, so the
value is converted per destination chain from the chain's nominal block time (`Chain.blockTime`, milliseconds in
viem), with a 30-second discovery allowance added before the conversion and the result rounded up — seconds rather
than a block count, because the lag between reading the head and the fill landing is wall-clock and does not scale
with block time.

`buildApprovalAndFillCalldata` encodes through the SDK's version-aware codec, since gateways predating the field take
a differently-selectored `fillOrder`. On such a chain the bound is dropped — there is nowhere to put it — and the
filler warns once per chain rather than silently believing itself protected.

Found by the scheduled IntentGateway/Simplex security audit.

Files: `src/services/ContractInteractionService.ts`, `src/services/FillerConfigService.ts`,
`src/config/filler-toml.ts`, `src/config/abis/IntentGatewayV2.ts`, `src/core/boot.ts`,
`filler-config-example.toml`, `src/tests/services/bid-validity-config.test.ts`.
