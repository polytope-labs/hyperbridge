# @hyperbridge/filler

## Unreleased

### Minor Changes

- Local web UI: browser setup wizard (`simplex` with no config) that writes the config and starts the filler in-process, plus an operator dashboard (status, pause/resume, graceful stop, balances, inflight price-curve edits persisted to the config file, overfill self-halt reset, live activity feed over SSE, manual vault sweep/redeem, runtime allowlist and log-level changes, rebalancing trigger view)
- Web wizard supports MPCVault/Turnkey signers and Uniswap V4 pool pricing
- Terminal setup wizard via `simplex init`
- `run` is the default command and `-c` is optional; configs are discovered at `./filler-config.toml` or `$SIMPLEX_HOME/config.toml`
- BREAKING: `--admin-port` is replaced by `--ui <[host:]port>` / `--no-ui`; the UI (same port 8686, same curve API) is now on by default on loopback, and mutating API requests require the `X-Simplex-UI: 1` header
- `pause()`/`resume()` on the filler, persisted across restarts
- Fixed: one-sided bid-only hyperfx configs crashed config validation

## 0.1.0

### Patch Changes

- Updated fee value
- Updated dependencies
    - @hyperbridge/sdk@1.3.22
