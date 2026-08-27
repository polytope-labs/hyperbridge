# ChangeLog

AI-maintained log of code changes in `sdk/packages/sdk`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/sdk/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-08-27 — `FillOptions` carries a `validUntil`, and `fillOrder` has two shapes

`FillOptions` gained `validUntil` (a block number; `0n` means unbounded), enforced by `fillOrder`, which now reverts
`FillExpired` past it. Adding a field changes the enclosing function's selector, so `fillOrder` exists in two
incompatible shapes — `0x5cfb1ea5` (v1) and `0xa5470064` (v2) — and gateways upgrade per chain, so both are on the
wire at once.

New `protocols/intents/fillOrderCodec.ts` owns that: `getFillOptionsVersion` reads the gateway's ERC-1967
implementation slot and matches the address against a set of known pre-`validUntil` implementations, defaulting to
v2; `encodeFillOrder` emits the matching shape and `decodeFillOrder` reads either. There is no version getter on
the contract — EIP-1967 has no version field either, and the implementation address is the value the proxy already
updates on upgrade. `GasEstimator` encodes through it so estimates do not revert on a missing function; `BidManager` and
`phantom-aggregation.extractFillData` decode through it so bids built against an older gateway are still priced in
rather than dropped.

Why the field exists: a solver bidding through the coprocessor signs this calldata and then has no further say in
when it is used. The order's `deadline` is placer-chosen with no ceiling, retracting the bid on Hyperbridge does not
reach the destination chain, and the placer holds the session key — so a signed bid stayed executable indefinitely
and was taken up only once the rate had moved against the solver. `validUntil` rides in the calldata, which
`userOpHash` already covers, so it is tamper-proof without touching the signature format.

`fillOrder` also no longer calls `IIntentPriceOracle.recordSpread`. It passed `order.inputs` and `options.outputs`
to a stateful oracle verbatim, and neither is validated against anything that costs the caller money — the escrow
lives on the source chain and is never consulted on the destination side. `Params.priceOracle` is left in place
because removing a field from a storage struct behind an upgradeable proxy shifts the layout.

Found by the scheduled IntentGateway/Simplex security audit.

Files: `src/protocols/intents/fillOrderCodec.ts` (new), `src/protocols/intents/GasEstimator.ts`,
`src/protocols/intents/BidManager.ts`, `src/protocols/intents/phantom-aggregation.ts`,
`src/protocols/intents/index.ts`, `src/abis/IntentGatewayV2.ts`, `src/types/index.ts`,
`src/tests/fillOrderCodec.test.ts` (new), `src/tests/phantomAggregation.test.ts`, plus
`evm/src/apps/IntentGatewayV2.sol`, `evm/src/apps/intentsv2/IntentsBase.sol` and
`sdk/packages/core/contracts/apps/IntentGatewayV2.sol`.

## 2026-08-25 — Intent quotes use aggregate indexed pool rates by default

`IntentGateway.quoteIntent` now prices orders from the pair-centric indexer's depth-weighted aggregate `LiquidityPool.buyRate` and `sellRate`. Source and destination chains resolve the configured token deployments, while the quote converts the pool's whole-token rate into raw amounts with configured decimals, applies the source gateway protocol fee, and exposes the selected rate and timestamp in metadata. Reverse sell-rate reciprocals round up so quotes do not overpromise output. Phantom snapshot and Uniswap V4 pricing remain explicit compatibility strategies. Live sequential tests cover exact-input USDC to cNGN and exact-output cNGN to USDC across BSC and Base, including their different token decimal scales. The dead `binance.llamarpc.com` BSC default was replaced with `bsc-rpc.publicnode.com` after it blocked those live checks.

Files: `src/configs/chain.ts`, `src/protocols/intents/IntentGateway.ts`, `src/protocols/intents/LiquidityEngine.ts`, `src/protocols/intents/index.ts`, `src/protocols/intents/quote/index.ts`, `src/protocols/intents/quote/indexedRates.ts`, `src/protocols/intents/quote/types.ts`, `src/tests/sequential/intentGateway.test.ts`, `package.json`, `CHANGELOG.md`, `docs/ai/ChangeLog.md`, `docs/ai/Decisions.md`, `../../../docs/content/developers/sdk/api/intent-gateway.mdx`, `../../../docs/content/developers/evm/intent-gateway/placing-orders.mdx`.

## 2026-08-24 — Pool-priced phantom bids are haircut 30bps before aggregation

A phantom bid that declares Uniswap V4 positions is quoting off those pools, and a pool price is what a trade gets before the pool takes its fee — so the amount such a bid names is richer than what the solver clears once the sourcing swap goes through. `aggregatePhantomBids` now nets 30bps out of every leg amount on a bid whose declaration carries positions, before the quote reaches the zero-check, the weighted median, or the bidder rows. Non-declaring bids are untouched: wallet inventory has already paid its cost of goods.

The haircut runs off the declaration rather than the positions that survive the on-chain ownership check, so a quote is priced on the same basis the solver priced it on — including on a chain with no V4 contracts configured, where declared positions contribute no weight. A leg whose amount the haircut rounds to zero falls into the existing declined-leg path.

`UNISWAP_QUOTE_HAIRCUT_BPS` and `applyUniswapQuoteHaircut` are exported from both `@/protocols/intents` and the `intents-helpers` sub-path, so the indexer and simplex read the same number instead of restating it.

Files: `src/protocols/intents/phantom-aggregation.ts`, `src/protocols/intents/index.ts`, `src/intents-helpers.ts`, `src/tests/phantomAggregation.test.ts`.

## 2026-08-21 — HTTP provider no longer caches responses, so a failed phantom poll read is retried

`IntentsCoprocessor.http()` built its `HttpProvider` with polkadot-js defaults, which cache every request that names a block hash by storing the request promise itself — rejected promises included — under a 30s TTL refreshed on every hit. `pollPhantomOrders` retries the block it failed on with identical parameters each tick, so after one reset connection (`fetch failed` / `ECONNRESET`) every later tick got the same rejection back from the cache in a few milliseconds — the same `state_getRuntimeVersion(parentHash)` hash in every log line — and the node never saw another request. With the 15s poll interval inside the 30s TTL the entries never expired, so a filler stayed wedged until restart, placing no phantom bids. The provider is now constructed with cache capacity 0, which sends every request to the node. The cache bought nothing for this api: the poll reads each block once, and `api.at(hash)` reuses registries at the api layer regardless.

`intentsCoprocessorHttpCache.test.ts` pins the dependency's behaviour against a stub node that RSTs one request, and that the provider the coprocessor builds retries it.

Files: `src/chains/intentsCoprocessor.ts`, `src/tests/intentsCoprocessorHttpCache.test.ts`.

## 2026-08-19 — Fix the refund-POST gas pin #1144 left behind

#1144 split `CANCEL_MESSAGE_GAS = 800_000n` into `SOURCE_GET_RESPONSE_GAS` and `REFUND_POST_GAS`, both 1M, but left `orderCanceller.test.ts` asserting the POST at 800k — main's own CI has failed the concurrent-sdk step since it merged, and every PR cut from it inherited the red check. The pin now matches the shipped constant, with a comment naming the origin so the next reprice updates both.

Files: `src/tests/orderCanceller.test.ts`.

## 2026-08-18 — `SigningAccount` shrinks to `signTypedData` alone

`SigningAccount` is the contract a solver's signing backend satisfies to submit bids (`SubmitBidOptions.solverSigner`). It declared `signMessage(messageHash, chainId)`, which nothing in this package ever called: bids are signed as EIP-712 UserOperations in `BidManager.prepareSubmitBid` via `signTypedData`, and `GasEstimator`'s one `signMessage` call is viem's own method on a locally derived account, not this interface. The requirement propagated out to every implementer — including `@hyperbridge/simplex`, whose public `Signer` satisfied this type (it no longer extends it; simplex adapts with `sdkSigningAccount` at the two call sites) — so removing it here is what let that interface shrink to what a solver actually needs.

`signTypedData`'s second parameter went the same way. EIP-712 carries the chain id in `domain.chainId` — that is what the digest covers — and `BidManager` was passing it alongside a payload that already contained it, for the benefit of no implementation in this package.

`signRawHash` followed in the same day's follow-up: once simplex's `Signer` required `signAuthorization` and `signTransaction`, no caller in either package handed a raw digest to the interface, leaving `signTypedData` as the whole contract.

Type-only narrowing: it can break a caller of a removed member, and there are none inside the workspace.

Files: `src/types/index.ts`, `src/protocols/intents/BidManager.ts`.
