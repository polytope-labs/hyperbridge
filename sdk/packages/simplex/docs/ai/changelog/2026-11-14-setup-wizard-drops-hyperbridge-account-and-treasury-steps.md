# Setup wizard drops the Hyperbridge account and Treasury steps

The browser and desktop setup wizard now has three steps: Signer, Chains, and Review & launch.

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
