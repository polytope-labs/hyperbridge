## Rebalancing Rulebook

Rebalancing gathers each asset on one hub chain. It is configured like a vault sweep: a
`threshold` that triggers a transfer and a `minBalance` that stays behind.

### 1. Configuration

```toml
[rebalancing]
intervalMs = 300000   # optional, default 5 minutes

[[rebalancing.hubs]]
asset      = "USDC"
chain      = "EVM-8453"
threshold  = "6000"
minBalance = "5000"
```

- One hub per asset. The asset is a symbol, resolved through the asset registry.
- The hub must be a chain the solver runs, written exactly as `EVM-<chainId>`.
- `threshold` must be greater than `minBalance`. Both are quoted amounts in human units,
  above zero, with at most two decimals. Transfers are whole cents, so a finer amount could
  differ on paper and scale to the same balance.
- `intervalMs` is an integer from 10000 to 2147483647. A timer armed with a longer delay
  fires after one millisecond.

`validateRebalancingConfig` checks the shape wherever a section is set: `validateConfig` at
boot, `RebalanceController.set`, and `PUT /api/rebalancing`. It rejects the settings from
before hubs (`triggerPercentage`, `baseBalances`) with the new shape in the message.
`simplex init` drops such a section before it validates an existing config.
`RebalancingService.preflight` adds the checks that need the running solver: the hub is a
configured chain, a route carries the asset, and the registry names the asset on the hub.

### 2. Which chains a hub covers

Every other configured chain is judged on its own. Nothing is declared per chain.

A chain is covered when both hold:

- The asset registry names the asset on that chain.
- One route moves that same token on the chain and on the hub.

A chain is left out, with a reason the dashboard shows:

- `no-asset`: the registry has no such asset there.
- `no-route`: no route reaches the hub from there.
- `vault`: a vault on that chain holds the asset. Its sweep already manages that wallet
  balance, and the two would compete for it. A vault on the hub does not count.

### 3. Routes

Each route says where it runs, through `RebalanceRoute.tokenOn(asset, chain)`. The service
keeps no chain list of its own.

- **CCTP** carries USDC between the chains in `CCTP_CHAINS` (`cctp.ts`), which is also the list
  Bridge Kit's adapter is built from. The token is Circle's USDC for the chain.
- **USDT0** carries USDT between chains whose config names both an OFT contract and a
  LayerZero endpoint id. An endpoint id alone is not enough.
- **Binance** carries USDC and USDT between the networks in `CHAIN_ID_TO_BINANCE_NETWORK`,
  and only when `[binance]` is configured. It refuses an amount below Binance's minimum
  withdrawal before it deposits anything.

Routes are tried in that order, so Binance is used only where no native route runs.

The route's token must equal the registry's token on both chains. Otherwise the balance that
was read would not be the token that is sent. An `[assets]` override that points a symbol at
another token therefore takes that chain out of the route.

### 4. One pass

A pass runs every `intervalMs`, and on "Rebalance now". For each hub and each covered chain:

1. Read the wallet balance and the token's decimals on that chain.
2. Below `threshold`, leave the chain alone.
3. At or above it, bridge `balance - minBalance` to the hub, rounded down to whole cents.

`threshold` and `minBalance` are scaled with that chain's own decimals. One asset has 6
decimals on most chains and 18 on BNB Chain.

The gap between `threshold` and `minBalance` is the smallest transfer, so no chain bridges dust.

There is no planning across chains. A chain whose balance cannot be read is skipped for that
pass, and the others still move.

### 5. Sending the transfers

`plan()` decides the transfers and `execute()` sends them. `IntentFiller.rebalanceNow()` runs
both. It resolves once the transfers are decided, and they finish in the background. When they
end it emits `rebalanceExecuted` with how many were started and how many arrived.

One pass runs at a time. A second pass is refused with `RebalanceInProgressError` while the
first is still sending.

Every transfer is sent from the one solver wallet, with no nonce manager. So two transfers that
send transactions on the same chain must not run at once. Each chain is a lane. A transfer
waits for the previous transfer on every chain it sends on. Transfers that share no chain run
in parallel.

- Every route sends from the source chain.
- CCTP also sends on the hub: Bridge Kit's mint (`receiveMessage`) goes out from the same
  wallet. So CCTP transfers into one hub run one after another.
- USDT0 is delivered by LayerZero and Binance withdraws by itself. Neither sends anything
  from the solver wallet on the hub.
- A lane is held for the whole transfer: for CCTP the burn, the attestation wait and the
  mint, for Binance the deposit's credit and the withdrawal, for USDT0 the wait for delivery.
- Lanes serialise rebalancing transfers only. Fills, vault sweeps, top-ups and operator sends
  share the wallet's nonce and do not take them.

A transfer can wait in its lane for many minutes, and fills keep spending the wallet
meanwhile. So when its turn comes it is decided again, from the balance and the hub as they are
then. It sends `balance - minBalance` as it stands. It is not sent at all when the wallet is back
under `threshold`, or when the hub was moved or removed (`hub-changed`). Such a transfer is
reported as `notSent`, not as failed.

Once the solver is stopping, transfers still waiting their turn are not sent either
(`cancelled`). One already under way runs to its end, and `stop()` waits for it.

Routes send native transactions. Every chain that bridges needs native gas, and so does the
hub for USDC.

### 6. Failures

- A transfer that fails is logged and reported in the pass's outcome. It does not stop the
  transfers behind it.
- A CCTP transfer that Bridge Kit ends in any state but `success` counts as failed. Its
  error names the step that failed and the burn transaction, when there was one.
- Nothing is retried within a pass. The next pass reads the balance again. If the funds never
  left, the chain is still at its threshold and the transfer is sent again.
- A transfer that left its source and did not arrive is not sent again: the source balance is
  already down to `minBalance`. It has to be completed by hand.
