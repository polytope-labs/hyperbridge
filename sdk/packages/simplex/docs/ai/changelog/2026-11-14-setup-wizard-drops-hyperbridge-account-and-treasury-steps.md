# Setup wizard is two steps: Signer and Chains

The browser and desktop setup wizard now has two steps, Signer and Chains. The Hyperbridge account,
Treasury and Review & launch steps are gone.

The config it writes holds `[simplex]` with the signer and `maxConcurrentOrders`, `[orderbook]`, and
`[[chains]]`. It no longer writes `simplex.substratePrivateKey`, `simplex.hyperbridgeWsUrl` or
`[vault]`.

A solver set up this way bids on the HyperFX orderbook alone. Both settings are still available
afterwards:

- Vaults are connected from the dashboard's wallet tools, which use the same `VaultRowsEditor`.
- A Hyperbridge account is added by setting `substratePrivateKey` and `hyperbridgeWsUrl` in the config
  file, or by running `simplex init`, which still asks for one. The dashboard has no form for it, and
  the desktop app's config is encrypted, so a desktop install cannot add one yet.

`POST /api/setup/generate-substrate-key` and `POST /api/setup/check-substrate-balance` still exist.
The wizard no longer calls them. `GET /api/setup/defaults` still returns `hyperbridgeWs` and
`knownVaults`.

`RecoveryPhraseDialog` was only used by the Hyperbridge account step and is removed with it.

## Launching from the Chains step

The Chains step's primary button is "Save config & start Simplex". It is enabled once the chains pass
their checks and the assembled config passes `validateConfig`, the check the server runs again on
save. The launch screen and a failed start's message are shown in that step, and the failed state
turns the button into "Retry startup".

The wizard checks the start every 500 ms, where the review step checked every two seconds, because
the solver is usually running within a few seconds.

The review step's own content is not carried over: the wallet address is already on the Chains step,
and the config path and the rendered TOML preview are no longer shown. `POST /api/setup/preview` still
exists, and the wizard no longer calls it.

The dashboard's overview shows its "BRIDGE available" tile, and the top-up button on it, only when
`status.addresses.substrate` is set. A solver without a Hyperbridge account has no BRIDGE balance to
show, and the tile used to read "—" for it.
