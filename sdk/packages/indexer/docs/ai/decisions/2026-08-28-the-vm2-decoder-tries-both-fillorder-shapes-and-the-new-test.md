# 2026-08-28 — The VM2 decoder tries both `fillOrder` shapes, and the new test avoids the SDK root import

Alternatives to trying both shapes:

- **Track the gateway's version and pick one.** That is what `getFillOptionsVersion` does for *encoding*, where
  you must choose. Decoding has no such constraint: the selectors differ, so attempting both is unambiguous and
  needs no chain state, no RPC read, and no cache — all of which the SubQuery sandbox makes awkward.
- **Decode by selector lookup.** Equivalent in effect, more code, and it would duplicate the selector constants
  that the ABIs already encode.

Trying v2 then v1 mirrors `decodeFillOrder` exactly, which is the point: the two implementations of this decode
diverged once already, and keeping them structurally identical is what makes the next divergence visible.

The regression test lives in its own file, `phantom-decode.fill.test.ts`, importing only the `intents-helpers`
sub-path. The sibling `phantom-decode.test.ts` also imports `@hyperbridge/sdk` root for `CryptoUtils`, which is
not available on the sub-path. Splitting keeps the new test on exactly the entry point the indexer uses at
runtime, so it exercises the same module graph the sandbox loads.

The jest `transformIgnorePatterns` addition names the ESM-only packages explicitly rather than transforming all of
`node_modules`. The broad form is slower and pulls unrelated packages through ts-jest; the explicit list fails
loudly (an unparsed `export`) if the SDK's dependency graph grows another ESM-only package, which is the right
failure mode — silence here is what let the tests stop running unnoticed.
