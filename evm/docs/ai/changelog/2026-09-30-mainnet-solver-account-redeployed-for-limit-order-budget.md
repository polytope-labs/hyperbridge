# 2026-09-30 — Mainnet solver account redeployed for the limit order budget

`SolverAccount` is `0x77c3394CA5881A74f18139AC87D0c11F8Faa90cC` on ethereum, arbitrum, optimism,
base, bsc, gnosis, polygon and polkadot hub, the same address on every chain. It replaces
`0xd5535d4DeB17F050e52B6efda2fDe00435f39279`. It was deployed through the CREATE2 factory
`0x4e59b44847b379578588920cA78FbF26c0B4956C` with the `VERSION` salt against the unchanged
`IntentGatewayV2` proxy `0xAe041F7B…9716`. The source is verified on every explorer.

| Chain | Deployment transaction | Block |
|---|---|---|
| ethereum | `0x5d88804d813ee7b6671dee28711286db86e867117671cd3d620038a498537c1a` | 26091113 |
| arbitrum | `0xbfa73c1581c59057edc3c36225585447545e0d6cbb4764b21576310dc228c2c0` | 510381809 |
| optimism | `0xed32934a15c9c61497b8d44b59991116b1756f05f44c23c78cf87f6a43ba7a0c` | 157591144 |
| base | `0x01b2c78d296f569b75d8d3e52e565b1f0a88ec0674b1713df474080b48d6f9b4` | 51995854 |
| bsc | `0x901ba1ba81b062a0099c9fdb6d5d85fb4d851fd3bb1faf035cb36c2b6a21b2b9` | 124929627 |
| gnosis | `0x1df1844825c9d63257ee0d91a01d6e58beaf6e6703609377f77b48bd3315e856` | 48518153 |
| polygon | `0xa67d15a59e378e74a911bec34a152450d075cb0d5a7a0a65e2ae966acd5be017` | 94717130 |
| polkadot hub | `0x77c952124c2b52a6e83d41d5f80846b4f304696132075d472cc25dad17d8641a` | 21270621 |

Soneium is not redeployed and keeps `0xd5535d4D…9279` in `config.mainnet.toml` and the contract
address docs. The SDK has no soneium `SolverAccount` entry.

The new account adds `debitOrder`, `spent` and the `LimitOrderExceeded` error, described in
`2026-09-29-solver-account-limit-order-budget.md`.

**Solvers must re-delegate.** An EOA still delegated to `0xd5535d4D…9279` has no `debitOrder`,
and any batch that calls it reverts. `chain.ts` carries the new address, which is what
`getSolverAccountAddress` hands to `BidManager` and `GasEstimator` as the EIP-7702
implementation. Simplex's `DelegationService` re-delegates at boot when the EOA's delegate
differs.

The mainnet indexer lists both addresses under `solverAccount` for each chain, so a solver
delegated to either one counts.

## Deploying with forge 1.8.3

`DeploySolverAccount` needs `--always-use-create-2-factory` so the address matches across chains.
On gnosis it also needs `--disable-block-gas-limit`, because loading `config.mainnet.toml` in
`setUp` exceeds the chain's block gas limit in simulation. On optimism and base, forge 1.8.3
ignores the factory and simulates CREATE2 from the sender EOA, which predicts
`0xe4bD6fbb…1b8c`. Those two chains, and ethereum (where the admin balance could not cover
forge's gas-limit multiplier), were deployed by calling the factory directly with
`salt ‖ initcode`, where the initcode is `SolverAccount` with the `IntentGatewayV2` address
appended as its constructor argument.
