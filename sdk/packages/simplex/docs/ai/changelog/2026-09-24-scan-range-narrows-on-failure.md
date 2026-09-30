# 2026-09-24 — The scan range narrows when a read fails

A failed log read now halves the range the next pass asks for, down to a single block, and a good
read doubles it back toward the full 1000. The cursor still never moves past a range it failed to
read, so no block is skipped either way.

## The chain that stayed stuck

Free endpoints cap `eth_getLogs` ranges well below the full span. Probed on Arbitrum mainnet:

| endpoint | 100 blocks | 250 blocks |
|---|---|---|
| publicnode | refused (archive requires a token) | refused |
| nodies | refused (rate limited) | refused |
| drpc | ok | refused |
| tenderly, fastnode, pocket, arb1 | ok | ok |

A filler's first pass after boot reads everything since the head it took at startup. At four
Arbitrum blocks a second, a thirty-second boot makes that about 117 blocks. Three of seven endpoints
refused it, so a 5-of-7 quorum could not form. At a fixed span the next pass asked for the same
range plus whatever had been produced since, so the range only grew, to the full 1001 blocks, and
the capped endpoints never rejoined. The chain stayed stuck, and restarting reproduced it.

## The rule

`ChainScanner.span` is how far past the cursor a pass reads, as `toBlock - fromBlock`. A failed read
sets it to half the range that failed, not half the span: near the head a pass reads fewer blocks
than the span allows, so halving the span would not shrink the next read. A good read doubles it,
capped at `MAX_BLOCK_RANGE`.

A range the RPC has not indexed yet (`isBlockRangeError`) does not narrow. That failure is about
where the range ends, not how wide it is.

## What it costs

When the caps fall between two doublings, a catch-up alternates: the span that succeeds doubles past
the cap, that read fails, and it halves back. That is one failed read in two while catching up, and
none at the head, where a pass reads only the few blocks produced since the last one whatever the
span. A single growth rule was preferred over tracking the ceiling that last failed.
