# 2026-09-16 — The posted op is checked against the orderbook's own vectors

`src/tests/fixtures/orderbook-userops.json` is a copy of `fixtures/userops.json` from
polytope-labs/hyperfx-orderbook: 14 signed `fillOrder` UserOps, each one's description naming the
verdict the orderbook gives it. Refresh it with

```
gh api repos/polytope-labs/hyperfx-orderbook/contents/fixtures/userops.json -q .content | base64 -d
```

`posted-userop.test.ts` reads an op the way the orderbook does, through the SDK's `decodeUserOpScale`,
`decodeERC7821ExecuteBatch`, `decodeFillOrder` and `decodePhantomBidDeclaration`, and returns the
refusal it would earn: `NO_FILL_ORDER`, `MISSING_VALID_UNTIL`, `NOT_PHANTOM`, `UNSUPPORTED_SHAPE`,
`TTL_TOO_SHORT`, `MISSING_DECLARATION`, `EMPTY_DECLARATION`, `MIN_ORDER_SIZE`, `BAD_NONCE_BINDING`,
`BAD_SIGNATURE`, or nothing. All 14 vectors get the verdict they say they should, which is what makes
the function worth pointing at our own op. The codes that depend on the server's own config or state, such as
`UNSUPPORTED_CHAIN`, `UNSUPPORTED_SOURCE_CHAIN` and `REPLAYED`, are not decidable from an op and are
not checked.

`RejectionCode` gains `UNSUPPORTED_SOURCE_CHAIN`, which the orderbook's `schema.graphql` has and we
did not. It is returned when a declared source chain does not register the order's input symbol on
the server. The vectors describe the under-a-tier case as `BELOW_MIN_TIER`, but that name appears
only in their prose: the wire enum calls it `MIN_ORDER_SIZE`.

The op `ContractInteractionService.prepareLimitOrderUserOp` builds then goes through the same
function and is accepted. The test builds it over the real signing path: an `IntentGateway` whose
fee token read is stubbed, because that is the only part of a posting that wants a node.

A v1 payload and a v2 payload asking for zero seconds both decode to a `validUntil` of zero, and they
earn different refusals, so the v2 selector is read off the vectors rather than written out.
