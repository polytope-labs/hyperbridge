# SDK: 50% order-fee gas-price bump for BNB Chain and Polygon

`IntentGateway.quoteOrderFees()` (and `execute()` / `executeBest()` when `order.fees` is `0n`)
now prices cross-chain order fees originating on BNB Chain mainnet (`EVM-56`) and Polygon
mainnet (`EVM-137`) with the same 50% gas-price headroom already used for Ethereum mainnet
(`EVM-1`). All other source chains, including testnets, keep the 10% default; same-chain quotes
and direct `estimateFillOrder()` calls remain unbumped.

The policy lives in `ORDER_FEE_GAS_PRICE_BUMP_POLICY` in
`sdk/packages/sdk/src/protocols/intents/IntentGateway.ts`. Released in `@hyperbridge/sdk` 2.8.22.
