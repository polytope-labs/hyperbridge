# 2026-09-23 — Vault holdings back a limit order

`assertWalletCanPay` refused any limit order whose payout exceeded the solver's **wallet** balance
on the fill chain. A fill does not pay from the wallet alone: it spends the wallet down to its
reserve and withdraws the shortfall from the configured ERC-4626 vaults in the same batch
(`FXFiller` → `FundingVenue.planWithdrawalForToken`). An operator who swept inventory into a vault
was refused orders their own filler would have filled:

```
The wallet holds 108.58 USDC on EVM-97, which cannot pay out 186.33
```

`LimitOrderService` now takes the vault venue (`VaultHoldings`, satisfied by `VaultFundingPlanner`)
and counts what those vaults hold of the payout token on the fill chain, alongside the wallet.

- Vault positions are counted the way wallet balances already are: the whole position, not what is
  left after reservations for fills in flight, since every order on a book rests on one balance.
- Only vaults on the fill chain holding the payout token count.
- A snapshot that cannot be read counts as zero, so an unreachable RPC refuses an order the filler
  might have managed rather than accepting one it could not pay.
- The refusal now says where the money is: `holds 1000 in the wallet and 200 in vaults CNGN on
  EVM-8453, which cannot pay out 1500`.

An operator running no vaults is unaffected: `vaultBalances` is undefined and the check reads the
wallet alone.

## The operator UI drops the markets section

The overview no longer carries the `Active markets` metric or the markets panel that listed each
strategy's pair, its curve editor and the create-market form. `OperatorMarkets`,
`StrategyMarketEditor`, `CreateMarketForm`, `useStrategyEditor` and `useCreateMarket` are gone, as
is the `/api/strategies` read that fed them; `marketSymbols` stays, because the limit order form
lists the same symbols. The server's strategy endpoints are untouched.
