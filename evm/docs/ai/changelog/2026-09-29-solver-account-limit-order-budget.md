# 2026-09-29 — Solver account limit order budget

`SolverAccount` keeps a tally of what each limit order has paid out and rejects a fill that would
take the tally past the order's cap. A solver fills limit orders from one account, and several
pending fill bids can each promise what a limit order has left, so without a limit on chain one
limit order could pay out more than its size.

```solidity
function debitOrder(bytes32 orderId, uint256 cap, address token, uint256 approved, uint256 fee) external;
function spent(bytes32 orderId) external view returns (uint256);
error LimitOrderExceeded(bytes32 orderId, uint256 total, uint256 cap);
```

`debitOrder` is the last call of a fill bid's ERC-7821 batch, after the approvals and `fillOrder`.
Only the account itself may call it; any other caller gets `AccountUnauthorized(address)`. It:

1. reads the payout off the gateway's allowance: `used = approved - allowance(account, gateway) - fee`;
2. reverts with `LimitOrderExceeded` when `total = spent[orderId] + used` is greater than `cap`. The
   revert takes the whole batch with it, so the payout is undone. A fill that lands exactly on the
   cap passes;
3. stores `total` and sets the token's allowance to the gateway to zero with `forceApprove`, which
   works for tokens whose `approve` returns no value.

The tally sits in an ERC-7201 namespaced slot, namespace `hyperbridge.storage.SolverAccount.Budgets`.
Under EIP-7702 that is the EOA's own storage, so the tally survives re-delegation to a later
implementation and does not clash with other delegates.

## What the bid builder passes

| Argument | Value |
|---|---|
| `orderId` | The limit order the fill counts against. The tally has no unit, so an `orderId` must always be debited in the same token. |
| `cap` | The most the order may pay out in total. Passed on every call and not stored; the solver's signature over the operation's calldata protects it. |
| `token` | The token the fill paid out. |
| `approved` | The exact total the batch approved to the gateway for `token`. |
| `fee` | The dispatch fee the gateway pulls from that allowance: the fill's `relayerFee` when the fill is cross-chain, the host's fee token is `token`, and the dispatch is not paid in the native token. Zero otherwise. |

The tally counts everything paid out of the allowance for the order, including the share of any
surplus that goes to the protocol, so `cap` limits the total paid out. Anything approved but not
pulled is not counted.

`approved` and `fee` must be exact:

- An `approved` that is too high, or a `fee` that is too low, counts more than the order paid, so
  the order reaches its cap early.
- An `approved` that is too low, or a `fee` that is too high, counts less than the order paid. When
  the subtraction underflows, the checked arithmetic panics and the batch reverts.

## Not covered

- Nothing is checked during `validateUserOp`. A bid that fails the cap reverts at execution and
  still costs gas.
- A budget cannot be shrunk, reset or cancelled on chain.
- A batch is not required to contain the call.
- Native token payouts are not counted.

## Deployment

The budget needs a new `SolverAccount` implementation and solver EOAs re-delegated to it.
`script/DeploySolverAccount.s.sol` deploys it with CREATE2 and the shared salt, so the new bytecode
has one new address, the same on every chain with the same `INTENT_GATEWAY_V2` address. An EOA
delegated to an implementation without `debitOrder` reverts any batch that calls it.
