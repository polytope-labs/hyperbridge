# Flow

AI-maintained map of how code paths in `sdk/packages/simplex` actually execute, so that when something breaks you can tell whether the fault is upstream or downstream of where the symptom appears. Only flows that have been read and verified are documented; coverage grows as areas of the package are touched.

## Order intake: what reaches the filler at all

`ChainScanner` polls `eth_getLogs`, rebuilds each `OrderPlaced` log into an `Order` via
`reconstructOrdersFromLogs`, and hands it to every subscriber. `EventMonitor` is the per-filler
subscriber and applies three filters in order (`src/core/event-monitor.ts`):

1. **Chain.** A shared scanner may carry chains this filler is not configured for, so events are
   matched on `chainId` against the monitor's own set.
2. **Leg count.** The order must have exactly one entry in `inputs` and one in `output.assets`.
   Anything else emits `orderSkipped` with reason `Multi-leg order` and stops here.
3. **De-duplication.** The order id must not be in the seen set (last 5,000 ids). A scanner
   resuming from a cursor re-delivers, and the fill path has no idempotency of its own.

Only what survives all three is emitted as `newOrder`, which `IntentFiller` picks up in its
constructor and passes to `handleNewOrder`. Fills take a separate path: `onFill` narrows on the
filler address — `filler` is `indexed: false` in the ABI, so it can never be a topic filter — and
emits `orderFilledOnChain`.

Phantom orders do not come through here. They are polled from Hyperbridge inside `IntentFiller`,
so the leg-count filter does not apply to them and `quotePhantomLeg` still splits a bundled
phantom order into positional single-pair legs.

## Filling: how a fill amount is sized, and who else spends the same balance

Verified against the reverted Base fill `0x31de53fe...` (UserOp `0xe090acd1...`), traced end to end.

### Sizing, per leg

`FXFiller.evaluateOrder` walks `order.inputs` and sizes each leg independently (`src/strategies/fx.ts`):

1. `targetOutput` = what the curve will pay (`policyMaxOutput`), in every case. This is the amount the leg intends to hand over, and it may exceed what the user asked for — `IntrinsicIntents.fillOrder` takes `solverAmount > totalRequired` and splits the excess between the beneficiary and the protocol (`surplusShareBps`), and it is the same figure `quotePhantomFill` publishes as the pair's quoted rate. A pair's `maxOrderSize` is optional and does not shorten this: it binds earlier and in the other unit, where `computeLegPolicyOutput` rations `token0ForLeg` against the pair's remaining budget before the rate is applied. `desiredOutput` — the user's ask scaled by the same cap — is no longer a ceiling on payout; it survives as the price gate's comparand and in the short-fill logs.
2. `reserve` = the paymaster reserve for this token (`paymasterReserveForToken`, from `src/services/paymaster`) plus every funding venue's `walletReserveForToken`. Only the vault returns a non-zero venue reserve, its configured `minBalance`; `UniswapV4FundingPlanner` returns `0n`. The paymaster half is seeded outside the venue loop deliberately — the loop is empty when no vault and no V4 positions are configured, and that filler still sizes partial fills.
3. `usableWallet` = balance − reserve. `walletContribution` = `min(targetOutput, usableWallet)`.
4. Any shortfall is requested from each funding venue in turn via `planWithdrawalForToken`, which returns ERC-7821 calls and the amount it expects them to credit. The calls accumulate in `fundingCalls`. For V4 that credit is priced from `liquidityRemoval`, the liquidity the encoded DECREASE_LIQUIDITY actually carries — not the liquidity the planner asked for, which the SDK truncates.
5. `finalOutputAmount` = `min(walletContribution + credited, targetOutput)`.

After the loop, `estimateGasFillPost` prices the fill. A cross-chain order then has to clear one more affordability check: `fillOrder` dispatches the escrow-release message and `HyperApp.dispatchWithFeeToken` pulls `dispatchFee` from the same wallet in the destination host's fee token (USDC on Base — often the token just paid out). The fee is only known here, after the funding calls it depends on exist, and a cross-chain order cannot be partially filled, so the order is skipped when the residue will not cover the fee plus the paymaster reserve.

A capped leg is separately gated as a partial fill when the cap actually shortens it — `capFraction.lt(1) && policyMaxOutput < output.amount`. Both halves matter: with the payout unclamped, a curve running far enough above the order's rate covers the whole ask out of a capped slice, which is a full fill.

Whenever `finalOutputAmount < output.amount` the fill is an under-fill and has to clear `partialEligible()` — same chain, no output calldata, no prior partial — or the order is skipped.

The outputs and the funding calls are cached against the order id (`setFillerOutputs`, `setFundingPrepends`). `ContractInteractionService.prepareBidUserOp` reads them back verbatim and signs them into the bid; nothing re-reads the balance between sizing and execution, and the bid is committed, so an amount that was affordable at evaluation must still be affordable when the UserOp lands.

### The batch, in execution order

`callData` is an ERC-7821 batch: the funding calls first (for V4, `PositionManager.multicall(modifyLiquidities)` encoding DECREASE_LIQUIDITY + TAKE_PAIR), then `approve` for the fill amount, then `IntentGateway.fillOrder`, which does the `transferFrom` that moves the output token to the user.

What matters is what happens *before* any of that. The EntryPoint runs paymaster validation first, and `SimplexPaymaster` prefunds by pulling the EntryPoint's worst-case gas cost out of the solver's wallet with `transferFrom`, in whichever stablecoin `selectToken` picked — refunding the unused part in `_postOp`, long after the batch has already run or reverted. So the balance the batch sees is always lower than the balance the sizing saw, by the prefund.

That is what `paymasterReserveForToken` exists to absorb. Without it, a balance-limited fill — `walletContribution == usableWallet == balance` — is sized to the last unit and reverts by exactly the prefund: 28,993 units of USDC against a 14,808,699,383 fill, where 14,377,624,370 (wallet) + 431,075,013 (V4 credit) reproduces the bid amount exactly.

### Which token the paymaster charges

Decided at submit time, not at sizing time, by `buildPaymasterAndData` (`src/services/paymaster/index.ts`): the Circle paymaster if configured and USDC balance >= 1 USDC, else the Simplex paymaster over the first of `[USDC, USDT]` with a balance >= 1 token (`selectToken`), else no paymaster. Because the sizing decision feeds the balances that decide this, the reserve covers both eligible tokens instead of predicting one.

## Signing: from construction to each signature

### Where the signer comes from

There are two entry points, and they meet at `bootFiller`.

1. **Library.** The consumer builds a `Signer` (`privateKeySigner`, `turnkeySigner`, `mpcVaultSigner`, `viemSigner`, or their own) and passes it as `Simplex.start({ signer })` (`src/simplex.ts`). Before anything else, `start` rejects an object that carries a `simplex.signer` block without a `signer` argument. `SimplexConfig` has no such field, so this is a runtime property read: what it catches is a parsed config file.
2. **Binary.** `src/bin/simplex.ts` parses the TOML as `FillerConfigFile` (the library's `FillerTomlConfig` plus the `[simplex.signer]` block), checks the block is present unless watch-only, and calls `signerFromToml` → `validateSignerConfig` → `createSigner`, which dispatches on `type` to one of the three bundled factories. The resolved instance goes into the same `Simplex.start({ signer })` call. The `paymaster-keeper` command does the same thing without a Simplex.

   The parsed object keeps its signer block on the way in — the library ignores the extra key, and `UiServer.persistConfig` regenerates the config file from that same object, so removing it would delete `[simplex.signer]` from the operator's file on the next dashboard edit.

`bootFiller` (`src/core/boot.ts`) then:

- Throws if there is no signer and not every resolved chain is watch-only.
- Passes it to `new ChainClientManager(configService, options.signer)`. When it is absent — watch-only only — `ChainClientManager` substitutes `privateKeySigner(generatePrivateKey())`, a key that exists solely so wallet-client construction has an account; nothing ever signs with it. The runtime records this as `signerless`, and the chain controller enforces it: `setWatchOnly(chainId, false)` throws and `chains.add` defaults to watch-only, so an observer started without a signer cannot be flipped into filling from the throwaway key.
- Reads it back with `chainClientManager.getSigner()` and hands that one instance to `ContractInteractionService`, `UserOpSender`, `IntentFiller`, `FXFiller`, the rebalancers and `PaymasterKeeperService`. There is exactly one signer per solver.
- Logs `EVM signing strategy: <mode>` (`mode ?? "custom"`) — the only place `mode` matters.

### Which method signs what

- **`signTypedData` — the hot path.** Two callers:
  - `ContractInteractionService` builds a bid and calls `sdkHelper.prepareSubmitBid({ solverSigner: sdkSigningAccount(this.signer), … })`; the SDK's `BidManager` signs `CryptoUtils.packedUserOpTypedData(userOp, entryPoint, chainId)`. Signing the typed data rather than the digest yields the same signature the `SolverAccount` recovers, while leaving the payload legible to a policy engine.
  - `UserOpSender.buildSignedUserOp` does the same for self-initiated UserOps — delegation-via-bundler, vault sweep and redeem.
  - `paymaster/permit.ts` signs the EIP-2612 permit that lets the Circle or Simplex paymaster pull USDC/USDT for gas. It takes `Pick<Signer, "signTypedData">`, not the whole signer.

  No caller passes a chain id: every payload carries `domain.chainId`, which is what the digest covers and what MPCVault reads for its request envelope.
- **`signAuthorization`.** `DelegationService.buildAuthorization` calls it for every delegation, with no branching — the signer owns the encoding. Turnkey uses its structured path; MPCVault and any digest-only backend hash `keccak256(0x05 ‖ rlp([chainId, contractAddress, nonce]))` themselves (`viem/utils`' `hashAuthorization`).
- **`signTransaction`.** Every transaction the solver sends: the type-0x04 delegation tx, rebalancing transfers, operator sends. It returns signed RLP, so the backend owns serialisation — MPCVault's vault API and Turnkey's transaction payloads both keep the transaction legible to their policy engines, and `digestSigner` serialises with viem and signs the hash.
- **`address`.** Read directly everywhere the solver's identity is needed (`fillerAddress`, delegation authority, balance lookups, vault initialisation).
- **`mode`.** Logs only: the boot line and the two delegation log lines.

### The viem boundary

`Signer` names no viem type. `accountFor(signer)` (`src/services/wallet/account.ts`) builds the `LocalAccount` viem wants from one, and `ChainClientManager` derives it once in its constructor and hands it to every wallet client. The mapping is:

- `signTypedData` → straight through.
- `signTransaction` → viem's prepared request narrowed by `toSignerTransaction`, which rejects a request with no `chainId` rather than letting a replayable transaction be signed.
- `signMessage` → rejects. viem's `toAccount` requires it; no solver path personal-signs.
- `sign` → not implemented. Nothing calls `account.sign` now that authorizations and transactions are signer operations.

`digestSigner` is the same boundary from the other side: it turns one `sign(hash)` into the three operations, hashing typed data with viem's `hashTypedData`, authorizations with `hashAuthorization`, and transactions with `serializeTransaction` + `keccak256`.

`sdkSigningAccount(signer)` is the last piece: the sdk's `SigningAccount` takes `unknown` typed data where `Signer` takes `TypedDataPayload`, so the two call sites that hand a signer to the sdk go through it.

### Delegation, the one branching path

`DelegationService.setupDelegation(chain)` prefers the bundler: a no-op UserOp with the authorization attached, gas paid by the paymaster in stablecoins (`setupDelegationViaBundler`, signed with `signTypedData` through `UserOpSender`). It falls back to a direct type-0x04 transaction when no paymaster is available or the solver holds no stablecoins, which needs native balance.

The direct path is uniform: `sendDelegationTransaction` calls `walletClient.sendTransaction` with the authorization list and a 650k gas floor, whatever the signer is. viem prepares the transaction and hands it to the derived account's `signTransaction`, which routes to the signer's. `mpcVaultSigner` is where that matters: its structured request has no field for an authorization list, so it detects one and serialises + raw-signs instead, which is the only reason a set-code transaction from an MPC-backed solver installs a delegation at all.

`revokeDelegation` runs the same two steps against the zero address.

### What `viemSigner` derives

`viemSigner(account)` (`src/services/wallet/accounts/viem.ts`) maps a viem `LocalAccount` onto the interface: `address` → `account.address`, `mode` → `account.source` (so `privateKeyToAccount` reports `"privateKey"` and `toAccount` reports `"custom"`), `signTypedData` and `signTransaction` → the account's own.

`signAuthorization` is the one that needs work, because viem makes it optional on an account and the interface does not: the adapter uses `account.signAuthorization` when present (private keys, Turnkey) and otherwise hashes the tuple and signs it with `account.sign`. An account with neither is rejected at construction — a solver that cannot delegate cannot bid, and finding that out at the first fill is worse.

`privateKeySigner` is `viemSigner(privateKeyToAccount(key))`. `turnkeySigner` is `viemSigner(turnkeyAccount)` plus `mode: "turnkey"`. `mpcVaultSigner` uses no viem account: it implements the three operations against `MpcVaultService` directly.

## Phantom probe: curve value -> published price

Verified 2026-08-19 by reading the path end to end and reconciling against live mainnet bids and
the nexus indexer.

```
FXFiller.quotePhantomFill(order)                      src/strategies/fx.ts
  canFill(order)                                      bail if halted / unsupported / one-sided
  resolveOrderLegs(order)                             order legs -> ResolvedLeg[]
  sizeOrder(order, legs, venuePriceMemo())            per-leg notionals ONLY here
  for each leg:
    resolveLegRates(..., legNotionals[i], ...) -> rate    curve sampled at THIS leg's size
    computeLegPolicyOutput(input.amount, ..., null, rate) <-- precision collapses HERE
  returns TokenInfo[] (token, amount)
```

Two things to keep straight about this path:

- **`sizeOrder`'s exposure outputs are unused here.** `cappedByPair` and `capFractionByPair` ration
  real fills; a probe commits no capital, so it passes a `null` budget and prices the whole input.
  Only `legNotionals` is consumed, as the rate sample point. See Decisions.md.
- **`computeLegPolicyOutput` is where an arbitrary-precision `Decimal` becomes the integer that
  leaves the process**, floored. Nothing downstream can recover the discarded fraction — the
  filler's `Decimal` rate is never transmitted. The floor is deliberate and load-bearing.

The integer then travels unchanged:

```
outputs[i].amount                   e.g. 715
  -> fillOrder calldata outputs[i]  uint256, covered by userOpHash
  -> bid submitted to the coprocessor
  -> aggregatePhantomBids           quotes.push({ price, weight })
  -> weightedMedian(backedQuotes)   SELECTION — returns an input element verbatim
  -> PhantomOrderPriceSnapshotV2    medianPrice = lowestPrice = highestPrice
  -> indexer updateLiquidityPools   renormalized by the leg's own standardAmount
```

A quote's weight in that median is the solver's balance of **that leg's output token on the
destination chain** — so a solver holding over half the leg's weight sets the published price
verbatim, and inventory in the wrong token buys no influence on that leg.

### Precision budget

The output integer *is* the price, to whatever resolution the output token's decimals allow, and
the probe size is what buys that resolution. At the original one-token probe, one whole cNGN
priced into 6-decimal USDC quoted ~715 base units, so the grid was `1/715` = 0.14% — and because
`LiquidityEngine.getBuyAndSellRates` reports the cNGN side as the *reciprocal* of that integer,
the floor read high: a 1391 pool mid published as 1392.76. Chains whose output token has 18
decimals never had the problem on that leg, which is why EVM-56 published `716845878136200` where
Base published a bare `715`.

**The pallet's standard amount is now 1000 units for every token (on-chain, 2026-08-22.)** The same
leg now quotes ~718,907 base units and the grid is `1/718907` = 0.00014%; a 1391 mid publishes as
1391.0005. The lever was the probe size, not the rounding mode — the floor stays, see Decisions.md.

Two consequences of the bump, neither a regression:

- A **curve-priced** pair is now sampled at 1000 token0 units instead of 1 (`resolveLegRates`
  takes the leg's own notional), so any sloped curve publishes a different — correctly, worse —
  rate than it did at the origin. Venue-priced pairs read a flat pool mid and do not move.
- `quotePhantomFill` warns when a probe notional exceeds the pair's `maxOrderSize`, and 1000
  token0 units clears caps that 1 unit never approached. The probe still publishes the full-size
  price; the warning is the signal that the pair will not fill what it quotes.

## Venue pricing (Uniswap V4 funded pairs)

Verified 2026-08-19.

```
resolveLegRates(...)
  curveless pair && token0 is a USD stable
    -> venuePriceMemo() -> getVenueUsdPrice(chain, token1)
         -> UniswapV4FundingPlanner.getExoticTokenPrice
              picks the position with the largest pool liquidity
              -> computeDirectPoolPriceUsd -> sdkPool.token0Price / token1Price
    -> checkPriceGuard(...)   reject if outside maxDeviationBps of the static reference
    -> rate = 1 / venueUsd
  otherwise -> the pair's ask/bid curve at the leg's notional
```

`computeDirectPoolPriceUsd` returns the **raw pool mid** derived from `sqrtPriceX96`. The pool's
fee tier is read and stored on the hydrated position (`pos.fee`) but never applied to the price,
and there is no size or impact term — `computeLegPolicyOutput` extends the mid linearly across the
whole priced quantity. `checkPriceGuard` is the only defense on this path, and it checks deviation
from a static reference, not execution cost. A venue-priced pair that has to swap through its own
pool to source inventory pays a fee tier it never quoted against.
