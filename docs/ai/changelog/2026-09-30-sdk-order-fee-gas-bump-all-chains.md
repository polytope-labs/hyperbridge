# SDK: 50% order-fee gas-price bump for every source chain

`IntentGateway.quoteOrderFees()` (and `execute()` / `executeBest()` when `order.fees` is `0n`)
now prices every cross-chain order fee with 50% gas-price headroom, regardless of source chain.
Previously only Ethereum mainnet used 50% and all other source chains used 10%. Same-chain quotes
and direct `estimateFillOrder()` calls remain unbumped.

The percentage is `CROSS_CHAIN_ORDER_FEE_GAS_PRICE_BUMP_PERCENT` in
`sdk/packages/sdk/src/protocols/intents/IntentGateway.ts`. Released in `@hyperbridge/sdk` 2.8.22.
