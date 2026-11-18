# 2026-11-18 — Rebalancing is configured as a hub chain per asset

Issue #1370. Rebalancing now has the vault interface: a `threshold` that triggers a transfer and a
`minBalance` that stays behind. `triggerPercentage`, `baseBalances` and the surplus/deficit planner
are gone.

## Config

```toml
[rebalancing]
intervalMs = 300000   # optional, default 5 minutes, at least 10000

[[rebalancing.hubs]]
asset      = "USDC"
chain      = "EVM-8453"
threshold  = "6000"
minBalance = "5000"
```

One hub per asset. Every other configured chain is covered without being declared. Once a chain's
wallet holds `threshold` of the asset, `balance - minBalance` is bridged to the hub, rounded down to
whole cents. Amounts are human units, scaled with each chain's own token decimals.

`validateRebalancingConfig` (`src/config/filler-toml.ts`) checks the shape: one hub per asset, an
`EVM-<id>` chain, quoted amounts above zero with at most two decimals, `threshold` above
`minBalance`, and `intervalMs` no larger than a timer can take (2147483647). Given the configured
chain ids it also requires the hub to be one of them. A section that still has `triggerPercentage`
or `baseBalances` is refused with the new shape in the message. It is not translated: a base balance
per chain does not say which chain the funds should gather on. `simplex init` drops such a section
before validating an existing config (`dropRemovedRebalancing`), so an update run rewrites the file
without it.

## Behaviour

`RebalancingService` (rules in `src/services/rebalancers/Rules.md`):

- `coverage(hub)` says, for every other chain, which route reaches the hub or why the chain is left
  out: `no-asset` (not in the asset registry there), `no-route`, or `vault` (the chain has a vault
  for the asset, whose sweep manages that wallet balance). A vault on the hub does not count.
- Each route declares where it runs through `RebalanceRoute.tokenOn(asset, chain)`. The service has
  no chain table. A route is used only when its token equals the registry's token on both chains.
- `plan()` reads balances and decides the transfers. `execute()` sends them, one lane per chain as
  before. A CCTP result that is not `success` now counts as failed.
- A transfer is decided again when its turn in the lane comes, from the balance and the hub as they
  are then, so a wallet that fills drew down while it waited is not taken below `minBalance`. One
  that is no longer due is reported as `notSent`. So is one still waiting when the solver stops.
- The Binance route refuses an amount below Binance's minimum withdrawal before depositing it. The
  gap between `threshold` and `minBalance` can be small, and a deposit Binance will not withdraw
  stays there.
- Boot logs, per hub, the chains it gathers from and the ones it leaves out with the reason.
- The service is always built at boot and reads the config on every pass, so hubs saved at runtime
  apply from the next pass. `IntentFiller` arms each pass from the live `intervalMs`.
- `IntentFiller.rebalanceNow()` resolves with `{ started, skipped }` once the transfers are decided.
  They finish in the background and `rebalanceExecuted` reports the outcome. A pass requested while
  one is still sending is refused with `RebalanceInProgressError`.

The old route table listed Base as a USDT0 chain. Base has a LayerZero endpoint id and no USDT0, so
USDT on Base is now `no-route` unless Binance is configured.

## Interfaces

- SDK: `simplex.rebalancing.list()`, `preflight(hubs)`, `set(hubs, { intervalMs? })` and
  `rebalanceNow()`. `set(config)` and `trigger()` are removed.
- HTTP: `GET /api/rebalancing` returns the saved hubs, whether a pass is running, and for every
  asset a route can bridge, each chain that could be its hub with what that hub would cover.
  `PUT /api/rebalancing` takes `{ hubs, intervalMs? }`. `POST /api/rebalancing/run` starts a pass and
  answers `409` while one is running.
- Dashboard: a Rebalancing sheet on the Wallet page, beside the vault treasury. One row per asset
  with its hub chain, threshold and minimum balance, the chains the hub gathers from, and the reason
  for each chain it leaves out.
- A chain that is a hub cannot be removed until the hub is moved.

## Not changed

- Funds only move into the hub. A chain below `minBalance` is not topped up.
- Routes still send native transactions from the solver wallet, not sponsored UserOps. Every chain
  that bridges needs native gas, and the hub does too for USDC.
- A transfer that left its source and did not arrive is not retried.
