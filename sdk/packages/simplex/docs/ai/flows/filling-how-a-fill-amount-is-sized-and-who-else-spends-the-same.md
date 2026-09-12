# Filling: how a fill amount is sized, and who else spends the same balance

Verified against the reverted Base fill `0x31de53fe...` (UserOp `0xe090acd1...`), traced end to end.

## Sizing, per leg

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

## The batch, in execution order

`callData` is an ERC-7821 batch: the funding calls first (for V4, `PositionManager.multicall(modifyLiquidities)` encoding DECREASE_LIQUIDITY + TAKE_PAIR), then `approve` for the fill amount, then `IntentGateway.fillOrder`, which does the `transferFrom` that moves the output token to the user.

What matters is what happens _before_ any of that. The EntryPoint runs paymaster validation first, and `SimplexPaymaster` prefunds by pulling the EntryPoint's worst-case gas cost out of the solver's wallet with `transferFrom`, in whichever stablecoin `selectToken` picked — refunding the unused part in `_postOp`, long after the batch has already run or reverted. So the balance the batch sees is always lower than the balance the sizing saw, by the prefund.

That is what `paymasterReserveForToken` exists to absorb. Without it, a balance-limited fill — `walletContribution == usableWallet == balance` — is sized to the last unit and reverts by exactly the prefund: 28,993 units of USDC against a 14,808,699,383 fill, where 14,377,624,370 (wallet) + 431,075,013 (V4 credit) reproduces the bid amount exactly.

## Which token the paymaster charges

Decided at submit time, not at sizing time, by `buildPaymasterAndData` (`src/services/paymaster/index.ts`): the Simplex paymaster if configured and its EntryPoint deposit covers the op's max prefund with headroom, preferring the first of `[USDC, USDT]` with a balance >= 1 token and Permit2 allowance >= $5 (`selectToken`). If neither is ready, the first balance-qualified token takes the bootstrap path; if sponsorship cannot be prepared, no paymaster is used. Because the sizing decision feeds the balances that decide this, the reserve covers both eligible tokens instead of predicting one.
