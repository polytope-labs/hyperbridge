# Flow

AI-maintained map of how code paths in `sdk/packages/sdk` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## How a solver's bid gets signed

`SubmitBidOptions.solverSigner` is a `SigningAccount` (`src/types/index.ts`), supplied by the caller. `@hyperbridge/simplex` does not pass its `Signer` directly: the payload parameter types differ (`unknown` here, `TypedDataPayload` there), so its `ContractInteractionService` adapts at the call site with `sdkSigningAccount(signer)`.

1. The caller assembles the bid and calls `prepareSubmitBid` (`src/protocols/intents/BidManager.ts`), passing the solver account, nonce, entry point, gas limits, pre-built ERC-7821 `callData`, and any `paymasterAndData`.
2. `BidManager` builds the v0.7-packed `PackedUserOperation` with an empty signature, then checks the nonce key binds the order commitment and session key (`CryptoUtils.bidNonceKey`) — a mismatch is warned about, not thrown, and fails on-chain validation later.
3. It signs `CryptoUtils.packedUserOpTypedData(userOp, entryPointAddress, chainId)` with **`solverSigner.signTypedData`** — the only `SigningAccount` member this package calls, and it passes the payload alone: the chain id the backend might need is already in `domain.chainId`. Signing the typed data rather than the digest produces the same signature `SolverAccount._rawSignatureValidation` recovers, while leaving the payload legible to a custody backend's policy engine.
4. The returned signature is prefixed with the order id (`concat([order.id, solverSignature])`) — that concatenation, not the bare signature, is what goes on the UserOperation.

`signTypedData` is the interface's only member: `signMessage`, `signRawHash`, and `signTypedData`'s chain-id argument were all removed on 2026-08-18 as uncalled. `GasEstimator`'s `signMessage` call is viem's method on a locally derived account, unrelated to `SigningAccount`.

## How the phantom order poll reads Hyperbridge

`IntentsCoprocessor.pollPhantomOrders` (`src/chains/intentsCoprocessor.ts`) drives every read over the HTTP api from `http()`, never the websocket. `http()` derives the endpoint from the websocket provider's own endpoint (`deriveHttpUrl`) and builds an `ApiPromise` on an `HttpProvider` with its response cache disabled (capacity 0); the connect is `isReadyOrError` raced against `HTTP_CONNECT_TIMEOUT_MS`, and a failed connect is not cached.

Each tick, skipped if the previous one is still running:

1. `chain_getHeader()` for the head. No block hash, so polkadot-js never caches it — always a live request.
2. On the first successful head read the cursor is set to `head - 1 - lookbackBlocks`; if `head <= cursor` the tick ends.
3. For each block from `cursor + 1` to `min(head, cursor + maxBlocksPerPoll)`, `getPhantomOrdersInBlock` calls `chain_getBlockHash(n)`, then `api.at(hash)`, then `system.events` at that block. `api.at` resolves a registry through `getBlockRegistry`: reused by `lastBlockHash` or by runtime version where it can, otherwise `chain_getHeader(hash)` followed by `state_getRuntimeVersion(parentHash)` — which is why a failure there logs the parent's hash, not the scanned block's.
4. The cursor advances per block, only once that block's events were read. A failure leaves it in place and fires `onError`; the next tick re-reads the same block with the same parameters.

Step 4 is what made the provider cache dangerous: with caching on, re-reading a block whose `state_getRuntimeVersion` had rejected was answered by the cached rejection rather than a fresh request (fixed 2026-08-21). In `@hyperbridge/simplex` one `HyperbridgeScanner` owns this poll and fans `onError` out to every subscribing filler, so a single failed tick logs once from the scanner and once per filler.

The cadence is `intervalMs` when given, otherwise `phantomPollIntervalMs()`: 6s on Gargantua, 15s elsewhere, decided by the runtime's `specName` read over the same HTTP api, falling back to 15s if that read fails.

## How a phantom order's bids become one price per leg

`aggregatePhantomBids` (`src/protocols/intents/phantom-aggregation.ts`) is the whole path, run by the indexer (`handlePhantomOrderPrices.handler.ts`) and by simplex's phantom E2E test against the same code.

1. Resolve the destination chain's RPC, EVM chain id, and `SolverAccount`. Any of the three missing means no snapshot at all — an unverified quote must never reach the price — and `fetchBidsForOrder` then pulls the bid set from the Hyperbridge node.
2. Per bid: SCALE-decode the UserOperation, pull the inner `fillOrder` out of its ERC-7821 batch (`extractFill`), and reject it unless the order in that calldata commits to the order being priced. The solver's signature is checked over the EntryPoint userOpHash (`isVerifiedSolverBid`), and a solver already counted for this order is skipped, so one bid copied under N fillers still counts once.
3. Decode the paymasterAndData declaration — accepted source chains and declared V4 position tokenIds — which the userOpHash covers, so it is as authentic as the quote.
4. **Price adjustment**: if the declaration names positions, every leg amount is multiplied by `(10_000 - UNISWAP_QUOTE_HAIRCUT_BPS) / 10_000`. This happens before the `solverAmount !== 0n` filter, so a quote the haircut rounds away is read as a declined leg.
5. Weight each quoted leg by the solver's deliverable inventory in *that leg's* output token on the destination chain: ERC-20 balance + redeemable vault shares (`getBalance`) plus the withdrawable side of any declared position the solver actually owns on-chain (`readPosition`, filtered by owner). Then sweep the solver's whole inventory into `lpBalances` once per bid.
6. Per leg: drop every zero-weight quote — from the median, from `bidCount`, and from `bidders` alike — and drop the leg entirely if none is left. Otherwise `weightedMedian` picks the price, and `lowestPrice`/`highestPrice` are set to that same median rather than the raw bid extremes.

A malformed bid is skipped and the rest are priced; a `PhantomRpcError` aborts the whole run instead, because a partial bid set publishes a confident price built from whichever bids happened to be readable.
