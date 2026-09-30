# Limit order budget on chain

Every fill bid that a limit order priced ends with a `debitOrder` call on the solver's account.
The account keeps a tally of what each limit order has paid out and reverts a fill that would take
the tally past the order's total size.

A hold taken with `reserve()` does not reduce what other bids may promise, so several pending bids
can each promise what a limit order has left. `reserve()` and the store's draw-down are the
filler's own bookkeeping. The limit on total payout is enforced by the account.

## The budget

`budgetFor(order, token, decimals)` in `src/orderbook/amounts.ts` builds a `LimitOrderBudget`:

| Field | Value |
|-------|-------|
| `budgetId` | `budgetIdFor(order.id)`: `keccak256` of the UTF-8 bytes of `LimitOrder.id` as stored. It is the same for every repost of the order. |
| `cap` | `LimitOrder.size` converted from the store's 1e18 unit to the output token's own units with `toRaw`. It is the total size and not `remaining`, because the on-chain tally counts from zero over the order's life. |
| `token` | The output token the bid pays on the fill chain. |

`FXFiller` sets `budget` on each `BidPlan` it builds from a limit order. `IntentFiller` passes
`plan.budget` to `CacheService.setFillerOutputs(orderId, outputs, inputs, budget)` before each bid
goes out, and `prepareBidUserOp` reads it with `getBidBudget(orderId)`. The budget is stored in the
same cache entry as the filler outputs, so it is replaced and expires with them.

## The call

`buildApprovalAndFillCalldata` appends
`debitOrder(budgetId, cap, token, approved, fee)` (`SOLVER_ACCOUNT_ABI`) as the last call of the
bid's ERC-7821 batch, after the funding calls, the approvals and `fillOrder`. Its target is the
solver account itself.

- `approved` is exactly what the batch approves to the gateway for `token`.
- `fee` is `fillOptions.relayerFee` when the order is cross-chain and the fee token is `token`.
  Otherwise it is zero. The batch sends no native value for the dispatch, so the gateway takes the
  fee in the fee token. The approval for the fee token also covers the estimated gas cost, which
  the gateway does not pull and which is not counted as paid out.
- The account counts `approved`, less the allowance the gateway has left, less `fee`, and then
  clears the allowance.

`callGasLimitFor` adds `DEBIT_ORDER_GAS` (60,000) to the bid's call gas limit when the call is
appended.

## Draw-down

`chargedFor` in `src/core/filler.ts` works out what a fill charged in the output token's own units,
with the gateway's rounding, and scales the result to 1e18 with `toScaledUp`. For a token with 18
decimals or fewer, the store's `remaining` and the account's tally move by the same amount on every
fill. For a token with more than 18 decimals, a bid signs an output that 1e18 can express, and the
store draws down at least what the tally counted. Either way a bid sized from `remaining` fits
under the cap.

## Behaviour

| Case | Result |
|------|--------|
| No limit order priced the bid | The bid carries no budget and its calldata has no `debitOrder` call. |
| The budget's token is the zero address | Nothing is appended. Limit orders cannot pay out the native token. |
| The batch approves nothing in the budget's token | `buildApprovalAndFillCalldata` throws and the bid is not sent. The filler releases the bid's hold and continues with the order's other bids. |
| The fill would take the tally past `cap` | The call reverts with `LimitOrderExceeded(budgetId, total, cap)`. The whole batch reverts, the payout is undone, and the solver pays the gas of the reverted operation. |

The operation built by `prepareLimitOrderUserOp`, which is the signed price posted to the
orderbook, does not carry the call.

## Requirement

The solver's EOA must be delegated to a `SolverAccount` implementation that has `debitOrder`. On
an implementation without it, the batch of every bid that a limit order priced reverts.
