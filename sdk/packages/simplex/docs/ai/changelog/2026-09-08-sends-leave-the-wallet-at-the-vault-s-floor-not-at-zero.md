# 2026-09-08 — Sends leave the wallet at the vault's floor, not at zero

A send on Base reverted with `ERC20: transfer amount exceeds balance` inside a UserOp that the
outer transaction reported as successful (tx `0x737bdede…`, block 51039630). The batch was
`withdraw(3990.010227)` then `transfer(4000)`, against a wallet holding 9.989773 USDC — sized to
leave exactly zero. The SimplexPaymaster then debited 0.031564 USDC of gas from that same wallet
during validation, before the batch ran, so the transfer was short by exactly that and the whole
batch rolled back. The vault position was untouched (share balance identical either side of the
block), the paymaster still kept ~0.0097 USDC net, and a retry would fail the same way from a
slightly smaller wallet.

`vaultWithdrawalCall` now takes the token's decimals and leaves headroom on top of the shortfall:
the larger of the matched vault's `minBalance` and `paymasterReserveForToken`, the helper the fill
path already uses for exactly this hazard — its own doc comment says the paymaster "pulls it from
the same wallet during validatePaymasterUserOp — before the UserOp's callData runs". TokenSender
never imported it. The reserve applies only when `userOpSender.canSponsor(chain)` is true, so an
unsponsored chain and the native-tx fallback are unaffected, and it covers the case a floor alone
cannot reach: a wallet that already covers the transfer but would be left with nothing for
validation to take. That send skipped the vault branch entirely and reverted the same way. A vault that cannot cover shortfall-plus-floor still funds the send with the bare
shortfall: a floor is a preference, not a reason to refuse a transfer the operator asked for. A
send the wallet already covers is unchanged — it does not start pulling from the vault to top
itself up. Withdraw-only vaults declare no `minBalance` and behave exactly as before, which means
they keep the original failure mode; the treasury docs now say so.

Files: src/services/TokenSender.ts, src/tests/token-sender.test.ts (9 tests, 4 new — the first
replays the numbers from the reverted transaction), docs/content/developers/evm/simplex/treasury.mdx.
