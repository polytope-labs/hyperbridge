# Vault selection and balance defaults

`VaultRowsEditor` groups the server catalog per chain. `UiServer.knownVaultCatalog` (operator) keys
the registry's `erc4626Vaults` by state machine id for every `INIT_CHAINS` entry on the running
network (testnet if any running chain is testnet, else mainnet), plus every running chain; the setup
`defaults` endpoint already covered all `INIT_CHAINS`, so the wizard passes `network={state.network}`
to filter. `chainGroups` puts the enabled chains (the `chains` prop) first, then every other chain
with vaults in catalog order. Each enabled chain renders as a `VaultChainGroup`: a header with the
chain logo, name, vault count and an "Enabled" pill, then compact rows (token icon only, no chain
logo) and any custom vaults on that chain. The other chains fold into one `OtherNetworks`
collapsible (stacked logos, chain names, vault count, "Not enabled" pill, chevron; closed by
default) whose content is the same per-chain groups, each header carrying a "Not enabled" pill and,
when `onEnableChain` is given, an "Enable chain" link — the dashboard opens the Chains & endpoints
sheet, the wizard jumps to the Chains step via the new `goToStep` in `StepProps`. Rows in a locked
group have a disabled checkbox and `data-disabled`; a row already saved for a chain that is no
longer enabled stays unlocked so it can be deselected. Select all and its "all selected" state only
consider enabled groups. The dashboard drawer shows one summary line ("2 vaults connected · Base,
Arbitrum", from the saved config) with Sweep now / Redeem all beside it, the editor, then Save; the
duplicate inner heading is gone and the restart caveat appears only when the filler booted without a
vault venue. Send funds and Vault treasury live on the Wallet page (`ui/src/operator/WalletTools.tsx`,
rendered above the ledger by `Wallet.tsx`); Operations keeps the allowlist and chain editors. The
vault editor's "Enable chain" link closes its sheet and calls `onOpenChains`, which `Operator` turns
into `setTab("operations")` plus an `initialPanel="chains"` prop that `Operations` opens once and
acknowledges through `onInitialPanelShown`. Selecting a curated vault directly or
through Select all creates a `VaultRowDraft` with product-specific balance defaults: Aave stataUSDC uses
`threshold=20` and `minBalance=10`, while Yield Bearing cNGN uses `1000` and `1`. Custom and unknown
vaults use the generic `5000`/`3000` fallback. Existing rows are never rewritten, so saved operator
settings survive reopening the editor.

Both numeric labels use the shared `@hyperbridge/ui` tooltip components. Their icon buttons open on
hover or keyboard focus and explain that `threshold` triggers a sweep while `minBalance` is the
amount Simplex never sweeps below; for USDC and USDT the help adds that the token also pays
paymaster gas.

In the operator drawer, Save sends the current draft to `PUT /api/vault`, then refreshes the config.
If another edit is followed by Save while that request is pending, `WalletTools` records one queued
retry and reads the newest rows when the prior request finishes; repeated clicks coalesce to that
latest draft rather than running concurrent vault hydrations. A response only reaches the success
state when `persisted` is true. Persistence failures are rendered inside the still-open vault drawer;
after the queued-save loop drains, a persisted final draft emits one success toast. The UI does not
convert the response's `restartNeeded` advisory into a restart instruction.
