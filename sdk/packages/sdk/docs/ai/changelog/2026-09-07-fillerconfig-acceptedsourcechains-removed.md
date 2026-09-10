# 2026-09-07 — `FillerConfig.acceptedSourceChains` removed

The optional field is gone from `FillerConfig`: simplex now derives a bid's accepted sources from its configured chains and watch-only flags at bid time, so nothing reads it. `encodePhantomBidDeclaration` and the decoder are unchanged.
Files: `src/types/index.ts`.
