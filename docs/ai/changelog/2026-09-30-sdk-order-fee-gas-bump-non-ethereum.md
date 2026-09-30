# SDK: 100% order-fee gas-price bump for non-Ethereum source chains

`IntentGateway.quoteOrderFees()` (and `execute()` / `executeBest()` when `order.fees` is `0n`)
now prices cross-chain order fees originating on any chain other than Ethereum mainnet with
100% gas-price headroom (previously 10%). Ethereum mainnet (`EVM-1`) keeps 50%. Same-chain
quotes and direct `estimateFillOrder()` calls remain unbumped.

The percentages are `ETHEREUM_ORDER_FEE_GAS_PRICE_BUMP_PERCENT` and
`DEFAULT_ORDER_FEE_GAS_PRICE_BUMP_PERCENT` in
`sdk/packages/sdk/src/protocols/intents/IntentGateway.ts`. Released in `@hyperbridge/sdk` 2.8.22.
