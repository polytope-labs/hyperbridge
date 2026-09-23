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

## Limit orders are stated as a pair, a side, a size and a rate

The create form asked for two amounts and showed the rate they implied. It now asks for what the
operator is actually choosing:

- **Pair** — only the books the orderbook lists (`GET /api/orderbook/books`, backed by
  `LimitOrderService.books()`). `resolveBook` refuses anything else, and a book's own spelling of
  its symbols is what a request must carry.
- **Buy / Sell** — sub-tabs over the book's base, the selected one green for buying and red for
  selling. Buy takes the base in and pays the quote out (a `BID`); Sell takes the quote in and
  pays the base out (an `ASK`).
- **Amount** in the book's base, and **rate** in quote per base, the way the book is quoted.

`requestFrom` derives the two amounts: buying takes `amount` base in and pays `amount × rate` quote
out, selling pays `amount` base out and takes `amount × rate` quote in. The arithmetic is exact at
1e18 rather than in floating point, and rounds so the posted price is never better for the taker
than the rate stated — a bid pays out no more quote, an ask takes in no less. A size that rounds
away to nothing is refused rather than posted. The form shows the derived amounts before posting.
