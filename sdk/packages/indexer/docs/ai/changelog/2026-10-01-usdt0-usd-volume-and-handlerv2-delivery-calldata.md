# 2026-10-01 — USDT0 counts toward USD volume, and HandlerV2 deliveries record transfer volume

Two gaps left USD volume frozen while the indexers themselves were at chain tip.

## USDT0 is priced at $1

Polygon's USDT reports the symbol `USDT0`, and Arbitrum's and Optimism's report `USD₮0`. Neither was in
`STABLE_SYMBOLS`, and the orderbook quotes no book for them, so every such order logged `No USD price for USDT0 …`
and added nothing to `CumulativeIntentGatewayVolumeUSD`.

`STABLE_SYMBOL_ALIASES` (`src/services/orderbookRates.service.ts`) lists both, and `fetchTokenUsdPrice` answers $1
for a stable or an alias through `isStableSymbol`. Aliases are not quote currencies: the `bestRate` query still
asks only about `USDC` and `USDT`. Everything that prices by symbol picks this up — the gateway rollup, an order's
`inputUSD`, filler volume and points, and `TokenPriceService`, so `Transfer.USDT0` volume as well.

This applies from deploy onwards. USD values already written for USDT0 orders are not recomputed; the raw amounts
in `IntentGatewayTokenVolume` are intact for a backfill outside the indexer.

## Delivery calldata is read with the `HandlerV2` ABI

`PostRequestHandled`, `GetRequestHandled`, `PostRequestTimeoutHandled` and `GetRequestTimeoutHandled` each decode
the delivering transaction's calldata to find the modules its messages concern, then record `Transfer.<symbol>`
volume for the transaction's ERC-20 transfers and `Contract.<address>` volume for transfers touching one of those
modules. The bundled handler ABI no longer matched what relayers send:

- deliveries are wrapped in `batchCall(bytes[])` (`0x68be3cf2`), typically `handleConsensus` followed by the
  message call;
- a leaf is `(request, index)` with no `kIndex`, and a GET request's `from` is `bytes`, which gives
  `handlePostRequests` (`0x698b1267`), `handleGetResponses` (`0xef7baa0b`) and `handleGetRequestTimeouts`
  (`0x4130514b`) new selectors.

The decode threw `no matching function` inside the `try` that also holds the transfer loop, so neither volume was
recorded.

`src/configs/abis/HandlerV2.abi.json` is now the only handler ABI, and the `handlerV2` key replaces the old
handler key in `config-*.json`. `getHandlerMessageModules(calldata, fn)` (`src/utils/handler.helpers.ts`) replaces
the four inline decodes: it unwraps `batchCall`, nested ones included, and collects the modules from every call
named `fn`. It never throws: calldata it cannot read is logged with its selector and yields no modules, which
costs only the `Contract.*` attribution — `Transfer.*` volume is recorded regardless.

`GetRequestHandled` also read the answered request as `response.get`, a field the handler ABI does not have. It
now reads `response.request`.

Deliveries made through the previous handler's signatures no longer decode, so a resync records their
`Transfer.*` volume without `Contract.*` attribution. `Transfer.*` and `Contract.*` volume for deliveries already
indexed while the decode was failing is not backfilled; those events are not replayed.

The parked transaction handler for direct `handlePostRequests` calls, and its commented-out data source in
`scripts/templates/evm-chain.yaml.hbs`, are removed: batched deliveries never match a `handlePostRequests`
function filter.
