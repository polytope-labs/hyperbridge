# 2026-10-13 — `[[pairs]]` removed, and balances follow the orderbook's books

## Balances are read for every book token

The dashboard's balances used to cover USDC, USDT and the `token1` of each cross-asset
`[[pairs]]` entry. A token only reached the balances if a pair named it. Configs written before
the orderbook listed `USDC-BRIDGE` and `USDT-BRIDGE` showed no BRIDGE balance. The new limit
order form then picked no chain under "Fills on", so the order could not be posted.

`BalanceProvider` now takes a `tokens` function. Boot passes one that reads the orderbook's books
through `LimitOrderService.books()` and resolves both sides of every book on each configured
chain through the asset registry. It is called on each refresh, so a book the orderbook adds
shows up without a restart. `books()` is cached and falls back to its last read. If the books
have never been read, the refresh logs a warning and reads USDC and USDT only.

## `[[pairs]]` is gone

Pairs never gated matching or carried prices, and balances no longer depend on them. Nothing
read them any more, so they are removed:

- Config: `FillerTomlConfig.pairs` and its validation. An old config that still has `[[pairs]]`
  boots. Boot logs `[[pairs]] is no longer read and can be deleted from the config`. The section
  is dropped the next time the dashboard or `simplex init` rewrites the file.
- Engine: `FXFiller` takes no pair list. `TradingPair`, `addPair` and `removePair` are gone.
- Library: `simplex.pairs` (`PairController`), `PairView`, `PairConfig`, `validatePairConfigs`,
  `pickAnchorStable` and `SimplexStatus.pairs` are gone. `src/config/pairs.ts` is deleted.
- HTTP: `GET`/`POST /api/strategies` and `DELETE /api/strategies/:index` are gone, and so are
  `strategies` and `strategyTypes` on `GET /api/status`.
- Browser wizard: writes no `[[pairs]]`. `tradeableBooks` replaces `orderbookPairs`, and still
  refuses a chain selection that carries none of the orderbook's books.
- CLI wizard: `assembleConfig` strips a carried `[[pairs]]`, or the round-trip gate would fail
  now that the emitter no longer writes it. `migrateLegacyConfig` drops `[[strategies]]` instead
  of turning it into pairs, still moving per-strategy confirmation policies top-level.
