# ChangeLog

AI-maintained log of code changes in `sdk/packages/simplex`. Every AI-assisted change appends an entry here: date, what changed, and the files touched. This is not the release changelog — `sdk/packages/simplex/CHANGELOG.md` is the published release log and is managed separately.

Entry format:

```
## YYYY-MM-DD — short title (issue/PR if any)
What changed and why, in a few sentences.
Files: list of files touched.
```

Newest entries first.

## 2026-09-04 — Say why a vault sweep did nothing, and restore the restart notice for vault saves

Diagnosed on a running solver: the periodic sweep ran every five minutes against a wallet holding
165k cNGN over a threshold of 10, and never deposited. The ycNGN vault is a `StreamingYieldVault`,
whose `maxDeposit` returns 0 while a tranche vests (22h of every 24h cycle); the sweep clamped the
deposit to 0 and `continue`d with no log line, and the dashboard's Sweep now button reported
"Sweep executed" on the same silent path.

`VaultFundingPlanner.sweepExcessToVault` now returns a `VaultSweepResult`: the batches it submitted
(with per-vault deposit amounts) and every vault it skipped with a reason — `sweeping-disabled`,
`below-threshold`, or `deposits-closed` with the wallet balance, threshold and `maxDeposit` it saw.
A `deposits-closed` skip logs a warning the first time per closure and debug on every repeat until
a deposit goes through or the venue is reconfigured. `VaultLiquidityState.refresh` reads
`maxDeposit(solver)` alongside `maxWithdraw`, so the balance snapshot carries `acceptsDeposits` per
vault and the overview shows "Deposits closed" under the asset's In vault figure.

`POST /api/vault/sweep` returns the pass as `VaultSweepDto` with amounts formatted in token units.
The vault panel turns that into one sentence — what was deposited, or which vault refused and how
far over its threshold the wallet is, or that balances are simply below their triggers.

Vault saves: the 2026-09-03 change that dropped the `restartNeeded` handling is reverted in
substance. The server sends `restartNeeded: true` only when the filler booted without a vault venue;
the rows are persisted but nothing in the process uses them until a restart, so the panel now shows a
warning notice and toast saying so, and the hint copy once again says edits re-hydrate the running
venue "after a restart" when no venue exists.

Also fixed the branch's declaration build: `InitChainMeta` gained the optional `note` the setup API
and terminal wizard were already reading.

Files: `src/funding/types.ts`, `src/funding/vault/{VaultFundingPlanner,VaultLiquidityState}.ts`,
`src/services/BalanceProvider.ts`, `src/services/server/{UiServer,dto}.ts`, `src/simplex.ts`,
`src/index.ts`, `src/cli/init/chains.ts`, `ui/src/types.ts`,
`ui/src/operator/{Operations,OperatorOverview}.tsx`, `ui/src/styles/{controls,operator}.css`.
Tests: `src/tests/funding/vault.test.ts`, `src/tests/ui-server.test.ts`. Docs:
`docs/content/developers/sdk/{simplex,api/simplex}.mdx`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-04 — Link successful sends to their block explorer

The Send funds success row now renders the transaction hash and an external-link icon as one link
that opens the selected network's block explorer in a new tab. The explorer URL is captured with the
completed send so changing the form's network afterward cannot redirect the prior hash to the wrong
chain.

Files: `ui/src/operator/Operations.tsx`, `ui/src/styles/operator.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Correct vault save feedback

Vault saves no longer turn a successful persisted response into an error instructing the operator to
restart the filler. A persisted save now emits a clear success toast, while the vault editor limits
its explanatory copy to the configuration persistence it can accurately promise. Runtime, server,
and vault lifecycle behavior are unchanged.

Files: `ui/src/operator/Operations.tsx` and `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Preserve vault edits made during a save

Vault saving now queues one latest-draft retry when the operator clicks Save again while an earlier
request is still in flight, so the shared action guard cannot silently discard newer values. The UI
also treats `persisted: false` as a save failure and renders the result inside the open vault drawer.

Files: `ui/src/operator/{Operations.tsx,Operations.test.tsx}` and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Show selected-asset liquidity in Send funds

The Send funds form now displays the selected chain and asset's available balance beside the Amount
label. ERC-20 values reuse the canonical wallet-reserve and vault-aware balance snapshot, native gas
uses the chain's native balance, unavailable reads remain explicit, and a successful transfer triggers
an immediate dashboard refresh.

Files: `ui/src/operator/{Operator,Operations}.tsx`, `ui/src/styles/operator.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Remove obsolete BSC gas warnings from the Simplex UI

Removed the chain-card warning renderer from setup and operator network screens, deleted the obsolete
chain-note metadata, and removed the BNB-specific native-gas text from the setup review. BSC paymaster
support is now represented consistently throughout the Simplex UI; runtime paymaster behavior is
unchanged.

Files: `src/cli/init/chains.ts`, `ui/src/{operator/Chains.tsx,wizard/steps/{Chains,Review}.tsx}`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Give the shared operator sheet complete motion

Kept the shared shadcn-style Radix sheet used by every operator drawer and replaced its mount-only
effect with state-aware motion. The panel now enters from fully off canvas, exits before Radix removes
the portal, and coordinates both directions with an overlay fade; reduced-motion users receive the
same state change without animation.

Files: `ui/src/styles/operator.css` and `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Show current market prices in the operator list

Operator market rows now show the first valid configured buy and sell prices with their price unit.
Missing sides, venue-priced markets, and reference-only entries remain free of fabricated values; the
underlying market data and pricing behavior are unchanged.

Files: `ui/src/operator/OperatorMarkets.tsx`, `ui/src/styles/{operator,responsive}.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Seed USDC and USDT markets for CNGN

The setup wizard now seeds both USDC/CNGN and USDT/CNGN markets when CNGN and USDT are available in
the selected network's token catalog. Networks without that catalog combination retain the existing
single-market default, and user-created or existing markets are unchanged.

Files: `ui/src/wizard/state.ts`, `ui/src/wizard/strategies/useStrategiesModel.ts`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Recompose operator market drawers

Reworked the shared drawer spacing and live-market editor around the onboarding UI's open editorial
hierarchy. Market identity, risk limits, pricing directions, and actions now flow as flat sections
separated by restrained rules instead of nested cards; only the price chart retains a quiet visual
canvas. Order-limit controls align on one baseline, each direction uses the full width, previews and
point inputs share a balanced desktop row, and disabled sides use an inline action. Create-market and
standard operator drawers inherit the same header spacing and overflow-safe shell.

Files: `ui/src/operator/markets/StrategyMarketEditor.tsx`,
`ui/src/styles/{operator,responsive}.css`, and `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Make new web markets uncapped by default

New markets created from the operator form or setup wizard now leave the optional maximum-order field
blank, which emits no `maxOrderSize` and therefore creates an uncapped market.

Files: `ui/src/operator/markets/{CreateMarketForm.tsx,useCreateMarket.ts}`, `ui/src/wizard/state.ts`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Show configured market prices in setup summaries

Updated setup-wizard market rows to show the first configured Buy and Sell prices, including their
`token1/token0` unit, as soon as either curve has a value. The order-cap summary was removed from the
row while the cap input remains available in the Configure editor.

Files: `ui/src/wizard/steps/Strategies.tsx`, `ui/src/styles/{markets,responsive}.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Remove the BSC paymaster warning from network setup

Removed the stale BSC and BSC Chapel paymaster caveats from the onboarding chain catalog so selecting
those networks no longer displays the native-gas warning in the network setup step. Runtime and review
funding behavior remain unchanged.

Files: `src/cli/init/chains.ts` and `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Improve dashboard token balance cards

Reworked each network's token balances into image-led cards using the existing Simplex token asset
library. The new hierarchy separates total ownership from wallet and vault balances, highlights the
amount currently available to fill, and keeps partial or unavailable data visibly distinct without
presenting it as zero. Network gas remains visible in a compact network header.

Files: `ui/src/operator/OperatorOverview.tsx`, `ui/src/styles/{operator,responsive}.css`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Clarify setup delegation failures

Setup now translates the internal all-chain EIP-7702 shutdown message into network-aware funding,
RPC/bundler, retry, and image-version guidance while retaining raw messages for unrelated startup
failures. The failed-state action is labelled Retry startup and makes clear that the configuration
was already saved.

Files: `ui/src/wizard/startError.ts`, `ui/src/wizard/steps/Review.tsx`,
`src/tests/setup-completion.test.ts`, and `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Installable offline UI and complete dashboard liquidity accounting

Made the Simplex UI installable as a desktop PWA with a manifest, branded FX icons, an offline app
shell, and a permanent install entry in both setup and the operator dashboard. The desktop-only
guide now walks through one generic three-step flow with visual examples; cancelled or unavailable
native prompts report through toasts rather than altering dialog layout.

Replaced the dashboard's wallet-only stablecoin total with an explicit liquidity model. The vault
planner now exposes a mutex-consistent read-only snapshot, and the balance API reports wallet,
vault position, vault availability, wallet reserve, total holdings, and actual available liquidity
per asset. Initial balances are loaded during startup instead of after a five-second timer, failed
reads are surfaced as partial/unavailable state rather than silently rendered as zero or a dash,
and the dashboard presents the full breakdown per network.

Files: `src/{core/boot,funding/types,funding/vault/{VaultFundingPlanner,VaultLiquidityState},index,services/BalanceProvider,services/server/{dto,static}}.ts`,
`src/tests/{balance-provider,funding/vault,ui-server}.test.ts`, `ui/{index.html,public,src}`, and
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Explain vault balance controls and seed curated defaults

Added focusable info icons beside the sweep-threshold and minimum-wallet-balance labels using the
shared `@hyperbridge/ui` tooltip primitives. Newly selected Aave stataUSDC vaults now start at
`20`/`10`, and Yield Bearing cNGN starts at `1000`/`1`, matching the supplied reference; custom and
unknown vaults retain the generic fallback values.

Files: `ui/src/components/VaultRowsEditor.tsx`, `ui/src/styles/treasury.css`, `package.json`,
`../../pnpm-lock.yaml`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Simplify market setup defaults and remove transfer-market setup

Removed the setup wizard's dedicated same-token transfer section and its prefab, state, prefill, and
stylesheet plumbing. Setup and operator market creation now use the normal cross-asset editor, reject
same-asset creation, default new order caps to `50000`, and prefill newly added curve-point sizes with
`1`. Optional field labels now use brackets for clarity.

Files: `ui/src/{components/CurveEditor,operator/markets/{CreateMarketForm,StrategyMarketEditor,useCreateMarket},wizard/state,wizard/steps/Strategies,wizard/strategies/{MarketRow,UniswapPositionsDialog,useStrategiesModel}}`,
`ui/src/styles/{markets,responsive}.css`, `src/cli/init/steps/strategies.ts`,
`src/services/server/{dto,setup-api}.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Prevent the Uniswap pricing view from crashing

Restored the missing Uniswap icon import used by the selected-position summary. Added UI typechecking
to the standard check command so unresolved runtime identifiers fail before the bundle reaches a
browser, and repaired two stale identifiers in the live chain editor uncovered by that check.

Files: `ui/src/wizard/steps/Strategies.tsx`, `ui/src/operator/{Chains.tsx,chains/useChainSettings.ts}`,
`package.json`, `docs/ai/ChangeLog.md`.

## 2026-09-03 — Display Hyperbridge accounts in Polkadot's unified format

Configured the shared Substrate keyring to encode every derived account with Polkadot's unified
SS58 prefix. Setup, review, operator status, copied addresses, balance snapshots, and logs now all
receive the same unified account string without component-specific conversion.

Files: `src/services/substrate-key.ts`, `src/tests/balance-provider.test.ts`,
`docs/ai/{ChangeLog,Decisions}.md`.

## 2026-09-03 — Clarify testnet terminology

Replaced the inaccurate “Sepolia-family” wording in the CLI initializer and web setup wizard with
“EVM test networks”. The supported testnet catalog also includes Polygon Amoy and BSC Chapel, which
are EVM-compatible but are not Sepolia-family chains.

Files: `src/cli/init/steps/chains.ts`, `ui/src/wizard/steps/Signer.tsx`,
`docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-09-03 — Simplify the filler wallet guidance

Shortened the onboarding explanation for the filler wallet and replaced the Unix-specific
`permissions 600` wording with a plain-language assurance that credentials stay private on the
operator's machine.

Files: `ui/src/wizard/steps/Signer.tsx`, `docs/ai/ChangeLog.md`.

## 2026-09-03 — Rebrand the Simplex UI header as HyperFX

Replaced the Hyperbridge mark beside the Simplex product name in both the setup wizard and live
operator dashboard with the supplied white HyperFX wordmark. The transparent wordmark now sits
directly on the dark UI without a backing surface. Added the HyperFX website favicon to the UI build
and taught the local static server to serve bundled WebP assets with the correct MIME type.

Files: `ui/src/{operator/Operator,wizard/Wizard}.tsx`, `ui/src/styles/{operator,foundations,responsive}.css`,
`ui/src/{assets/hyperfx-logo.webp,vite-env.d.ts}`, `ui/index.html`, `ui/public/favicon.ico`,
`src/services/server/static.ts`.
## 2026-09-03 — Stop bidding on phantom orders whose window has closed

A mainnet filler (`0xb98306ac…`, Hyperbridge `12KyapjPpm2fK62gepzZKEEk3xP9vzEJdWBspagx8DxZaj2k`) spent nine hours
bidding on phantom orders that had expired hours earlier. Its bids landed, nothing errored, and it backed no pool
the whole time: the indexer shows 0 `PoolBidder` rows against 692 balance rows, the newest of which stops at
03:03 while healthy fillers keep writing them.

The lag grew monotonically — 4 blocks behind the order it bid on, then 147, 1611, 2228, 2765, 3448 — which is the
signature of a cursor that cannot catch up rather than a one-off stall. `pollPhantomOrders` advances the cursor
by at most `maxBlocksPerPoll` (10) per tick and never skips ahead, so a deficit accumulated while ticks were lost
(a scan overrunning the 15s interval, or the rate-limit backoff sitting out up to 8 ticks) is only repaid if the
sustained rate beats the chain's. Below that, the filler stays behind forever, and every order it then sees is
already dead: the bid window is tens of blocks.

Two fixes, at the two places the delay can come from:

- The SDK's poll now abandons a backlog older than one generation cycle rather than walking it, and the scanner
  logs each skip: a filler that keeps skipping is one falling behind the chain.
- `handlePhantomOrders` drops events older than the pallet's own bid window (read from chain, plus a 2-block
  margin) against one head read per batch. That also covers the delay the scanner cannot see — the global queue this runs on, and the
  quoting inside `preparePhantomBid` — and turns a silent outage into a warning naming the lag. A failed head
  read keeps every event: one flaky endpoint must not stop the filler bidding.

Neither is a substitute for the other: the first stops the backlog forming, the second refuses to act on one that
does.

Files: `src/scanner/hyperbridge-scanner.ts`, `src/core/filler.ts`, `src/tests/core/phantom-bid-staleness.test.ts` (new).

## 2026-09-02 — Remove `skipPermit`; delegation ops may use Simplex PERMIT mode

Deleted the `skipPermit` flag end to end: `SponsoredUserOpRequest.skipPermit`, `PaymasterOptions.skipPermit`, the `SimplexPaymasterOptions` interface and `buildSimplexPaymasterData`'s trailing options parameter, and the `skipPermit: true` that `DelegationService.setupDelegationViaBundler` passed. `hasPermit` is now just `await tokenSupportsPermit(client, tokenAddress)`. Delegation ops therefore reach EIP-2612 PERMIT mode on permit-capable tokens instead of being routed past it into the PERMIT2/APPROVE branch, whose bootstrap needs a native-funded `approve` — unsendable by a solver holding zero native, which left delegation with no sponsored path at all (observed on Base and Arbitrum). Test call sites drop the argument; every one of them already mocked a no-permit token, so mode selection is unchanged there.
Files: `src/services/DelegationService.ts`, `src/services/UserOpSender.ts`, `src/services/paymaster/index.ts`, `src/services/paymaster/types.ts`, `src/services/paymaster/provider/simplex.ts`, `src/tests/services/SimplexPaymaster.test.ts`, `src/tests/services/UserOpSender.test.ts`, `src/tests/services/SimplexPaymasterPermit2.probe.test.ts`, `docs/ai/Flow.md`, `docs/ai/Decisions.md`, `CHANGELOG.md`.

## 2026-09-02 — Harden Simplex-first selection (PR #1196 review)

Wrapped the `buildSimplexPaymasterData` call in `buildPaymasterAndData` in try/catch: a builder failure warns, joins `skipReasons` as `simplex: <message>`, and falls through to Circle instead of aborting selection (previously a throw lost the bid on the `prepareBidUserOp` path). `tokenSupportsPermit` now discriminates contract reverts from transport errors like `paymasterSupportsPermit2` — a transport error propagates (and demotes to Circle) instead of reading as "no permit". Test mock for `version()` updated to throw viem-shaped errors; added selection-level throw tests and a permit-probe transport test.
Files: `src/services/paymaster/index.ts`, `src/services/paymaster/provider/simplex.ts`, `src/tests/services/PaymasterSelection.test.ts`, `src/tests/services/SimplexPaymaster.test.ts`, `docs/ai/Decisions.md`, `CHANGELOG.md`.

## 2026-09-02 — Prefer Simplex paymaster over Circle in selection

Flipped the candidate order in `buildPaymasterAndData`: Simplex is evaluated first (deposit gate, then builder — the gate still precedes the builder because of its bootstrap approve tx), Circle second (USDC balance, then gate, then builder), `type: "none"` fallthrough unchanged. Each branch's internal gate semantics, the 150% headroom, and the fail-open deposit reads are untouched; only the order and the order-describing docs changed. `paymasterVerificationGasLimit` stays Circle-only, so it now only bites when Circle is the survivor (see Decisions).
Files: `src/services/paymaster/index.ts`, `src/services/paymaster/types.ts`, `src/services/UserOpSender.ts`, `src/services/ContractInteractionService.ts`, `src/services/DelegationService.ts`, `src/core/boot.ts`, `src/cli/init/help-text.ts`, `src/tests/services/PaymasterSelection.test.ts`, `docs/ai/Flow.md`, `docs/ai/Decisions.md`.

## 2026-09-01 — Paymaster selection gated on EntryPoint deposit

`buildPaymasterAndData` now skips a candidate paymaster whose EntryPoint deposit cannot cover the
op's max prefund with 150% headroom, falling through Circle to Simplex to `type: "none"` with a
per-candidate reason. Motivated by a live incident: on Base the Circle paymaster's deposit was
drained (5.04e12 wei against a 1.49e13 prefund) and the Simplex paymaster's deposit was zero, so
every bid was signed against a paymaster the bundler was bound to reject
("precheck failed: paymaster deposit is X but must be at least Y") — the paymaster is baked into
the signed bid UserOp, so nothing could re-select at execution. Callers pass the op's gas terms:
`prepareBidUserOp` from the cached estimate, `trySendSponsored` from the caller's fixed limits or
the fallbacks — for which `getGasPrice` moved above paymaster selection (also turning a gas-price
failure into the safe never-submitted null). The Simplex gate runs before its builder, which can
send a bootstrap approve tx. Deposit-read failures fail open. Skips are warn-logged with deposit
and required figures.
Files: src/services/paymaster/types.ts, src/services/paymaster/index.ts,
src/services/UserOpSender.ts, src/services/ContractInteractionService.ts,
src/tests/services/PaymasterSelection.test.ts.

## 2026-08-31 — Review fixes: zero-code guard, abandoned-request cleanup, logger and credentials injection, unit tests

Four fixes from review of the MPCVault retry change. `apiErrorText` no longer treats enum value 0
as an error — both code fields define 0 as UNSPECIFIED, so the previous `!== undefined` check would
have failed every signature against a server that sends an explicit zero; the guard is also now
framed as defensive, since the observed `{"code":16,"message":""}` was a gRPC status envelope, not
the in-band `Error` message. A signing request whose execute fails terminally is best-effort
rejected in the vault (`rejectSigningRequest`) instead of staying pending forever — a pending
request nobody will execute is a standing authorization to sign a stale payload. `MpcVaultClientConfig`
and `MpcVaultSignerConfig` accept an injected `logger` (the default process-wide context is
invisible to embedded fillers that pass `SimplexOptions.logger`) and `credentials` (the TLS
hardcoding left no test seam). New unit tests run `MpcVaultService` against an in-process gRPC
server: retry-then-sign, retries-exhausted with reject and error naming, code-only errors,
UNSPECIFIED-zero, and create-failure naming. Create deliberately gets no retry — see Decisions.md.
Files: src/services/wallet/mpcvault.ts, src/services/wallet/types.ts,
src/services/wallet/accounts/mpc.ts, src/tests/wallet/mpcvault.test.ts.

## 2026-08-31 — MPCVault RPC failures name their call, and execute retries the create/execute race

`MpcVaultService` rethrows gRPC failures naming the RPC, the signing-request uuid and MPCVault's
x-request-id (previously the bare grpc-js error propagated, so a production
`3 INVALID_ARGUMENT: Invalid uuid` could not be attributed to create vs execute from the order
log). The app-level error guard now also trips on code-only errors — MPCVault sends `message: ""`
with only a code set. `executeSigningRequest` retries INVALID_ARGUMENT/NOT_FOUND twice with short
backoff because MPCVault intermittently rejects a uuid its own createSigningRequest just returned,
before the callback co-signer is contacted. Created uuids are logged at debug for dashboard
correlation.
Files: src/services/wallet/mpcvault.ts.

## 2026-08-31 — Simplex UI cleanup and operator-market module

Reviewed the onboarding and operator UI against the agreed design system and extracted live market
administration from the dashboard shell into a dedicated module. Runtime market mutations now reject
duplicate submissions synchronously, token selection uses the shared image-rich control, mobile
operator navigation remains fixed in view, and the wizard lists every unresolved requirement instead
of hiding additional blockers. The live chain editor now shares the onboarding logos, collapsibles,
plain-language endpoint labels, and toast feedback. Added a clean UI check command and corrected the
setup flow notes to match the current navigation and validation ownership.

Follow-up cleanup moved application boot state into a discriminated-state hook, extracted the render
error boundary, replaced native dashboard drawers and wizard dialogs with Radix-backed shadcn-style
Sheet/Dialog primitives, split the monolithic stylesheet into ordered domain files, and separated
market and chain business logic from their view modules.

Files: `ui/src/app/`, `ui/src/operator/`, `ui/src/wizard/{steps,strategies}/`,
`ui/src/components/{ui/,OperatorSheet,ScreenErrorBoundary}.tsx`, `ui/src/lib/hooks.ts`,
`ui/src/styles/`, `package.json`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-08-27 — Hyperbridge-branded Simplex onboarding

Restyled the setup wizard around the shared Hyperbridge visual language: Aeonik and Aeonik Mono,
the Hyperbridge mark and spectrum accent, the `#131417` canvas, blue-black surface layers, muted
`#929daa` copy, white primary actions, and rounded controls. The wizard now presents a persistent
desktop journey rail, a compact horizontally scrollable mobile rail, a step-level progress header,
and a local-credentials reassurance without changing any setup state or API behavior. The app shell
also anchors the brand gradient to the top edge and centers all states in a max-width container.

Files: `ui/src/App.tsx`, `ui/src/styles.css`, `ui/src/wizard/Wizard.tsx`,
`ui/src/assets/hyperbridge-logo.svg`, `ui/src/assets/fonts/*.woff2`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-08-27 — Phantom orders are validated against the pallet's shape before they are quoted

`IntentFiller.preparePhantomBid` now refuses any phantom order that does not look like one
`phantom_order_commitment` built: a non-zero `session`, a non-zero output amount, or a
`source`/`destination` that is not the announced chain. Those checks are pure field comparisons —
no RPC. On top of them, the destination head is read and an order that is still fillable is
refused; that read is best-effort, and a failure warns and continues rather than stopping the
filler bidding.

The reason any of it is needed: `quotePhantomFill` deliberately runs with no budget, no
wallet-balance read and neither profit gate, so the amounts signed are bounded only by the order
body. The body is read from a single Hyperbridge node's offchain storage, and `fetchPhantomOrder`
assigns `id` from the event rather than re-deriving the commitment from the bytes — so it is
unauthenticated. A forged body would have turned that unbounded quote into a signed, executable
authorization to fill an order of someone else's choosing, paying out to a beneficiary named in
the same body.

Each invariant independently breaks that. `session` is the direct one: `_select` recovers a key
with `ECDSA.recover`, which can never return the zero address, so a genuine phantom order can
never have a solver selected at all.

Found by the scheduled IntentGateway/Simplex security audit.

The unit tests now also rebuild an order the way `phantom_order_commitment` does, encode it with
the real `placeOrder` ABI and decode it the way `fetchPhantomOrder` does, before running the guard
over it. The hand-built fixtures otherwise bake in the assumption the guard depends on — that a
pallet-generated order presents `source`/`destination` as the same string the event carries in
`chain` — and getting that wrong would refuse every genuine phantom order and silently stop the
filler bidding, which only the simnode E2E would have caught.

The simnode E2E was seeding `LatestStateMachineHeight` with `createType("u64", h).toHex()`.
polkadot-js renders integers big-endian in hex while SCALE stores them little-endian, so the
runtime read `0x00000000000f4240` back as 4.6e18: every phantom order in that suite carried a
far-future deadline and was, contrary to the entire point of a phantom order, genuinely fillable.
Nothing noticed until this guard refused to quote them. Fixed to seed `toU8a()`.

Files: `src/core/filler.ts`, `src/tests/core/phantom-order-validation.test.ts`,
`src/tests/phantom-filler.e2e.simnode.test.ts`.

## 2026-08-27 — Bids carry an on-chain expiry (`bidValiditySeconds`)

Every bid this filler signs now sets `FillOptions.validUntil`, so `fillOrder` reverts `FillExpired` once the quote
has gone stale. Configured as `simplex.bidValiditySeconds`, default 300 (5 minutes).

A bid is a firm price the order placer takes up whenever they choose, and nothing bounded that window:
`order.deadline` is placer-chosen with no ceiling, and `enqueueRetraction` only clears the bid on Hyperbridge, which
has no effect on the destination chain. A bid signed at one rate stayed executable indefinitely and was exercised only
if the rate moved against us — a written option on this filler's inventory, at no premium. Volatile pairs
(USDC/CNGN) are the worst case, since the naira reprices in steps rather than drifting.

Operators configure seconds because that is the unit the risk is in; the contract compares block numbers, so the
value is converted per destination chain from the chain's nominal block time (`Chain.blockTime`, milliseconds in
viem), with a 30-second discovery allowance added before the conversion and the result rounded up — seconds rather
than a block count, because the lag between reading the head and the fill landing is wall-clock and does not scale
with block time.

`buildApprovalAndFillCalldata` encodes through the SDK's version-aware codec, since gateways predating the field take
a differently-selectored `fillOrder`. On such a chain the bound is dropped — there is nowhere to put it — and the
filler warns once per chain rather than silently believing itself protected.

Found by the scheduled IntentGateway/Simplex security audit.

Files: `src/services/ContractInteractionService.ts`, `src/services/FillerConfigService.ts`,
`src/config/filler-toml.ts`, `src/config/abis/IntentGatewayV2.ts`, `src/core/boot.ts`,
`filler-config-example.toml`, `src/tests/services/bid-validity-config.test.ts`.

## 2026-08-26 — `decimals()` read failures fall back to the asset registry instead of guessing 18

`ContractInteractionService.getTokenDecimals` previously swallowed a failed on-chain `decimals()`
read and returned a hardcoded 18. That value flows into `computeLegPolicyOutput`, which scales
`policyMaxOutput` by `10 ** decimals` — so a 6-decimal token (USDC/USDT/cNGN) misread as 18
inflates the computed payout by 10^12. Since the overfill clamp is disabled, nothing bounds the
result back to the user's requested output, and the filler would size the leg against its whole
wallet balance.

The correct values were already in the tree: `chain.ts` carries a per-chain `tokenDecimals` table
and `ChainConfigService.getAssetMetadataByAddress` resolves it by address. Nothing in simplex
consulted it — `CacheService.tokenDecimals` starts empty and its only writer is the success path
of the very read that just failed. The catch branch now consults the registry through a new
`FillerConfigService.getAssetDecimalsByAddress` delegator, and raises a hard error when neither
the RPC nor the registry can supply a value — there is no safe guess, so the order is skipped
instead. Every caller was audited to confirm a throw skips one order rather than escaping:
`initCache` runs unawaited from the constructor and now swallows its own failures, so a boot-time
RPC hiccup cannot become an unhandled rejection.

Found by the scheduled IntentGateway/Simplex security audit.

Files: `src/services/ContractInteractionService.ts`, `src/services/FillerConfigService.ts`,
`src/tests/services/ContractInteractionService.decimals.test.ts`.

## 2026-08-26 — Final-review fixes: cause-chain probe classification, batching guards (#1071)

Second review round on the 08-24 fixes; the transport-error fix (F4) did not survive contact with viem, and the new batching/zero-first code had gaps.

- **Probe classification actually works now.** viem 2.47.6's `readContract` wraps _every_ failure — HTTP 429/timeout included — in `ContractFunctionExecutionError` (verified empirically against the installed package), so the 08-24 `instanceof` check still cached transport errors as "unsupported". `paymasterSupportsPermit2` now classifies by the cause chain: `error.walk(e => e instanceof ContractFunctionRevertedError || e instanceof ContractFunctionZeroDataError)` marks a genuine revert; everything else propagates uncached. Both unit-test mocks were reshaped to throw what viem actually throws (the old transport mock threw a bare `Error`, which real viem never does — the test was validating a fantasy), and the transport test now also proves the negative was not cached by probing again with a healthy client.
- **Zero-only batching.** `resolvePendingPermit2Approval` returns null for any non-zero allowance, not just one at the recommendation: the batched delegation tx approves max directly and skips simulation (explicit gas), so a stale partial allowance on a USDT-rule token was a deterministic on-chain revert. Stale-allowance cases defer to `sendFundedApprove`'s zero-first path.
- **Delegation retry checks `isDelegated` first.** EIP-7702 applies authorization tuples before execution and keeps them applied when execution reverts, so a batched tx that reverted on the approve usually still delegated — the retry now costs one `eth_getCode` instead of a full second tx.
- **Pre-check budgets the whole sequence.** `sendFundedApprove` reads the allowance up front and requires native for two txs when a zero-first reset is needed; previously dust for exactly one tx passed the check, landed the reset, and died mid-sequence with the allowance stuck at zero.
- **The 100k ceiling is now tested for profitability.** The gas-grief loop covers {100k, 40k, 30k}; 100k is where the EntryPoint penalty actually applies and is the case the restored ceiling's safety argument rests on. Stale "cap 40k" comments/labels in the same file and the `POST_OP_GAS_LIMIT_SIMPLEX` comment ("matching its on-chain MAX") corrected; the Permit2 fork test's `_maxCost` now sums the 40k postOp limit `_opWithData` actually packs (was 100k, overstating requiredPrefund).

Files: `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/types.ts`, `src/services/DelegationService.ts`, `src/tests/services/SimplexPaymaster.test.ts`, `evm/tests/foundry/{SimplexPaymasterGasGriefTest,SimplexPaymasterPermit2ForkTest}.t.sol`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-08-24 — PR #1147 review fixes (#1071)

Ten inline findings from Seun's review, all addressed.

- **F1 postOp rollout** — reverted `MAX_POST_OP_GAS_LIMIT` to 100k (kept `MIN`=30k); only the SDK value stays at 40k. An in-place upgrade of a live proxy (needed for stake recovery) no longer rejects clients still sending 100k. Contract + Solidity tests + CHANGELOG/Decisions updated.
- **F2 delegation fail-open** — the native-fallback batched delegate+approve now retries as a plain self-call on any revert (`trySendDelegation`), so a token that rejects the approve can't block delegation forever.
- **F3 zero-first approve** — `sendFundedApprove` resets a stale non-zero allowance to zero before approving max (Ethereum USDT rule).
- **F4 narrowed probe catch** — `paymasterSupportsPermit2` only caches a real contract revert as "unsupported"; a transport error propagates instead of silently reverting to a native paymaster allowance.
- **F5 chain-keyed probe cache** — keyed by `${chainId}:${address}` so a shared CREATE2 address can't leak support across chains.
- **F8 dropped `permit2DeadlineSeconds`** — unreachable config removed; fixed 1h deadline documented.
- **F6/F7 test rigor** — added assertions to the gas-band token test and named the exact reverts (`InvalidPostOpGasLimit`, `InvalidSigner`/`InvalidAmount`/`InvalidNonce`/`AllowanceExpired`) in the gas-grief and compromise fork tests.
- **F9 doc fix** — `sendFundedApprove` docstring and `Flow.md` now say two confirmations.
- **F10** — mode-0x02 `_prefund` uses the base's `prefunder_` instead of re-reading `userOp.sender`.

Files: `evm/src/utils/SimplexPaymaster.sol`, `evm/tests/foundry/{SimplexPaymasterTest,SimplexPaymasterGasGriefTest,Permit2CompromiseForkTest}.t.sol`, `src/services/paymaster/{provider/simplex.ts,types.ts,index.ts}`, `src/services/DelegationService.ts`, `src/tests/services/SimplexPaymaster.test.ts`, `modules/pallets/intents-coprocessor/src/{benchmarking,tests}.rs`, `parachain/runtimes/{gargantua,nexus}/src/weights/pallet_intents_coprocessor.rs`.

## 2026-08-20 — PR #1147 review: v,r,s signature layout + batched delegation approve (#1071)

Two changes from Seun's review of the PERMIT2 PR.

**Signature layout.** Mode 0x02's `paymasterData` now carries the Permit2 signature as explicit `uint8(v), bytes32(r), bytes32(s)` fields instead of a 65-byte `bytes` blob, mirroring the EIP-2612 mode 0x00 layout. Same 182-byte length; the contract reconstructs `abi.encodePacked(r, s, v)` for the Permit2 call. Client (`permit2.ts` / `provider/simplex.ts`) splits the signature before packing; fork/unit tests updated. Note: this keeps mode 0x02 65-byte-ECDSA-only, which the fixed length already enforced — no ERC-1271-length flexibility is lost.

**Batched delegation approve.** When Simplex falls back to a native EIP-7702 delegation (bundler/sponsored path unavailable), it now folds the one-time `approve(Permit2, max)` into that same set-code tx on no-permit chains, so the bootstrap costs one native tx (delegate + approve) instead of two. New `resolvePendingPermit2Approval` in `provider/simplex.ts` decides the token (null when a permit token, an approval already in place, no Permit2/PERMIT2-mode, or no fee-token balance yet); `DelegationService.sendDelegationTransaction` takes an optional approval and sets `to = token, data = approve(...)` with the authorization list attached (the delegation lands regardless of `to`). Best-effort: any resolver failure falls back to the plain self-call delegation. Scope is the native fallback only (per review).

Files: `evm/src/utils/SimplexPaymaster.sol`, `evm/tests/foundry/SimplexPaymasterPermit2ForkTest.t.sol`, `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/permit2.ts`, `src/services/DelegationService.ts`, `src/tests/services/SimplexPaymaster.test.ts`.

## 2026-08-19 — Security-review fixes: stake recovery, postOp gas band (#1071)

A threat-model audit of the paymaster (fork-executed, not just read) produced three fixes carried here.

**Stake recovery.** `PaymasterCore.addStake` was ungated while `unlockStake`/`withdrawStake` route through `_authorizeWithdraw()`, which reverts unconditionally, and no governance request kind covered stake — so staked native was permanently unrecoverable. Verified against the live deployments: 0.1 native was already stuck on each of Ethereum, BSC and Arbitrum, and any unprivileged address could call `addStake{value: 1 wei}(type(uint32).max)` to stretch the unstake delay from one day to 136 years, defeating even a future upgrade. Added `RequestKind.UnlockStake` / `RequestKind.WithdrawStake` (discriminators 5 and 6, both empty-payload, destination always the treasury) and gated `addStake` to the treasury.

**postOp gas band.** The EntryPoint penalises the unused part of `paymasterPostOpGasLimit` after fixing the cost handed to `postOp`, so that penalty is never billed to the user; it waives the penalty entirely while `gasLimit <= gasUsed + 40_000`. The SDK pinned the limit at the contract's 100k cap while postOp actually needs ~8-12k, so roughly half the per-op margin was being burned. `MAX_POST_OP_GAS_LIMIT` is now 40_000 — the largest unconditionally penalty-free value — and a `MIN_POST_OP_GAS_LIMIT` of 30_000 closes the refund-underflow window that an unbounded-below limit left open. Fork-measured margin at the cap rose from 9,053,104,589,778 to 21,354,113,277,112 wei per op, and is now identical at the cap and the floor.

The SDK's shared `POST_OP_GAS_LIMIT` split into `POST_OP_GAS_LIMIT_SIMPLEX` (40k) and `POST_OP_GAS_LIMIT_CIRCLE` (100k) — it was feeding both paymasters, and Circle's is a different contract whose postOp was never measured here.

Files: `src/services/paymaster/types.ts`, `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/provider/circle.ts`, `src/tests/services/UserOpSender.test.ts`, plus `evm/src/utils/SimplexPaymaster.sol`, `evm/tests/foundry/{SimplexPaymasterTest,SimplexPaymasterPermit2ForkTest,SimplexPaymasterGasGriefTest,SimplexPaymasterStakeLockForkTest}.t.sol` and `modules/pallets/intents-coprocessor/src/{lib,types,weights}.rs`.

## 2026-08-18 — Permit2 mode for the Simplex paymaster (#1071)

The Simplex paymaster client gained mode `0x02 PERMIT2`. On chains whose fee token has no EIP-2612 permit (BSC pegged USDC/USDT), the solver previously kept a $5 standing allowance to the paymaster and refilled it with a native-funded `approve` every time it dipped under $2. Now the bootstrap is a single funded `approve(Permit2, max)` per token, and every subsequent op carries a per-op Permit2 `PermitTransferFrom` signature (spender = paymaster, amount = the same $5 cap, random 256-bit nonce, 1 hour deadline) packed as `mode(1) + token(20) + permitAmount(32) + nonce(32) + deadline(32) + signature(65)`. Selection order in `buildSimplexPaymasterData`: EIP-2612 PERMIT when the token supports it (unless `skipPermit`), then PERMIT2 when the token is already approved to Permit2, then APPROVE while a legacy paymaster allowance is still in place, then the bootstrap approve. Mode 2 is only used when the paymaster deployment exposes `PERMIT2()` (older deployments reject it with `InvalidMode`), probed once per paymaster address and cached. `PaymasterOptions.forceApproveMode` was renamed to `skipPermit` since PERMIT2 stays allowed for delegation ops; `ensureCappedApproval` became the generic `sendFundedApprove`.

Live probe on Base Sepolia through Alchemy's bundler (deployment script `evm/script/SimplexPaymasterPermit2Probe.s.sol`, env-gated suite `src/tests/services/SimplexPaymasterPermit2.probe.test.ts`): a fresh EOA's sponsored EIP-7702 delegation and a follow-up no-op from the delegated account were both accepted in mode 2 (`Permit2Executed` on-chain), answering the ERC-7562 question for that bundler. The very first op right after the bootstrap approve was rejected `AA33` twice in a row until the approve was one more block old, so `sendFundedApprove` now waits for two confirmations.

Files: `src/services/paymaster/permit2.ts` (new), `src/services/paymaster/provider/simplex.ts`, `src/services/paymaster/types.ts`, `src/services/paymaster/index.ts`, `src/services/UserOpSender.ts`, `src/services/DelegationService.ts`, `src/config/abis/SimplexPaymaster.ts`, `src/tests/services/SimplexPaymaster.test.ts`, `src/tests/services/UserOpSender.test.ts`, `src/tests/services/SimplexPaymasterPermit2.probe.test.ts` (new).

## 2026-08-20 — The filler only takes single-leg orders

`EventMonitor.handleOrder` now forwards an order only when it has exactly one input asset and one
output asset. Anything else is dropped at the door with an `orderSkipped` event carrying the reason
`Multi-leg order`, so the operator sees it in the activity feed rather than losing it silently.

The check runs before the de-duplication set is touched: a rejected order id is never marked as
seen, so a later single-leg order sharing that id is still delivered.

Downstream multi-leg handling is untouched — `FXFiller`'s per-leg loops and the leg-splitting in
`filler.ts` still run for phantom orders, which do not come through this path.

Files: `src/core/event-monitor.ts`, `src/tests/core/chain-lifecycle.test.ts`, `package.json`.

## 2026-08-20 — Regression test: fills pay the curve amount

The payout fix below restored `targetOutput = policyMaxOutput`, but nothing asserted the payout —
the original regression landed silently precisely because no test pinned `calculateProfitability`'s
cached outputs (the figure `prepareBidUserOp` signs into the bid) against the curve.
`fx.curve-payout.test.ts` drives `calculateProfitability` with mocked chain access and pins the
three sizing outcomes: an uncapped leg pays the curve amount, not the user's requested amount; a
capped leg pays the capped slice's worth at the curve, not the user's pro-rata ask; and a
balance-limited leg pays what the wallet covers — a full fill when that still clears the ask.
Verified to fail on the pre-fix clamp: reintroducing `min(policyMaxOutput, desiredOutput)` fails
all three cases with exactly the clamped amounts.

The file is added to `test:filler`, the script CI actually runs — a payout test the CI never
executes would repeat the original failure mode. (`pnpm test` runs the full suite, but CI does not;
`pairs.test.ts`, which exercises the profit gates, is in no CI script at all and currently fails 12
of its cases on main — 9 predating #1154 and 3 from #1154 unrationing phantom probes without
updating the cap expectations. Repairing that suite and wiring it into CI is a separate task.)

Files: src/tests/strategies/fx.curve-payout.test.ts (new), package.json, docs/ai/ChangeLog.md.

## 2026-08-20 — A per-order cap can be removed, from the UI and over the API

The previous entry made `maxOrderSize` optional in config but left it one-way at runtime: an
operator could add a cap to an uncapped market, and could not take one off without editing the TOML
by hand. Both halves of that are now closed.

`AdminStrategy` gains `clearMaxOrderSize?: () => void` alongside `setMaxOrderSize`, implemented in
`adminStrategyFor` by setting the live `TradingPair.maxOrderSize` to `undefined` — the engine reads
the cap per order, so removal binds on the next evaluation exactly as a resize does.

`DELETE /api/strategies/:index/max-order-size` exposes it, on its own route rather than as a null on
the existing `PUT /api/strategies/:index`. `DELETE /api/strategies/:index` already means "remove the
market"; a cap removal one typo away from a market removal is not worth one fewer endpoint. The
handler is idempotent and refuses reference-only markets, matching the PUT.

`Simplex.clearMaxOrderSize(index)` is the library equivalent, shaped after the existing
`clearCurve`.

In the operator dashboard, the cap field is now blankable: emptying it turns the button into
"Remove cap" and issues the DELETE, while any other edit still PUTs. The button enables on any
change from the persisted value, including set-to-blank, which the old `!maxOrderSize.trim()` guard
disabled. The setup wizard's cap fields accept blank too and omit the key entirely rather than
emitting `""`, which config validation would reject as a malformed decimal rather than read as "no
cap".

Files: src/core/boot.ts, src/services/server/UiServer.ts, src/simplex.ts,
ui/src/operator/Operator.tsx, ui/src/wizard/state.ts, ui/src/wizard/steps/Strategies.tsx.

## 2026-08-20 — Fills pay the curve amount again, not the user's requested amount

`FXFiller` had stopped overfilling entirely: every fill paid out exactly
`order.output.assets[i].amount`. `targetOutput` was `min(policyMaxOutput, desiredOutput)`, and
`desiredOutput` is `output.amount` whenever the leg is not exposure-capped. Combined with the
acceptance gate just below it (`if (policyMaxOutput < desiredOutput) return 0` — skip when the
curve pays less than asked), the two form a pincer: any order that survives to a fill has
`policyMaxOutput >= desiredOutput`, so `targetOutput` was always `desiredOutput`. Not an edge
case — 100% of non-balance-limited fills paid the requested amount and nothing more, whatever the
configured exchange rate said. Observed on Base fill `0x60b299e8...`: 25.469 ycNGN redeemed to
1,380 cNGN and exactly 1,380 cNGN forwarded, no surplus transfer and no `DustCollected`.

Introduced by #1123, which added `capFraction`/`desiredOutput` for the `maxOrderSize` exposure cap
and then reused `desiredOutput` as the general fill target — conflating "the slice the cap allows"
with "the amount to pay". Before #1123 the target was `policyMaxOutput` outright. No test asserts
the payout against the curve, so it landed silently.

`targetOutput` is now `policyMaxOutput` in every case — see the entry below, which took the cap
branch back out after this landed.

Files: src/strategies/fx.ts.

## 2026-08-20 — The curve amount is the payout unconditionally, and `maxOrderSize` is optional

Two changes to the same sizing decision.

**`targetOutput = policyMaxOutput`, capped or not.** The fix above still let the exposure cap clamp
the payout down to the user's pro-rata ask on a capped leg. It no longer does. `maxOrderSize` still
binds where it always did — `computeLegPolicyOutput` rations `token0ForLeg` against the pair's
remaining budget before the rate is applied, so a capped leg's curve amount is already the capped
slice's worth. What changes is the escrow side: paying above the pro-rata ask draws down more input
than the cap fraction nominally allots. That is more of the user's token for the same outlay, and
the outlay itself is still capped.

`capLimited` had to follow. It was `desiredOutput < output.amount` — true whenever a cap was active
at all — and it forces the partial-fill eligibility gate. With the payout no longer clamped to
`desiredOutput`, a curve running far enough above the order's rate can cover the whole ask out of a
capped slice; that is a full fill and gating it as a partial would reject cross-chain and calldata
orders the filler can actually serve. The condition is now `capFraction.lt(1) && policyMaxOutput <
output.amount` — the cap is active _and_ it actually shortens the fill.

**`maxOrderSize` is now optional.** `TradingPair.maxOrderSize` is `Decimal | undefined`; absent
means uncapped, and the pair fills every order at its full notional. `validatePairConfigs` no
longer requires it (a malformed value is still rejected), `tradingPairFrom` maps an absent TOML
value to `undefined` instead of the placeholder `new Decimal(0)`, and `sizeOrder` budgets an
uncapped pair against the order's own total so the per-pair ration still stops sibling legs
double-spending the same token0 without ever binding below the order. `FXFiller`'s constructor
check accepts absence and keeps exempting reference-only pairs, which never fill and whose callers
still pass a placeholder `0`.

Test updated: `validatePairConfigs > requires a positive maxOrderSize` asserted the removed throw;
it is now `accepts an omitted maxOrderSize, and rejects a malformed one`.

Files: src/strategies/fx.ts, src/config/pairs.ts, src/core/boot.ts, src/simplex.ts,
src/tests/pairs.test.ts.

## 2026-08-20 — pairs.test.ts catches up with the probe and paymaster changes, and CI now runs it

`src/tests/pairs.test.ts` failed 12 of its 55 tests on main, unnoticed because no CI script ran
the file. Both failure groups were stale test expectations, not product regressions — each was
verified against the intent recorded in the 2026-08-19 entries before editing:

- Three phantom-probe tests still expected `quotePhantomFill` to cap its quote at the pair's
  `maxOrderSize`. The cap no longer rations probes (see "A phantom probe is no longer rationed by
  the pair's exposure cap", and the Decisions entry "The exposure cap governs fills, never
  probes") — that change updated `fx.one-sided-lp.test.ts` only and left this file behind. The
  assertions now expect the full unrationed quotes (e.g. 200,000 ZARP × 100 = 20,000,000 CNGN
  where the old cap produced 10,000,000), and the test names/comments no longer claim probes are
  capped.
- Nine profit-gates tests scored 0 where they expected a positive result (or the partial-fill
  flag). The leg loop now calls `paymasterReserveForToken` (see "Fill sizing reserves the
  paymaster's gas pull"), whose `hasPaymaster` check calls
  `configService.getCirclePaymasterAddress` / `getSimplexPaymasterAddress` — absent on the
  suite's `cfg` mock, so every evaluation threw `TypeError` into `calculateProfitability`'s catch
  and returned 0. Root cause pinned with a throwaway probe test against the exact mock shape. The
  mock now defines both getters as `() => undefined` (no paymaster configured → zero reserve),
  which restores the suite's exact spread/fee arithmetic.

`test:filler` now includes `src/tests/pairs.test.ts`, so the file runs in CI (the "Run simplex
test" step of `.github/workflows/test-sdk.yml`). It is pure-unit — no network, no env. Verified:
55/55 pass via `pnpm vitest run --maxConcurrency=1 src/tests/pairs.test.ts`, biome lint clean on
the file, and `vitest list` collects all four `test:filler` files after codegen.

Files: `src/tests/pairs.test.ts`, `package.json`, `docs/ai/{ChangeLog,Decisions}.md`.

## 2026-08-19 — A phantom probe is no longer rationed by the pair's exposure cap

`quotePhantomFill` fed the pair's per-order exposure budget into `computeLegPolicyOutput`, which
clamps the priced quantity to `min(legMaxToken0, remainingToken0)`. A probe is a price quote, not
an allocation — it commits no capital — so the clamp had no exposure to protect, and when it bound
it corrupted the published price: the output covered less than the standard amount while every
consumer still divides by the FULL standard amount. A pair capped below the probe therefore
published a proportionally worse rate with nothing to signal it. At `maxOrderSize` 10 against a
100-token probe that is a rate 10x too low.

`computeLegPolicyOutput`'s `remainingToken0` is now `Decimal | null`; `null` means "price the whole
input, unbudgeted" and only `quotePhantomFill` passes it. `fill()` passes the real budget exactly
as before, which is where exposure is actually taken.

Two related changes in the same path:

- The rate is now sampled at the leg's OWN notional (`sized.legNotionals[i]`) rather than the
  pair's exposure-capped budget. On a sloped curve, sampling at a smaller notional advertises a
  tighter rate than this filler would give at the probe's size, and optimistic is the one
  direction a published rate must never be — a quote built from it has to stay fillable.
- A probe whose notional exceeds the pair's `maxOrderSize` now logs a warning. The price is
  honest, but no order that size can clear it, so the operator needs to see the config gap.

This mattered because the coprocessor pallet's standard amount is moving from 1 to 1000 tokens to
buy quote precision. At one token the clamp effectively never bound (a pair's whole budget is
~2 tokens against a `maxOrderSize` of thousands); at 1000 it plausibly does.

Files: `src/strategies/fx.ts`, `src/tests/strategies/fx.one-sided-lp.test.ts`.

Not verified: `node_modules` was not installed in this worktree, so `tsc` and `vitest` were not
run. Needs a normal CI run.

## 2026-08-19 — Two more ways a fill could be sized past what the wallet can pay

Both found while tracing the paymaster shortfall below, both with the same failure signature — a credit or a balance that the sizing believed in and the chain did not.

**Uniswap V4 credited liquidity it was not going to remove.** `removeCallParameters` re-derives the decrease as `liquidityPercentage.multiply(position.liquidity).quotient`, which truncates, while the planner priced the fill from the untruncated liquidity it asked for. The gap is always in the reverting direction and scales with the withdrawal: at the old 1e6 denominator, a slice of ~0.0159% of a position shed 0.63% of it, roughly $93 of phantom credit on a $14.8k draw. A new `liquidityRemoval` helper resolves the percentage and the liquidity it actually encodes together, at a 1e18 denominator, and the planner now credits and consumes that figure and hands the same `Percent` to the SDK. It also returns null for a slice too thin to register rather than letting the SDK's ZERO_LIQUIDITY invariant throw.

**The escrow-release dispatch fee was unaccounted for.** `HyperApp.dispatchWithFeeToken` pulls `dispatchFee` from the solver's wallet in the destination host's fee token — USDC on Base, the same token most fills pay out. `buildApprovalAndFillCalldata` already added it to the approval; nothing subtracted it from the balance. It is priced by `estimateGasFillPost`, which depends on the funding calls the leg loop produces and so cannot run before it, so `evaluateOrder` now checks affordability straight after the estimate: for a cross-chain order it requires the fee token's post-fill residue to cover the dispatch fee plus the paymaster reserve, and skips otherwise. Shrinking the fill is not an option — cross-chain orders cannot be partially filled.

Files: `src/funding/uniswapV4/UniswapV4FundingPlanner.ts`, `src/strategies/fx.ts`, `src/tests/funding/uniswapV4Removal.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-08-19 — Fill sizing reserves the paymaster's gas pull

A partial fill on Base reverted with `ERC20: transfer amount exceeds balance` after being short by 28,993 units — $0.029 on a $14,808.699383 fill. The fill was sized as wallet balance plus the Uniswap V4 credit and bid to the last unit, but the paymaster charges gas in the same USDC and pulls it via `transferFrom` during `validatePaymasterUserOp`, before the batch runs. The bid amount was bit-exact `balance + credited`, and the revert delta was bit-exact the prefund.

The only reserve the sizing loop knew about came from `FundingVenue.walletReserveForToken`, which is the vault's `minBalance`. `UniswapV4FundingPlanner` returns `0n` there by design (LP positions have no wallet float), and a filler configured with no venue at all never enters that loop, so both setups sized fills with zero headroom. A new `paymasterReserveForToken` in the paymaster module now contributes a reserve for the chain's USDC and USDT whenever a paymaster is configured, scaled by each token's own decimals, and the leg loop seeds `reserve` with it.

Files: `src/services/paymaster/index.ts`, `src/strategies/fx.ts`, `src/tests/services/paymaster-reserve.test.ts`, `docs/ai/{ChangeLog,Decisions,Flow}.md`.

## 2026-08-19 — Cross-lane orders skip cleanly instead of erroring "Shared cache is not initialized"

An operator running only Base saw `ERROR: [intent-filler]: Shared cache is not initialized` whenever an order destined to a chain their filler does not run scrolled past. The solver-selection cache is only ever populated for configured, non-watch-only chains (boot's `solverSelectionChains`), so any other destination fell through `handleNewOrder`'s cache check and was dropped with an error that reads like the filler is broken. The behavior (drop) was correct — there is no client, bundler or float for an unconfigured destination — the diagnosis was not, and it dates to #287.

`handleNewOrder` now checks the destination first: not a configured chain, or configured but watch-only → debug-level skip, cache untouched. The "Shared cache is not initialized" error survives for what it actually means now — a configured, filling destination with a genuinely absent entry, which is an initialization bug. Pinned by `order-destination.test.ts`: unconfigured / non-EVM / watch-only destinations never touch the cache, a filling destination proceeds to the allowlist, and the configured-but-uncached case still errors.

Files: `src/core/filler.ts`, `src/tests/core/order-destination.test.ts` (new).

## 2026-08-19 — The binary silences @polkadot/\* startup noise; the library still never touches the console

Every start of the bundled CLI printed a wall of `@polkadot/util has multiple versions` warnings, `REGISTRY: Unknown signed extensions` / `API/INIT: RPC methods not decorated` logger chatter, and Node's punycode deprecation. `src/bin/quiet.ts` — the entry's first import, since most of this fires during `@polkadot/*` module init — sets polkadot's official `POLKADOTJS_DISABLE_ESM_CJS_WARNING=1` (silencing the same-version dual-instantiation the single-file bundle necessarily produces), sets `process.noDeprecation`, and wraps `console.warn` with a narrow filter for the remaining known patterns. Only `console.warn` is touched (both noise sources write there); `console.error` is untouched, and real polkadot output — connection failures included — passes through, pinned by test.

Strictly bin-scoped: `dist/index.js`/`dist/sqlite.js` contain none of it (verified by grep), because a library consumer seeing duplicate-package warnings has a real dedupe to do in their own tree.

Two gotchas worth recording: `package.json`'s `"sideEffects": false` silently tree-shook the bare `import "./quiet"` out of the bundle — it is now an array listing `src/bin/quiet.ts` as the one side-effectful module — and vitest's own console interception makes the patched `console.warn` unobservable through stderr, which is why the test asserts through the exported `filteringWarn` factory instead.

The genuine version skew is fixed at the source in the same change, at the maintainer's call: the sdk's `@polkadot/api: "latest"` (and its `types`/`util`/`util-crypto`/`keyring` "latest" pins) became concrete `^16.5.6`/`^14.0.3` ranges, simplex's direct `@polkadot/util{,-crypto} ^13.5.6` moved to `^14.0.3` to match what api 16.x requires, and both packages' `resolutions` blocks — which pnpm warned were ineffective — are deleted rather than moved (they were doing nothing; nothing changed by removing them). Verified structurally, not just by silence: the rebuilt binary's bundle contains zero `13.5.9` occurrences where it previously carried both versions, `pnpm why @polkadot/util` resolves a single 14.0.3 in both package trees, and substrate key derivation (`balance-provider.test.ts`) passes on util-crypto 14.

Files: `src/bin/quiet.ts` (new), `src/bin/simplex.ts`, `package.json`, `sdk/packages/sdk/package.json`, `src/tests/cli/quiet.test.ts` (new).

## 2026-08-19 — Quorum client suspends rate-limited endpoints for 5 minutes

`QuorumPublicClient` previously re-queried a 429ing endpoint on every call — `isRateLimited` existed but only labelled diagnostics — which both wastes the call and deepens the provider's throttle. An endpoint whose failure is unambiguously a request-rate limit (`isSuspendableRateLimit` — stricter than the diagnostic `isRateLimited` label: `-32005` alone never benches, since Infura returns it for deterministic getLogs result caps, and the free-text match excludes URL-bearing metaMessages) is now suspended for `RATE_LIMIT_SUSPENSION_MS` (5 minutes) and dropped by `participants()` — from the query set and from the quorum bar both: each call's threshold is `quorumThreshold(endpoints actually queried)`, so the remaining endpoints keep serving reads while a provider throttles (first shipped with a fixed full-set threshold; reversed by the maintainer — a throttled endpoint answers nothing either way, and counting it only makes the scanner miss orders). With every endpoint benched, all are queried again. Suspension is recorded even from stragglers that settle after a call already decided, `suspended()` exposes the benched URLs, and QuorumError messages carry `responders: N/M queried (K/S suspended for rate limiting)` — the skipped count snapshotted at endpoint selection, not re-sampled at throw time (a long call can outlive a suspension window). `settleUntilQuorum` now takes `{ idx, task }` pairs so failures map to real endpoint indices (`getTransactionConfirmations` previously reported `unknown` URLs in failure detail).

Tested on the real-HTTP harness in `rate-limit-detection.test.ts` (genuine 429/500/-32005 responses through viem). The traffic-stops assertion runs through uncached `getLogs`, not `getBlockNumber` — viem caches eth_blockNumber for 4s, and an adversarial reviewer proved the naive version passed with suspension disabled entirely; the hardened suite fails 2 tests under that regression (verified by probe). Also covered: re-query after expiry via a `Date.now` spy, the small-set availability rule, threshold non-shrinkage at n=5, -32005 result caps not benching, and 500s staying stateless.

Files: `src/services/QuorumPublicClient.ts`, `src/tests/rate-limit-detection.test.ts`.

## 2026-08-18 — Signer return contracts spelled out

The docs showed `Promise<HexString>` twice and `Promise<Signature>` once without saying that the three mean different things: `signTypedData` returns a bare 65-byte `r ‖ s ‖ v` signature (`v` 27/28, the `eth_signTypedData_v4` form), `signAuthorization` returns split components with `yParity` strictly 0/1, and `signTransaction` returns the whole signed transaction as typed-envelope RLP — not a signature at all. An implementer had to reverse-engineer that from the adapters. The contracts now live in the `Signer` and `Signature` docblocks (what an IDE shows), a "Return exactly" table on both doc pages, and the sdk's `SigningAccount.signTypedData` docblock.

Files: `src/services/wallet/types.ts`, `sdk/packages/sdk/src/types/index.ts`, `docs/content/developers/sdk/{simplex,api/simplex}.mdx`.

## 2026-08-18 — Audit fixes: yParity guards, the signerless one-way door, and honest tests

A six-dimension adversarial audit of the signer branch confirmed 21 distinct issues; this change fixes all of them except the two deliberate semver calls (patch releases carrying breaking surface changes — restated in the PR and left to the maintainer).

Two code defects. `digestSigner` now validates the backend's signature once for every operation — most HSM docs return the legacy v (27/28), viem encodes any truthy `yParity` as parity 1, and EIP-7702 skips an invalid tuple without reverting, so an unguarded 27 meant "Delegation successful" in the logs and an undelegated solver on chain; `DelegationService.buildAuthorization` carries the same guard so hand-written signers are covered. And a signerless boot is now recorded (`FillerRuntime.signerless`) and enforced: boot's new per-chain watch-only exemption had made it possible to start an observer and later flip it into filling with the generated throwaway key via `chains.add`/`setWatchOnly(false)` — both now refuse, and `add` defaults new chains to watch-only. The setup API's gate also stops crashing on a signerless watch-only config (`TypeError` on an absent block) and instead mirrors run's rule.

Test integrity. `assertSignsForItsAddress`'s transaction leg asserted shape only — a signer signing the digest of a _different_ transaction passed — and now parses the signed bytes and recovers the signer, matching the gated integration tests. The refactor had migrated the code but not several fixtures: `UserOpSender`, `ContractInteractionService.rpc`, `pairs`, `fx.price-guard` and `fx.one-sided-lp` tests still stubbed `{ account: { address } }`, so the paths under test ran with the solver address `undefined` (hidden by `as unknown as Signer` casts); all swept to `{ address }`, and the UserOp tests now assert `op.sender`. The deleted `validateConfig` signer-requirement tests are replaced at the layer that owns the rule now: `boot-signer.test.ts` pins the boot rejection through a mock RPC, unit-tests `allChainsWatchOnly` (exported for the purpose), and pins the signerless guards. The shared `TYPED_DATA` fixture lists `EIP712Domain`, honouring the branch's own contract; one ungated assertion now recovers an authorization against the hand-built `keccak256(0x05 ‖ rlp(...))` preimage, so sign and verify no longer share viem's hasher; and a persist-roundtrip test asserts the `[simplex.signer]` block survives a dashboard rewrite — the regression the `FillerConfigFile` split exists to prevent.

Docs drift from the design's three iterations corrected across both packages' `docs/ai` (final interface shape, `signRawHash`'s removal, `validateConfig` does not read the signer block, `UserOpSender.buildSignedUserOp`), the README's quick-start now compiles, and the `digestSigner` docs state the split-signature contract and the new yParity rejection. The API reference documents the signerless one-way door.

Files: `src/services/wallet/account.ts`, `src/services/DelegationService.ts`, `src/core/boot.ts`, `src/simplex.ts`, `src/services/server/setup-api.ts`. Tests: `src/tests/core/boot-signer.test.ts` (new), `src/tests/wallet/signer.test.ts`, `src/tests/services/{UserOpSender,ContractInteractionService.rpc,SimplexPaymaster}.test.ts`, `src/tests/{pairs,ui-server}.test.ts`, `src/tests/strategies/{fx.price-guard,fx.one-sided-lp}.test.ts`. Docs: both packages' `docs/ai/*`, `README.md`, `docs/content/developers/sdk/{simplex,api/simplex}.mdx`.

## 2026-08-18 — `Simplex.start` takes a Signer interface instead of a signer config

Signing was reachable only through `config.simplex.signer`, a tagged union over the three backends simplex happens to ship, so a consumer embedding the library could not sign with anything else. `Simplex.start` now takes `signer: Signer` — the interface the solver already used internally — and the config block is the binary's TOML format only.

The interface carries no viem types at all, and names operations rather than digests: `address`, `mode`, `signTypedData`, `signAuthorization`, `signTransaction` — all required. `signRawHash` is gone, from both packages: once a signer must be able to sign an authorization and a transaction, nothing in either package calls it. `digestSigner({ address, mode, sign })` is the escape hatch for backends that only see 32 bytes; it does the EIP-712 hashing, the EIP-7702 authorization hashing and the transaction serialising, so that work stays in one tested place instead of in every integration. The `account: LocalAccount` field is gone — `accountFor(signer)` builds the viem account wallet clients need on our side of the boundary, so a consumer no longer has to match this package's viem version to satisfy the type (the `pnpm.overrides` warning that used to head `types.ts`). `TypedDataPayload`, `Signature`, `SignerTransaction` and `Eip7702Authorization` are ours, defined from the specs rather than from viem.

Two members went as unused or misplaced. `signMessage`, along with its declaration in the SDK's `SigningAccount` — bids are signed as EIP-712 UserOperations, and no path in either package ever invoked it. And the `chainId` argument on `signTypedData`, in both packages: EIP-712 carries it in `domain.chainId`, every payload simplex signs sets it, and MPCVault (the only backend that reads one, for its request envelope) now takes it from the payload instead of an argument that defaulted to mainnet.

`sendEip7702DelegationTransaction` was replaced by the general `signTransaction`: `DelegationService` sends every set-code transaction through the wallet client, and a backend that takes transactions rather than digests implements `signTransaction`. MPCVault does, including the set-code case its structured `evmSendCustom` request cannot express (no authorization-list field), which it serialises and raw-signs itself. It no longer builds a viem account at all.

Added `viemSigner(account)`, which derives the whole interface from any viem local account, and rebuilt `privateKeySigner` and `turnkeySigner` on top of it (Turnkey keeps its structured `signAuthorization`). Factories renamed to their public names: `createSimplexSigner` → `createSigner`, `create*SigningAccount` → `privateKeySigner` / `mpcVaultSigner` / `turnkeySigner`, `initializeSignerFromToml` → `signerFromToml`, and the `SigningAccount` type → `Signer`.

`FillerTomlConfig` (exported as `SimplexConfig`) no longer declares `simplex.signer` at all. The block moved to a new `FillerConfigFile extends FillerTomlConfig`, the on-disk shape the binary parses, which the CLI, the setup API, the TOML writer, the wizard and the dashboard's config type now use. The binary passes the parsed file straight to `Simplex.start`, extra key and all — `UiServer.persistConfig` regenerates the TOML from the running config object, so stripping the block there would erase the operator's signer from their file on the next curve edit.

Boot now rejects a missing signer unless every chain is watch-only, replacing the `[simplex.signer]` presence check inside `validateConfig` (a config-shape validator has no business requiring — or reading — an argument that is no longer part of the config; a present block is validated by its consumers: `signerFromToml`, the wizard's write step, the setup API's gate). `Simplex.start` throws when an object carries `simplex.signer` and no `signer` was passed — a runtime check, since the type no longer has the field — rather than silently ignoring it or resolving it. The CLI resolves the TOML block into a `Signer` itself and keeps its file-oriented error message.

Verified against a live MPCVault vault (10/10, chain 1). Every signature is checked by recovery, transactions included: the signed bytes are parsed back and asserted to carry the chain id, nonce, recipient and value we asked for, and to recover to the solver's address — the vault builds the transaction itself on the structured path, so "it returned hex" proves nothing. The set-code branch additionally asserts the authorization list survives into the signed transaction. `createMpcVaultAccount` is deleted — with the interface no longer carrying a viem account and `accountFor` building one from any `Signer`, nothing called it; an MPC-backed viem account is `accountFor(mpcVaultSigner(config))`.

Turnkey verified live too: `signTypedData`, `signAuthorization` and `signTransaction` all recover to the wallet address, with the transaction parsed back and checked field by field. The delegate-then-revoke case on Sepolia still needs a funded wallet. Adding those cases is what turned up the EIP-712 finding below — it is not an MPCVault quirk, it is how both remote-hashing backends behave.

Rig facts, since they cost a round of guessing: the vault uuid in the dev machine's `~/.zshrc` is rejected by the API, and the live one is the `vault-uuid` in `~/mpcvault/config.yml` (the value the running client-signer container serves); the vault accepts chain 1 only, so `MPCVAULT_TEST_CHAIN_ID=11155111` fails every chain-scoped call with `Invalid chain id`.

Files: `sdk/packages/sdk/src/types/index.ts` (SigningAccount narrowed), `src/services/wallet/types.ts`, `src/services/wallet/mpcvault.ts`, `src/services/wallet/signer.ts`, `src/services/wallet/index.ts`, `src/services/wallet/accounts/viem.ts` (new), `src/services/wallet/account.ts` (new: `accountFor`, `sdkSigningAccount`), `src/services/paymaster/{types,permit}.ts`, `src/services/paymaster/provider/{circle,simplex}.ts`, `src/services/rebalancers/{binance,usdt0}.ts`, `src/services/RebalancingService.ts`, `src/services/wallet/accounts/privatekey.ts`, `src/services/wallet/accounts/mpc.ts`, `src/services/wallet/accounts/turnkey.ts`, `src/services/DelegationService.ts`, `src/services/ChainClientManager.ts`, `src/core/boot.ts`, `src/simplex.ts`, `src/index.ts`, `src/bin/simplex.ts`, `src/config/filler-toml.ts`, `src/services/server/{UiServer,setup-api}.ts`, `src/cli/init/{index,state,emit-toml}.ts`, `src/cli/init/steps/write.ts`, `ui/src/types.ts`, plus type-only renames across `src/core/filler.ts`, `src/services/{ContractInteractionService,PaymasterKeeperService,UserOpSender}.ts`, `src/strategies/fx.ts`. Tests: `src/tests/wallet/signer.test.ts` (new), `src/tests/wallet/turnkey.test.ts`, `src/tests/wallet/mpcvault.integration.test.ts`, `src/tests/cli/filler-toml-validate.test.ts`. Docs: `README.md`, `docs/content/developers/sdk/simplex.mdx`, `docs/content/developers/sdk/api/simplex.mdx`, `docs/content/developers/evm/intent-gateway/simplex.mdx`.
