# Reads race every endpoint, and startup waits less

A solver with five chains on the default public endpoints took over three minutes to start. The
ordered `fallback` transport retried each endpoint three times with 1s, 2s and 4s of backoff before
moving on, so every endpoint that was down ahead of a live one added seven seconds to every read.
Startup then paid that one chain at a time.

## Reads race

`ChainClientManager.raceTransport` is the public client's transport for a chain with more than one
endpoint. Every read is sent to all of the chain's endpoints at once, and the first answer is used.

- A result is an answer. So is a revert, by the same `shouldThrow` rule viem's `fallback` uses to
  stop failing over.
- An endpoint that is unreachable, times out, rate-limits or refuses the method is skipped. The read
  fails only when every endpoint has failed, and the transport then retries the whole race with
  viem's defaults.
- No endpoint retries on its own. The other endpoints are the retry.

This applies to every read through `ChainClientManager.getPublicClient`, during normal operation as
well as at startup. Consequences to keep in mind:

- Every listed endpoint receives every read outside the quorum.
- The first-listed endpoint is not the one that answers. Two consecutive reads can come from nodes at
  different heights.
- The wallet client keeps the ordered `fallback`, so a transaction goes to one endpoint and the nonce
  read behind it reaches the same node.
- `QuorumPublicClient` is unchanged. It is still the client that cross-checks answers.

A chain with a single endpoint keeps plain `http` with its retries.

## Chain ids are probed once, for all chains together

Every endpoint is asked for its chain id at startup, and `fetchChainId` gives each one ten seconds.

- `Simplex.start` runs that probe once and hands the result to the scanner it builds and to
  `bootFiller`, through `BootOptions.resolvedChains`. `bootFiller` probes on its own when it is not
  handed the chains.
- `OrderScanner.create` probes all its entries that omit a `chainId` together. The adds stay in
  order, so two entries that resolve to the same chain are still refused.

## Startup stops waiting for a chain's slow endpoints

With `tolerateUnreachable`, which startup sets, `resolveChainConfigs` stops waiting for a chain's
other endpoints `STRAGGLER_GRACE_MS` (one second) after the first one reports its chain id. An
endpoint still pending then is treated like one that could not answer: it is logged and kept.

The endpoints that answered in time must still agree, and a disagreement still refuses the config.
A slower endpoint is checked when its answer arrives. If it reports another chain, an error is
logged that names it, but the solver is already running by then. That is the cost: a wrong-chain
endpoint is only refused at startup if it answers within a second of the fastest one.

Without `tolerateUnreachable`, which runtime endpoint edits use, every endpoint is waited for.

## Startup steps that do not depend on each other run together

- `IntentFiller.solverSelectionChains`, `DelegationService.setupDelegationOnChains` and the
  EntryPoint deposit top-ups run their chains concurrently. Delegation state is keyed by wallet and
  chain, so the chains do not share any.
- `IntentFiller.initialize` connects to Hyperbridge while it sets the chains up.
- `bootFiller` starts the limit-order lifecycle while the filler initializes.

Where two steps run together, both are waited for before a failure is acted on, so a failed startup
never leaves the other step running.

The bundler EntryPoint check still comes first. It is a gate, and delegation uses the bundler.

## Startup does not wait for the first balance read

`bootFiller` starts the `BalanceProvider` without waiting for its first read, which covers every
chain and takes as long as the slowest one. Until it lands, `getSnapshot()` returns the `loading`
snapshot. The dashboard shows that state, and `inventoryOf` and `availableStablecoinLiquidity`
return `null` for anything that is not `fresh`, so no inventory snapshot or liquidity alert is made
from it.

The dashboard re-reads `/api/balances` every second while the snapshot is `loading`
(`balancePollMs` in `ui/src/operator/Operator.tsx`), then every 30 seconds.
