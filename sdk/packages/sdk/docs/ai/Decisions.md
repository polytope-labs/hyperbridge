# Decisions

AI-maintained record of non-obvious choices made in `sdk/packages/sdk`: what was decided, what the alternatives were, and why. Read this before changing related code so a later change does not silently undo a deliberate trade-off.

Entry format: heading with the decision, then alternatives considered and the reasoning. Newest first.

## 2026-08-27 — The bid expiry is a signed EIP-712 field, not a validation-time check

Chosen: the solver signs `BidValidity(bytes32 userOpHash,uint48 validUntil)` and `SolverAccount` returns `validUntil`
to the EntryPoint as a validity range. The account itself never judges whether the expiry is reasonable.

It cannot. ERC-7562 forbids the `TIMESTAMP` opcode during validation, so the account has no way to compare `validUntil`
against the current time — capping the tenor is necessarily the signer's job (`bidValiditySeconds` on the filler).
The account's role is narrower but load-bearing: make the expiry unforgeable, and hand it to the one component that
*is* allowed to enforce it.

Alternatives rejected:

- *Carry `validUntil` in an unsigned part of the operation.* Cheapest, and wrong: `userOpHash` does not cover
  `op.signature`, so a replayer would just rewrite the expiry and revive a dead bid. Everything about this fix depends
  on the expiry being inside the digest.
- *`keccak256(userOpHash ‖ validUntil)` instead of EIP-712.* Fewer bytes and no domain separator. Rejected because it
  hands the signer an opaque 32-byte digest. The existing `packedUserOpTypedData` exists specifically so signing
  infrastructure can see what it is signing, and here that property is worth more than usual: a named `validUntil`
  field is what lets an MPC/TEE policy engine refuse to sign a bid whose tenor is too long. Since the account cannot
  check the tenor on-chain, the signer is the only place that check can live — so the payload has to be legible there.
- *Re-hash the whole `PackedUserOperation` with `validUntil` appended.* Keeps full-operation visibility, but duplicates
  the EntryPoint's own hashing inside `validateUserOp` — more gas in the validation phase and a second copy of a
  consensus-critical encoding to keep in sync. Binding to `userOpHash` gets the same coverage transitively.
- *An on-chain ceiling like `validUntil <= block.timestamp + 1 days`.* Would have made the contract a backstop against
  a buggy SDK. Not expressible: same ERC-7562 restriction.

The domain's `verifyingContract` is `address(this)` — under EIP-7702 the solver's own EOA — so a bid cannot be
replayed against a different solver account. Rejecting `validUntil == 0` is part of the fix rather than a nicety:
zero is how the old unbounded behaviour is spelled, so accepting it would leave the hole open.

Rollout: `SolverAccount` is reached by EIP-7702 delegation, so a new deployment lives at a new address and solvers
migrate by re-delegating. Old and new can coexist per solver, which is why the contract refuses the 162-byte format
outright rather than accepting both — a compatibility window would keep the unexpiring path alive for exactly as long
as it existed.

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
