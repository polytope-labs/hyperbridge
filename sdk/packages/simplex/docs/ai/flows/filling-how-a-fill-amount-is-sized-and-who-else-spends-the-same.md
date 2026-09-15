# Filling: how a fill amount is sized, and who else spends the same balance

Verified against the reverted Base fill `0x31de53fe...` (UserOp `0xe090acd1...`), traced end to end.
The partially-filled-order handling was verified on 2026-09-15 by the unit tests in `src/tests/pairs.test.ts`.

## What is still owed

Before any leg is sized, `FXFiller.calculateProfitability` reads `ContractInteractionService.partialFillsFor`
on the destination chain. That is `_partialFills[commitment][token]` for every output, and it holds what any
solver has already delivered. Fills land on the destination for both paths, so both read it there. A failed
read skips the order, and so does an order whose every output is already met.

Each leg then has two remainders:

- `unfilled` = the output still owed.
- `remainingInputs[i]` = the escrow it can still release: the input less `cumulativeReleased(input, alreadyFilled, total)`.

`sizeOrder` computes the pair notionals and `maxOrderSize` caps from `remainingInputs`, not the original inputs.

## Sizing, per leg

`FXFiller.calculateProfitability` walks `order.inputs` and sizes each leg independently (`src/strategies/fx.ts`).
A leg with nothing left to fill goes out as a zero output. The gateway skips it without counting it as an
under-fill. Every other leg goes through these steps:

1. `targetOutput` = what the curve will pay (`policyMaxOutput`), in every case. This is the amount the leg intends to hand over, and it may exceed what the user asked for — `IntrinsicIntents.fillOrder` takes `solverAmount > totalRequired` and splits the excess between the beneficiary and the protocol (`surplusShareBps`), and it is the same figure `quotePhantomFill` publishes as the pair's quoted rate. A pair's `maxOrderSize` is optional and does not shorten this: it binds earlier and in the other unit, where `computeLegPolicyOutput` rations `token0ForLeg` against the pair's remaining budget before the rate is applied. `desiredOutput` — the unfilled ask scaled by the same cap — is no longer a ceiling on payout; it survives as the price gate's comparand and in the short-fill logs. One exception: on a leg that has already started, `targetOutput` is capped at `unfilled`. Once any output has landed, the gateway takes no more than the remainder and splits no surplus, so sourcing more would only strand tokens in the wallet.
2. `reserve` = the paymaster reserve for this token (`paymasterReserveForToken`, from `src/services/paymaster`) plus every funding venue's `walletReserveForToken`. Only the vault returns a non-zero venue reserve, its configured `minBalance`; `UniswapV4FundingPlanner` returns `0n`. The paymaster half is seeded outside the venue loop deliberately — the loop is empty when no vault and no V4 positions are configured, and that filler still sizes partial fills.
3. `usableWallet` = balance − reserve. `walletContribution` = `min(targetOutput, usableWallet)`.
4. Any shortfall is requested from each funding venue in turn via `planWithdrawalForToken`, which returns ERC-7821 calls and the amount it expects them to credit. The calls accumulate in `fundingCalls`. For V4 that credit is priced from `liquidityRemoval`, the liquidity the encoded DECREASE_LIQUIDITY actually carries — not the liquidity the planner asked for, which the SDK truncates.
5. `finalOutputAmount` = `min(walletContribution + credited, targetOutput)`.

After the loop, `estimateGasFillPost` prices the fill. A cross-chain order then has to clear one more affordability check: `fillOrder` dispatches the escrow-release message and `HyperApp.dispatchWithFeeToken` pulls `dispatchFee` from the same wallet in the destination host's fee token (USDC on Base — often the token just paid out). The fee is only known here, after the funding calls it depends on exist. Every cross-chain fill dispatches that message, partial or not. Shrinking the fill to make room is not attempted, so the order is skipped when the residue will not cover the fee plus the paymaster reserve.

A capped leg is separately gated as a partial fill when the cap actually shortens it — `capFraction.lt(1) && policyMaxOutput < unfilled`. Both halves matter: with the payout unclamped, a curve running far enough above the order's rate covers the whole ask out of a capped slice, which is a full fill.

Whenever `finalOutputAmount < unfilled` the fill is an under-fill and has to clear `partialEligible`, or the order is skipped. The only thing that fails it is output calldata: the gateway reverts an under-fill of such an order with `PartialFillNotAllowed`, on either path.

The valuation pass prices each leg on the escrow this fill releases: `cumulativeReleased(input, alreadyFilled + provided, total) − cumulativeReleased(input, alreadyFilled, total)`. Cross-chain releases exactly that. Same-chain releases `input × fill / total` per slice and sweeps the rest on completion, which agrees to within integer dust.

The outputs and the funding calls are cached against the order id (`setFillerOutputs`, `setFundingPrepends`). `ContractInteractionService.prepareBidUserOp` reads them back verbatim and signs them into the bid; nothing re-reads the balance between sizing and execution, and the bid is committed, so an amount that was affordable at evaluation must still be affordable when the UserOp lands.

## The batch, in execution order

`callData` is an ERC-7821 batch: the funding calls first (for V4, `PositionManager.multicall(modifyLiquidities)` encoding DECREASE_LIQUIDITY + TAKE_PAIR), then `approve` for the fill amount, then `IntentGateway.fillOrder`, which does the `transferFrom` that moves the output token to the user.

What matters is what happens _before_ any of that. The EntryPoint runs paymaster validation first, and `SimplexPaymaster` prefunds by pulling the EntryPoint's worst-case gas cost out of the solver's wallet with `transferFrom`, in whichever stablecoin `selectToken` picked — refunding the unused part in `_postOp`, long after the batch has already run or reverted. So the balance the batch sees is always lower than the balance the sizing saw, by the prefund.

That is what `paymasterReserveForToken` exists to absorb. Without it, a balance-limited fill — `walletContribution == usableWallet == balance` — is sized to the last unit and reverts by exactly the prefund: 28,993 units of USDC against a 14,808,699,383 fill, where 14,377,624,370 (wallet) + 431,075,013 (V4 credit) reproduces the bid amount exactly.

## Which token the paymaster charges

Decided at submit time, not at sizing time, by `buildPaymasterAndData` (`src/services/paymaster/index.ts`): the Simplex paymaster if configured and its EntryPoint deposit covers the op's max prefund with headroom, preferring the first of `[USDC, USDT]` with a balance >= 1 token and Permit2 allowance >= $5 (`selectToken`). If neither is ready, the first balance-qualified token takes the bootstrap path; if sponsorship cannot be prepared, no paymaster is used. Because the sizing decision feeds the balances that decide this, the reserve covers both eligible tokens instead of predicting one.
