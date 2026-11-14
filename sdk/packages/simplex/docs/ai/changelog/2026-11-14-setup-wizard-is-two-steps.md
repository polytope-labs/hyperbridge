# Setup wizard is two steps: Signer and Chains

The browser and desktop setup wizard has two steps, Signer and Chains. This removes its Hyperbridge
account, Treasury and Review & launch steps.

## What the wizard writes

The config holds `[simplex]` with the signer and `maxConcurrentOrders`, `[orderbook]`, and
`[[chains]]`. It does not write `simplex.substratePrivateKey`, `simplex.hyperbridgeWsUrl` or
`[vault]`, so a solver set up through the wizard bids on the HyperFX orderbook alone.

Both can be added afterwards:

- Vaults are connected from the dashboard's wallet tools, which use the same `VaultRowsEditor`.
- A Hyperbridge account is added by setting `substratePrivateKey` and `hyperbridgeWsUrl` in the config
  file, or by running `simplex init`, which asks for one. The dashboard has no form for it, and the
  desktop app's config is encrypted, so a desktop install cannot add one.

## Launching from the Chains step

The Chains step's primary button is "Save config & start Simplex". It is enabled once the chains pass
their checks and the assembled config passes `validateConfig`, the check the server runs again on
save. The launch screen and a failed start's message are shown in that step, and a failed start
turns the button into "Retry startup". `useLaunch` in `ui/src/wizard/launch.tsx` checks the start
every 500 ms.

The wallet address to fund is on the Chains step. The config path and the rendered TOML are not shown.

## Setup routes the wizard does not call

`POST /api/setup/generate-substrate-key`, `POST /api/setup/check-substrate-balance` and
`POST /api/setup/preview` still serve scripted setups. `GET /api/setup/defaults` still returns
`hyperbridgeWs` and `knownVaults`.

## Dashboard

The overview shows its "BRIDGE available" tile, and the top-up button on it, only when
`status.addresses.substrate` is set. A solver without a Hyperbridge account has no BRIDGE balance to
show.
