# 2026-10-16 — Vault batches refuse undelegated wallets, redeems respect `maxRedeem`, bids bound their Permit2 deadline on the chain's clock, and Binance deposits are matched against the coin's contract

## A vault batch is refused on a wallet that is not 7702-delegated

`VaultFundingPlanner.submitBatch` falls back to a native `walletClient.sendTransaction` when no
sponsored UserOp is available: an ERC-7821 `executeBatch` sent to the solver account. Before the EOA
is delegated that address has no code, so the tx succeeds and the batch inside it never runs —
`sweepChain` still fired `onTx` and recorded a sweep that did not happen, and the timer retried it
every interval, burning gas each time.

The planner now takes an optional `Pick<DelegationService, "isDelegated">` (boot passes a
`DelegationService`; it is only read here). Before the native send it checks `isDelegated(chain)` and
throws `The wallet is not 7702-delegated on <chain> (or its delegation could not be read); a vault
batch sent to it would do nothing` when it is false. The throw is before `onTx`, so there is no
phantom movement; `sweepExcessToVault` rejects and the periodic timer's own `.catch` logs it, as it
already did for any other batch failure. `redeemAll` also uses `submitBatch` and catches per chain. A
sponsored op is unaffected: the check sits after the sponsored attempt.

## Redeems are capped at the vault's `maxRedeem`

`redeemChain` redeemed `balanceOf(solver)` from every vault in one per-chain batch. A vault whose
`maxRedeem` is below the balance (a liquidity-capped stataToken) reverts the whole batch, taking the
other vaults with it. It now redeems `min(balanceOf, maxRedeem)`, skips a vault whose cap is zero,
and warns with both numbers when shares are left behind. `maxRedeem(address)` was added to
`ERC4626_ABI`.

`TokenSender.send` reads `maxRedeem(this.solver)` on the vault-share branch and throws before
submitting anything when the requested shares exceed it, naming the amount redeemable now.

## A bid's Permit2 deadline is bounded to its own window, on the chain's clock

`PERMIT2_DEADLINE_SECONDS` (1 hour) is sized for ops with no validity of their own, but a bid's
`validUntil` is `bidValiditySeconds + BID_DISCOVERY_PAD_SECONDS` (about 330s by default). The
paymaster reports the permit deadline as the op's `validUntil`, so a bid selected late still
validated and then reverted in `fillOrder`, with the solver paying the gas.

`PaymasterOptions.permitDeadline` threads an optional **absolute** deadline (unix seconds) through
`buildPaymasterAndData` into `buildPermit2Mode`; absent, it stays the host clock plus the hour.
`prepareBidUserOp` reads the destination head once with `getBlock`, reuses its number for
`bidValidUntilBlock`, and passes `head.timestamp + bidValiditySeconds + BID_DISCOVERY_PAD_SECONDS +
BID_PERMIT_SKEW_SECONDS` (120; it covers the head's age at signing, inclusion latency and the buffer
a bundler applies when an op's `validUntil` is too close to now). The deadline is anchored to the
chain because the EntryPoint judges it there: a window added to the host clock lost whatever the
host was behind by, and with 6–7 minutes of skew every sponsored bid failed validation. Sends,
sweeps and delegation keep the hour from the host clock. The one-time `buildPermitMode` bootstrap is
untouched: its EIP-2612 deadline is `maxUint256` because a paymaster cannot read `block.timestamp`
under 4337 validation rules.

What this guarantees is narrower than "a stale bid is dropped for free". An op more than about 120s
past the bid window is rejected at validation. An op landing inside that tail, after the bid's
block-based `validUntil` but before the permit deadline, still reverts `FillExpired` at the
solver's cost. Closing it needs `SolverAccount` to return a block-number `validUntil` (contract
follow-up).

## A capped redeem tells the operator the rest is theirs

`redeemAll` runs only at shutdown or on `POST /api/vault/redeem`, so the shares a `maxRedeem` cap
leaves behind are not picked up by a later pass. The warning now says the operator has to redeem the
rest.

## A Binance deposit is checked against the coin's contract for that network

The CEX leg fetched a deposit address and then `transferOnChain` looked the token up from the SDK
chain config. Nothing compared that to the `contractAddress` Binance reports for the coin/network,
so a drift (a redeploy, a stale config entry, the wrong network) would strand the deposit as an
unrelated token.

`sendViaCex` now resolves the token `transferOnChain` will send (a shared `tokenAddressForCoin`) and
requires Binance's `contractAddress` to be non-empty and to match it, case-insensitively and
tolerating a missing `0x`. A deposit address carrying a non-empty `tag`/memo is refused too, since
an ERC-20 transfer cannot carry one. Both fail before any on-chain transfer. The existing
`depositEnable`/`withdrawEnable` checks stay; the wider withdrawal-precondition work (audit high
c02) is out of scope here.
