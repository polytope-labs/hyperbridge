# 2026-10-16 — Vault batches refuse undelegated wallets, redeems respect `maxRedeem`, bids bound their Permit2 deadline, and Binance deposits are matched against the coin's contract

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

## A bid's Permit2 deadline matches its own validity

`PERMIT2_DEADLINE_SECONDS` (1 hour) is sized for ops with no validity of their own, but a bid's
`validUntil` is `bidValiditySeconds + BID_DISCOVERY_PAD_SECONDS` (about 330s by default). The
paymaster reports the permit deadline as the op's `validUntil`, so a bid selected late still
validated and then reverted `Expired` in `fillOrder`, with the solver paying the gas.

`PaymasterOptions.permitDeadlineSeconds` threads an optional window through
`buildPaymasterAndData` into `buildPermit2Mode` (absent, the hour default is unchanged).
`prepareBidUserOp` passes `bidValiditySeconds + BID_DISCOVERY_PAD_SECONDS + BID_PERMIT_SKEW_SECONDS`
(120; the skew covers clock skew, inclusion latency and the buffer a bundler applies when an op's
`validUntil` is too close to now). The permit deadline is wall-clock, while the bid's on-chain
`validUntil` is block-based and can run longer in wall-clock time when blocks are slow, so whichever
is earlier binds. Sends, sweeps and delegation keep the hour. The one-time `buildPermitMode`
bootstrap is untouched: its EIP-2612 deadline is `maxUint256` because a paymaster cannot read
`block.timestamp` under 4337 validation rules, and it has no window to match.

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

Files: `src/funding/vault/VaultFundingPlanner.ts`, `src/config/abis/Erc4626.ts`,
`src/services/TokenSender.ts`, `src/services/paymaster/{types,index}.ts`,
`src/services/paymaster/provider/simplex.ts`, `src/services/ContractInteractionService.ts`,
`src/services/rebalancers/binance.ts`, `src/core/boot.ts`.
Tests: `src/tests/funding/vault.test.ts`, `src/tests/token-sender.test.ts`,
`src/tests/services/{SimplexPaymaster,bid-budget}.test.ts`,
`src/tests/rebalancers/binance.preflight.test.ts`.
