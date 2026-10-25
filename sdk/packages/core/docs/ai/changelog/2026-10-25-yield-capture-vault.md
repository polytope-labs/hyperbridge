# 2026-10-25 — `YieldCaptureVault`: 1:1 ERC-4626 vault that returns principal and sends all yield to its owner

`YieldCaptureVault` wraps an inner ERC-4626 vault (Aave's stataUSDC in production). Depositors get their
principal back and all of the yield goes to the owner, paid in the asset by `harvest()` (owner only).

Shares are 1:1 with the asset and deposits always mint at par. `totalSupply()` is what depositors are owed:
`unharvestedYield()` is the inner vault's value above it, `depositorShortfall()` the value below it, and
`totalAssets()` is `min(innerVaultValue(), totalSupply())`. Donations go to the owner.

`harvest()` redeems the yield rounded down, so depositors are untouched. It pays whoever is the owner when
it is called.

Unharvested yield, and the inner vault's rounding dust, is a first-loss buffer. Beyond it, redemptions are
pro-rata, and gains refill depositors before the owner earns again.

Deposits and mints close while the shortfall exceeds `MAX_SHORTFALL_FOR_DEPOSITS` (1,000 wei) and reopen
once yield refills it; exits through this vault are never blocked by the shortfall.

The owner can `pause()` deposits, withdrawals and share transfers; `maxDeposit`, `maxMint`, `maxWithdraw`
and `maxRedeem` return 0 meanwhile. `harvest()` and `execute` still work while paused, but share transfers
don't. A paused vault freezes exits, so the owner should be a multisig. `renounceOwnership` always reverts: a
renounced owner could never unpause or collect yield.

The owner can `execute` ERC-7821 call batches as the vault, e.g. to claim Aave incentives, which only the
stata holder can do. A batch is refused if it calls the asset, calls any `transferFrom`, approves the
inner-vault shares, calls the vault itself for anything but `transfer`, or leaves the vault with fewer
inner-vault shares. So it cannot take depositors' principal or spend the approvals the vault asks for (the asset
and its own shares); yield still leaves only through `harvest()`. Allowances users grant the vault for any other
token are not protected, so they shouldn't grant any.

Stray balances are recovered with `execute`: another token with `transfer(owner, amount)`, vault shares sent to
the vault with a self `transfer`, and donated USDC by depositing it into the inner vault, then `harvest()`.

Aave pauses: a paused or inactive reserve zeroes the inner vault's max functions and `harvest()` returns 0;
a frozen one blocks only deposits; stataUSDC's own pause reverts everything until lifted. The inner vault and
the asset are trusted (no fees, no rebasing, no reentrancy).

Files: `contracts/vaults/YieldCaptureVault.sol`, `test/YieldCaptureVault.t.sol`,
`test/YieldCaptureVaultBaseFork.t.sol`, `package.json`, `docs/ai/changelog/2026-10-25-yield-capture-vault.md`.
