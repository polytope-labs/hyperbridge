# 2026-11-13 — Mainnet OwnableHandler and OwnableHostManager deployed

`evm/script/DeployOwnableHandlerAndHostManager.s.sol` deploys an `OwnableHandler` and an
`OwnableHostManager` for a host that already exists. Both are owned by the `OWNER` env var. The
manager is bound to the host at construction. Its admin is `GOVERNANCE_RELAYER`, the relayer the
current HostManager accepts governance from.

Both contracts are deployed on ethereum (1), arbitrum (42161), base (8453), bsc (56), polygon (137)
and polkadot hub (420420419). The addresses are the same on all six chains:

| Contract | Address |
|---|---|
| `OwnableHandler` | `0x3568374Fa9fcb72eE232dCFFF5569b3bF6C60726` |
| `OwnableHostManager` | `0x712515887e0DFF042530A120DCa9e8a3C90178A5` |

The owner of both is `0x08BcC96ceC579Ce2eDb34E5C5d790e4A40c3e224`. The manager's admin is
`0xD15007f5fF5c8c5cCc77339A03a3EA60d1Ece854`, and its host is
`0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3`.

The hosts still use HandlerV2 `0x2a18AB35DEa43474882E05A661e2F20fe89c0535` and HostManager
`0xc09A8229e2C18dEad6A2795154d7D8C7F1AC5Dfa`. Switching a host over takes a `SetHostParam`
governance request delivered through the current HostManager. `updateHostParams` replaces the whole
`HostParams`, so the request must carry every live field unchanged except `handler` and
`hostManager`.

The mainnet contract addresses page lists the new contracts on these six chains. Its
`ConsensusClient` rows now show ConsensusRouter `0x64094Bb4BCa533C8322a9E6d6ab88e9d2A36909a`, which
every mainnet host uses, in place of the stale `0x8FEe4eeEB9dD15247A2b8D0aC05fCaEcad710280`.
