# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/sdk`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-27 — The bid expiry rides in `FillOptions`, not in the bid signature

Chosen: `validUntil` is a field on `FillOptions`, checked by `fillOrder` at execution.

The alternative was to put the expiry in the ERC-4337 signature blob and return it from
`SolverAccount.validateUserOp` as a `validUntil` validation range — the mechanism 4337 provides for exactly this.
That was built first and then abandoned, for reasons worth recording:

- **It needs a new signed field.** `userOpHash` does not cover `op.signature`, so an expiry carried there is
  rewritable by whoever replays the bid. Making it tamper-proof meant changing what the solver signs (an EIP-712
  `BidValidity` digest) and widening the selection signature 162 → 168 bytes.
- **That is a `SolverAccount` redeploy.** The account is reached by EIP-7702 delegation, so a new version means a new
  address and every solver re-delegating — and, since the old account keeps accepting the old format forever, it
  retires nothing already signed.
- **`FillOptions` needs none of that.** The options are part of `callData`, which `userOpHash` *does* cover. The
  expiry is authenticated for free, with no signature format change, no account redeploy, and no migration.
- **It covers more.** The signature-side check only bounds solver-selection bids. A check in `fillOrder` bounds every
  path into it.

The cost is that this fires at execution rather than validation: an expired bid is included, the nonce is consumed
and the account pays that op's gas, where a validation-time range would have had the bundler drop it for free. That
is a bounded, one-off cost per bid — and consuming the nonce permanently retires the bid, which the validation-time
version does not do. Fund loss, the thing that matters, is prevented either way.

Denominated in blocks rather than a timestamp so it reads against the same clock as `order.deadline` (`_blockNumber()`,
the L2 block number where those differ), and so the two cannot disagree about what "expired" means.

`0` means unbounded. That is the right default for a solver filling directly — it is only exposed to its own
staleness — and it keeps every existing caller working. The protection is opt-in by the party that needs it.

## 2026-08-27 — The implementation address identifies the FillOptions shape

Chosen: `getFillOptionsVersion` reads the ERC-1967 implementation slot and checks the address
against `LEGACY_FILL_OPTIONS_IMPLEMENTATIONS`, a set of implementations deployed before
`validUntil` existed. Anything else is v2.

EIP-1967 standardises three slots — implementation, admin, beacon — all holding addresses. There
is no version field in the spec to read, and OZ `Initializable`'s `uint64` only moves under
`reinitializer(N)`, which this contract does not use. The implementation address is the only value
the proxy actually updates on upgrade, so it is what identifies the deployed code.

The list is of **legacy** implementations, not current ones, so the default is v2. That direction
is the whole point: a newly shipped implementation needs no edit here, and once every deployment
is upgraded the set is vestigial and still correct. Listing known-good implementations instead
would be the version constant this replaced wearing a different hat — a value someone must
remember to update on every upgrade, where forgetting breaks every fill on that chain.

Only v2 answers are cached, keyed by proxy address. A deployment can move from legacy to current
but never back, so a v2 result is true forever; caching a v1 result would pin the old encoding
across the very upgrade that changes it, since the proxy address does not move and nothing would
invalidate it. A still-legacy gateway therefore costs one storage read per fill, an upgraded one
costs none.

Also considered and dropped: scanning the implementation's runtime code for the v2 `fillOrder`
selector. It needs no address list and self-updates, and the selector does survive `via-ir` and
the optimizer — but it is a heuristic (a 4-byte sequence can appear in non-dispatcher data), and
both failure directions break every fill on the chain, since the two shapes cannot decode each
other. An address match is exact.

Earlier still, and rejected: a `fillOptionsVersion()` getter on the contract. A hand-maintained
integer is a second source of truth that answers what a deployment claims rather than what it can
decode.

## 2026-08-25 — Intent quotes default to directional indexed rates without fallback

Chosen: `quoteIntent` defaults to an `indexed_rates` strategy that selects the depth-weighted aggregate `LiquidityPool.buyRate` for base-to-quote orders and `sellRate` for quote-to-base orders. Source and destination chains resolve the configured token deployments; raw amounts are calculated from the indexer's 18-decimal whole-token pool rate and both tokens' configured decimals. A missing directional rate is an error.

Alternatives considered:

- **Keep defaulting to the legacy directional Phantom snapshot.** Rejected: those snapshots resolve through a canonical Base market and do not use the pair-centric pool rate, so quotes can disagree with the indexer's current market.
- **Quote directly from one source/destination pair of `PoolChainLiquidity` rows.** Rejected: those rows are inputs to the indexer's pool price. `LiquidityPool.buyRate` and `sellRate` are the maintained depth-weighted merge of fresh chain samples and are the intended market-level quote.
- **Silently fall back to Phantom or Uniswap when a rate is absent.** Rejected: an order would be priced from a different market than the caller requested, hiding stale or incomplete indexer coverage and producing another unfillable quote.
- **Remove the old strategies immediately.** Rejected for compatibility: callers that explicitly select them can continue doing so, while all calls without a strategy use the corrected path.

The result uses a new `indexed_rates` discriminant and includes the selected rate side, value, timestamp, pair symbols, chains, and protocol fee. This makes the price used to construct the order inspectable without exposing indexer internals.

The public sell rate is the reciprocal of the indexer's base-per-quote direction. That reciprocal rounds up at 18 decimals: rounding down would let a quote-token-to-base-token order request slightly more base output than the indexed direction supports. Buy rates are already indexer-floored outputs and remain unchanged.

## 2026-08-24 — The 30bps pool haircut keys off the declaration, and lands on price only

Chosen: in `aggregatePhantomBids`, haircut a bid's quoted leg amounts by 30bps when its paymasterAndData declaration names Uniswap V4 positions.

Alternatives considered:

- **Keying off the positions that pass the ownership check** (the `positions` array, not `declaration.uniswapV4Positions`). Rejected: those are filtered by owner and skipped entirely when the chain has no `positionManager`/`stateView` configured, so the same bid would be priced two different ways depending on indexer config. The declaration is what the solver signed, and it is what says "this quote came off a pool".
- **Haircutting the weight instead of the price.** Rejected: the weight is deliverable inventory, read on-chain, and it is also what the liquidity sweep reports — discounting it would make a provider's reported inventory and the depth attributed to it disagree, which the sweep exists to prevent. The fee is a cost of the trade, not a reduction in the size held.
- **Applying it after the median, to the leg's published price.** Rejected: the median is liquidity-weighted across bids, so haircutting the aggregate would also discount wallet-funded quotes that never pay a pool fee, and it would change which quote wins the median only by accident. The haircut belongs on the individual quote, before it competes.
- **Making the rate configurable per chain or pool.** Deferred: the positions these bids declare sit in 30bps-tier pools, and a knob invites the number to drift out of sync between the indexer and simplex. A single exported constant is easy to widen into a lookup if a different tier ever shows up.

Why 30bps at all: without it, a pool-priced bid reads richer than a wallet-funded one on a fee it has not yet paid, so it wins the median and the published price is one nobody can actually execute at.

A consequence worth knowing: a leg amount small enough that the haircut rounds it to zero now takes the "solver declined this leg" path. That is the correct reading — a quote that rounds away is not a price — and it only bites at dust amounts.

## 2026-08-21 — The coprocessor's HTTP provider runs with its response cache off

Chosen: `new HttpProvider(httpUrl, {}, 0)` — capacity 0 disables polkadot-js's per-provider LRU outright, so every `send` reaches the node.

Alternatives considered:

- A shorter TTL, or a poll interval longer than the TTL. The TTL slides — each hit refreshes it — so any poll faster than the TTL keeps a cached rejection alive indefinitely, and a slower poll would heal after one TTL only by coupling the cadence to a cache constant; the poll was made faster on purpose (#1138).
- A provider wrapper, or an upstream fix, that drops an entry when its promise rejects. Correct, but more code holding state this api has no use for, and an upstream fix still needs the workaround until it ships: polkadot-js master has the same `send` as 16.5.6.
- Keeping the cache and retrying inside the poll tick. A retry hits the same cache key and gets the same rejection; only a different request shape would get past it.

Why: the HTTP api exists for one-shot reads whose whole value is that nothing persists between calls (see the `http()` and `pollPhantomOrders` doc comments). A promise cache is exactly such persistence, and its only effect on this api was the failure mode: the poll reads each block once, so there are no repeat hits to serve, and `api.at(hash)` already reuses decoded registries at the api layer. Disabling it removes the hazard without adding a mechanism.

## 2026-08-18 — `SigningAccount` describes only what the SDK calls

(Amended the same day: `signRawHash` was removed in a follow-up commit; see the closing paragraph.)

Chosen: drop `signMessage` from `SigningAccount`, leaving `signRawHash` and `signTypedData`.

Alternative considered: leaving it in place, since removing a member from a published interface is a public API change.

Why: the interface exists so a caller can hand the SDK a signing backend, and every member it declares is a cost paid by every implementer. `signMessage` was never invoked — bid UserOperations are EIP-712-signed through `signTypedData` — so the cost bought nothing, and it forced downstream signer abstractions (notably `@hyperbridge/simplex`'s `Signer`) to carry a dead method into their own public surface. The narrowing is safe in the direction that matters: implementers with an extra method still satisfy the type, and only a caller of `solverSigner.signMessage` would break, of which there are none.

`signTypedData`'s `chainId` parameter went for a sharper reason than disuse: it is redundant with EIP-712 itself. The digest covers `domain.chainId`, `BidManager` built payloads that always set it, and the one downstream implementation that needed a chain id (MPCVault, for its request envelope) could read it from the payload — and defaulted the argument to `1` when absent, which is a bad failure mode for a signature.

`signRawHash` went the same day, in the follow-up that made `signAuthorization` and `signTransaction` required members of simplex's `Signer`: once those are guaranteed, nothing in either package computes an authorization digest for the signer to raw-sign, and keeping the member would have recreated the `signMessage` situation. `SigningAccount` is down to the one method this package invokes — `signTypedData`.
