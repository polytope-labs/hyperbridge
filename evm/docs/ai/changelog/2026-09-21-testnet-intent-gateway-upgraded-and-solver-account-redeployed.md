# 2026-09-21 — Testnet intent gateway upgraded; solver account redeployed

The `IntentGatewayV2` proxy on bsc-testnet (97) and polygon-amoy (80002), `0x6CF42FA9…7cFA` on both,
now runs the current implementation. It had been left on a build that predates `version()`, the
relayer gate and the `Execute` governance action. The proxy address is unchanged.

New addresses, identical on both chains:

| Contract | Address |
|---|---|
| `IntentGatewayV2` implementation | `0x5E03fe27c7372E044296B4BA38B2cF9f760984CA` |
| `IntrinsicModule` | `0x19587e181634Fd118258e2Af37e5c6531e6cFf43` |
| `ExtrinsicModule` | `0x0205338774146975D86E3164FfB3882b26b6d742` |
| `SolverAccount` | `0x1bBaa8bf14790A823EF3A9D3c44cD04dA636936f` |

**Solvers must re-delegate.** `SolverAccount` moved from `0x110C7E18…6a17`, so an EOA still
delegated to the old address runs the previous validation logic. `chain.ts` carries the new address
for both chains, which is what `getSolverAccountAddress` hands to `BidManager` and `GasEstimator`
as the EIP-7702 implementation.

The gateway reports `version() == 3` and an owner of `0xc8809DD0…2757`, whose only powers are
`pause()` and `unpause()`. Its params now match the deploy script's defaults: `surplusShareBps`
6000 and `protocolFeeBps` 5, up from 5000 and 30.

`relayer()` is `address(0)` on both chains, which `onlyRelayer` reads as accepting any relayer.
Arming the gate needs a `setRelayer` through `execute_on_gateway`.
