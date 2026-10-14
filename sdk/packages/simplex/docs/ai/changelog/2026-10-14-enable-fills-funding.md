# 2026-10-14 — Enable fills only on funded chains, and enable them live

Limit orders already name the chains a solver fills on, so per-chain observe-only is no longer
shown in the UI. Simplex watches every chain. "Enable fills" now means the wallet is funded on a
chain and delegated to the SolverAccount there.

## What funded means

`readChainFunding` (`src/services/read-chain-funding.ts`) reads a chain's `ChainFunding`
(`src/services/chain-funding.ts`):

- `feeToken`: the gateway fee token, read from the host's `feeToken()`, and the wallet's balance of
  it. That is USDC on Ethereum, Arbitrum, Base and Polygon, and USDT on BNB Chain.
- `nativeSetup`: native the paymaster's one-time Permit2 approval still needs, from
  `nativeSetupNeed`. Only set where the paymaster's token has no permit, which today is BNB Chain.
- `funded`: the wallet holds the fee token and needs no setup native.
- `delegated`: the wallet is delegated to the SolverAccount, which is what "fills enabled" shows.

`fundingShortfall` turns an unfunded read into "USDT and about 0.00014 BNB". The fee token address
is cached per chain.

## Setup wizard

- Every mainnet chain goes into the config, and none is written watch-only.
- On the Chains step, `POST /api/setup/chain-funding` reads each chain's funding through the
  wizard's RPCs every 15 seconds. Each card shows the fee token balance, or what is missing.
- A chain's "Enable fills" stays disabled until it is funded. Continue needs fills enabled on at
  least one chain, and each of those funded.
- The switch is not written to the config. At boot the filler delegates wherever the wallet can
  pay for it, as before.

## Dashboard

- `simplex.chains.enableFills(chainId)` checks funding, delegates on a running chain through the
  filler's existing delegation path, and clears a watch-only flag left by an older config. It
  throws with what is missing while the chain is unfunded.
- `POST /api/chains/enable-fills` calls it and answers with refreshed balances.
  `GET /api/balances?refresh=1` re-reads every chain first.
- Each `ChainBalanceRow` carries `funding`, computed on every balance refresh.
- On the Chains settings page, a running chain's "Enable fills" switch reflects `delegated` and
  enables fills at once, once funded. It cannot be switched back off; limit orders decide where the
  solver fills. A chain added on that page still starts after a restart.
- The "Observe only" checkbox is gone. Saving the chain set writes watch-only only for a chain
  Hyperbridge runs no bundler for.
