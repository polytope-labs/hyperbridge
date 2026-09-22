# 2026-09-16 — The posted op is pinned to its wire format

A posting is a signed message to a service that answers a misplaced field with a rejection code and
nothing else, so `posted-userop.test.ts` decodes the op simplex builds the way the orderbook decodes
it — `decodeUserOpScale`, `decodeERC7821ExecuteBatch`, `decodeFillOrder`, `decodePhantomBidDeclaration`
— and pins every field it reads where it expects to find it:

- one leg quoted, the output token approved to the gateway first;
- `options.outputs[0]` what the operator pays and `options.inputs[0]` the whole input they take for
  it, which is the rate, since the order's own output amount is zero as the orderbook requires;
- `validUntil` carrying the TTL in seconds from receipt;
- the declaration in `paymasterAndData`, and a build refused outright when the list is empty, since
  the encoder itself would take it and the orderbook would answer `EMPTY_DECLARATION`;
- the commitment prefixing the signature, the nonce key bound to it as `SolverAccount` reads it, and
  the solver recovering from the bare userOpHash.

There is one `fillOrder` shape — the one the gateways simplex fills on speak, and the only one the
SDK encodes or decodes — so there is no version to pick and nothing to decode two ways.

`RejectionCode` gains `UNSUPPORTED_SOURCE_CHAIN`, which the orderbook's `schema.graphql` has and we
did not. It is returned when a declared source chain does not register the order's input symbol on
the server.

A posting is also taken through a running orderbook end to end in CI, which is what covers the
verdicts the server alone can give.
