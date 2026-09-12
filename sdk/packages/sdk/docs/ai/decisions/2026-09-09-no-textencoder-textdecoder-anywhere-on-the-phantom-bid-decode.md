# 2026-09-09 — No TextEncoder/TextDecoder anywhere on the phantom bid decode path

Chosen: the declaration codec encodes and decodes its UTF-8 chain ids by hand, in the same file, with no
dependency on `TextEncoder`, `TextDecoder`, or the `@polkadot/util` wrappers over them (`stringToU8a`,
`u8aToString`). The rule this sets: nothing `aggregatePhantomBids` runs per bid may touch either API.

Why. The indexer runs the aggregation inside SubQuery's vm2 sandbox, and that sandbox exposes neither API as a
global; `@polkadot/x-textdecoder` then falls back to Node's `util.TextDecoder`, whose argument check
(`ArrayBuffer.isView`) does not recognise a `Uint8Array` created inside the sandbox, because it arrives as a
proxy. Injecting the host's `TextDecoder` into the sandbox does not help — the failure is the realm of the
bytes, not the presence of the API — so this cannot be fixed from the sandbox configuration side. The same
class of failure is why `extractFillDataVm2` and `recoverBidSignerVm2` exist in the indexer, and this one hid
for a month because no live bid carried a source chain until simplex started declaring every configured
chain (#1216): the empty and positions-only declarations never reach the string decode.

Alternatives considered:

- Add `utf8Decode` to the indexer's VM2-safe injection set (a `decodeDeclaration` parameter on the
  aggregation). It would work, but it puts a second copy of a security-relevant parser downstream and leaves
  the SDK's own decoder silently unusable in the one environment it is mainly run in. A pure decoder in the
  SDK fixes every consumer at once and needs nothing injected.
- Decode chain ids with `String.fromCharCode` over the bytes, since state machine ids are ASCII. It would pass
  today and misread the first non-ASCII id ever declared, byte by byte, into a name that matches no chain —
  a silent route loss rather than a rejected declaration. Real UTF-8, with the same rejections as
  `TextDecoder`, keeps the wire format exactly what it was.
- Catch the decode error and treat the bid as declaring nothing. That reads a solver's explicit source list
  as "any chain" and hands it routes it never offered.

The test that guards this bundles the shipped `intents-helpers` into one file with esbuild (as `subql build`
does with webpack) and runs the decoder inside a NodeVM configured like `@subql/node-core`'s Sandbox, because
vm2 cannot load the pnpm module graph piecemeal (ESM-only packages, dynamic imports at load time). It lives in
the indexer, which has vm2 and esbuild through `@subql/node-core` and `@subql/cli`.
