# Setup wizard is two steps: Wallet and Chains

The browser and desktop setup wizard has two steps, Wallet and Chains. This removes its Hyperbridge
account, Treasury and Review & launch steps.

## The Wallet step

The step says in three points what the wallet is for: it holds the funds, it signs each fill, and
what is entered stays on the machine. It then asks how Simplex should sign: private key, secret
phrase, MPCVault or Turnkey.

A private key or secret phrase reports its result under the field. A refused one shows the reason,
and an accepted one shows the derived wallet address. The footer shows no "Required to continue" list
while the step does this. MPCVault and Turnkey still list their empty fields there.

The step says where the key or phrase is kept. `GET /api/setup/defaults` returns
`configEncrypted: true` when the config file is saved encrypted, as the desktop app saves it, and
the step then says so. Otherwise it says the file keeps the key in plain text.

## The Chains step

The step opens with three points on funding. Send at least 5 USDC to the wallet on each chain that
will fill, or USDT on BNB Chain. 5 is kept for gas. A chain's switch unlocks once its funds arrive.
The wallet address is under them.

Each chain's row says where it stands. An unfunded chain reads "Send at least 5 USDC". A funded chain
that is off reads its balance and "ready". A chain that fills but has run short reads "Needs at least
5 USDC" as a warning. A chain that also needs native for a one-time setup names the amount there.

Both steps draw their opening points with `StepPoints` in `ui/src/wizard/StepPoints.tsx`.

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
their checks and the assembled config passes `validateConfig`. The server decides on save: it runs
`validateConfig` again along with its own checks, and a config it refuses is not written.

The launch screen and a failure's message are shown in that step, and a failure turns the button
into "Retry startup". The "Your configuration is saved" hint appears only when the save went through
and the start then failed. `useLaunch` in `ui/src/wizard/launch.tsx` checks the start every 500 ms.

The wallet address to fund is on the Chains step. The config path and the rendered TOML are not shown.

## Setup routes the wizard does not call

`POST /api/setup/generate-substrate-key`, `POST /api/setup/check-substrate-balance` and
`POST /api/setup/preview` still serve scripted setups. `GET /api/setup/defaults` still returns
`hyperbridgeWs` and `knownVaults`.

## Dashboard

The overview shows its "BRIDGE available" tile, and the top-up button on it, only when
`status.addresses.substrate` is set. A solver without a Hyperbridge account has no BRIDGE balance to
show.
