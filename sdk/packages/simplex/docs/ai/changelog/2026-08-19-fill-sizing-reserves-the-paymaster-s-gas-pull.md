# 2026-08-19 — Fill sizing reserves the paymaster's gas pull

A partial fill on Base reverted with `ERC20: transfer amount exceeds balance` after being short by 28,993 units — $0.029 on a $14,808.699383 fill. The fill was sized as wallet balance plus the Uniswap V4 credit and bid to the last unit, but the paymaster charges gas in the same USDC and pulls it via `transferFrom` during `validatePaymasterUserOp`, before the batch runs. The bid amount was bit-exact `balance + credited`, and the revert delta was bit-exact the prefund.

The only reserve the sizing loop knew about came from `FundingVenue.walletReserveForToken`, which is the vault's `minBalance`. `UniswapV4FundingPlanner` returns `0n` there by design (LP positions have no wallet float), and a filler configured with no venue at all never enters that loop, so both setups sized fills with zero headroom. A new `paymasterReserveForToken` in the paymaster module now contributes a reserve for the chain's USDC and USDT whenever a paymaster is configured, scaled by each token's own decimals, and the leg loop seeds `reserve` with it.

Files: `src/services/paymaster/index.ts`, `src/strategies/fx.ts`, `src/tests/services/paymaster-reserve.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.
