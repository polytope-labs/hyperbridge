# 2026-10-13 — Balances follow the orderbook's books, and `[[pairs]]` is optional

## Balances are read for every book token

The dashboard's balances used to cover USDC, USDT and the `token1` of each cross-asset
`[[pairs]]` entry. A token only reached the balances if a pair named it as `token1`. Configs
written before the orderbook listed `USDC-BRIDGE` and `USDT-BRIDGE` showed no BRIDGE balance. The
new limit order form then picked no chain under "Fills on", so the order could not be posted.

`BalanceProvider` now takes a `tokens` function in place of the `token1` map. Boot passes one that
reads the orderbook's books through `LimitOrderService.books()` and resolves both sides of every
book on each configured chain through the asset registry. It is called on each refresh, so a book
the orderbook adds shows up without a restart. `books()` is cached and falls back to its last read.
If the books have never been read, the refresh logs a warning and reads USDC and USDT only.

`FillerRuntime.balanceTokens` is gone, and `PairController.add` no longer touches balances.

## `[[pairs]]` is optional

A config with no `[[pairs]]` used to fail at boot with `FXFiller requires at least one trading
pair`. The engine stores the pair list but never reads it, so the check is gone:

- `FXFiller` accepts an empty pair list, and `removePair` can remove the last market.
- `DELETE /api/strategies/:index` can remove the last market.
- `validatePairConfigs` accepts an empty array.
- The browser setup wizard no longer writes `[[pairs]]`. It still refuses a chain selection that
  carries none of the orderbook's books.
