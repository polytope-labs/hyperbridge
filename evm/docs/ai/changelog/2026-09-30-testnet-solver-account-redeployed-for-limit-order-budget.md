# 2026-09-30 — Testnet solver account redeployed for the limit order budget

`SolverAccount` on bsc-testnet (97) and polygon-amoy (80002) is
`0xf98E484858F59C30e31D2695ac18F92bc7C7b799`, the same address on both chains. It replaces
`0x153DB990FE3b761B54ad71D0f1A0987dF11740DB`. The source is verified on both explorers.

| Chain | Deployment transaction | Block |
|---|---|---|
| bsc-testnet | `0xdb79c4169061656436669f129f674d2bf3c89be5b5f8c282076ac170c6aa405c` | 134053398 |
| polygon-amoy | `0xd709f7a56db58e05eea59ed74b0b52bf653d04534fde2680e5bdbb77cec172ec` | 48948596 |

The new account keeps a tally of what each limit order has paid out and rejects a fill that would
take the tally past the order's size. It adds `debitOrder`, `spent` and the `LimitOrderExceeded`
error, described in `2026-09-29-solver-account-limit-order-budget.md`. Everything else is
unchanged, so a solver delegated to it behaves as before until its fill bids call `debitOrder`.

**Solvers must re-delegate.** An EOA still delegated to `0x153DB990…40DB` has no `debitOrder`, and
any batch that calls it reverts. `chain.ts` carries the new address for both chains, which is what
`getSolverAccountAddress` hands to `BidManager` and `GasEstimator` as the EIP-7702 implementation.
Simplex's `DelegationService` compares the EOA's delegate with that address at boot and
re-delegates when they differ.

The testnet indexer lists both addresses under `solverAccount` for EVM-97 and EVM-80002, so a
solver delegated to either one counts.

The `IntentGatewayV2` proxy is unchanged, `0x6CF42FA9…7cFA` on both chains. Mainnet is unchanged.
