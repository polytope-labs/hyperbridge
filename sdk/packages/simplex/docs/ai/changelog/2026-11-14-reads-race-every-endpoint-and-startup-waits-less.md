# Reads race every endpoint, and startup waits less

A solver with five chains on the default public endpoints took over three minutes to start. The
ordered `fallback` transport retried each endpoint three times with 1s, 2s and 4s of backoff before
moving on, so every endpoint that was down ahead of a live one added seven seconds to every read.
Startup then paid that one chain at a time.

## Reads race

`ChainClientManager.raceTransport` is the public client's transport for a chain with more than one
endpoint. A read is sent to all of the chain's endpoints at once, and the first answer is used. Two
kinds of read are treated differently, and both are described below: an answer that says "nothing",
and nonces and logs.

- A result is an answer. So is a revert, by the same `shouldThrow` rule viem's `fallback` uses to
  stop failing over.
- An endpoint that is unreachable, times out, rate-limits or refuses the method is skipped. The read
  fails only when every endpoint has failed.
- No endpoint retries on its own. The other endpoints are the retry. When all of them fail, the
  race reports a failure viem retries, such as a timeout or a 503, ahead of one it does not, such
  as "method not found" or a 401. The whole race is then retried with viem's defaults, up to three
  more times. `isRetried` mirrors viem's `shouldRetry`, which viem does not export.

That preference has a cost. When one endpoint refuses a method and the others are down, the read is
retried although it cannot succeed, and the error reported is the outage, not the refusal.

### An answer that says "nothing"

A node that is a block behind answers `null` for a receipt, transaction or block another node
already has. A fast "nothing" should not beat a slower result.

`firstResponse` therefore holds an empty answer back. Empty means `null`, an empty array, or `0x`
from `eth_getCode`. It is returned once every endpoint has answered or failed, or
`EMPTY_ANSWER_GRACE_MS` (one second) after the first empty answer, whichever comes first. A
non-empty answer from any endpoint in that time is returned instead.

This is a preference, not a guarantee:

- An endpoint that failed has nothing to add. If the only endpoint that has the result is
  rate-limited on that call, the empty answer is returned at once.
- A read whose true answer is empty takes up to a second when any endpoint is slow to answer.
- The preference also works on a value that has just become empty. After a delegation is revoked,
  a node that is behind still reports the old code, and that answer is the one returned.

`0x` from `eth_call` is data and is not held back. Neither is a number: a balance or a block number
from a node that is behind is returned as it is.

### Receipt waits do not check for a replacement

`waitForTransactionReceipt` on the public client runs with `checkReplacement: false` unless the
caller passes it. viem's replacement check reads the transaction's block and then its receipt. When
those two reads land on endpoints at different heights, the receipt is missing for a transaction
that has mined, and viem rejects the wait. With the check off, a missing receipt only means the wait
polls again. A wait still ends early when every endpoint fails the receipt read on all four
attempts, as it does with viem's default.

The cost: a transaction that is replaced, for example re-sent with a higher fee from another tool
using the same key, is not followed. The wait for the original hash times out. Simplex never
re-sends a wallet transaction at the same nonce.

### Nonces and logs are read in order

Some reads do not race (`readsInOrder`). They go to the endpoints in list order, the order the
wallet client sends in, and move to the next only when one fails. A node that is behind answers each
of them wrongly without saying so.

- `eth_getTransactionCount`. A nonce from a node that has not seen the last transaction is one too
  low.
- An `eth_call` to `EntryPoint.getNonce` or to an ERC-2612 token's `nonces`, matched by selector.
  They are the nonces a UserOp and a permit are built on. The bundler refuses a UserOp nonce that
  was just used, and a permit signed over a used nonce is invalid.
- `eth_getLogs`. A node that is behind, or that keeps few logs, answers `[]` at once for a range
  that holds some.

One difference from the wallet client remains: an endpoint is not retried before the read moves on,
where a send retries it three times first. Like a send, the read waits out the 30-second timeout of
an endpoint that does not answer at all.

### One node's view of a head and its logs

`BridgeTopUp` scans the gateway's logs for its own placement and cancel. The scan reads the head and
then the logs up to it, and takes an empty range as evidence that nothing was placed. With the head
from one node and the logs from another that is behind, the range would look complete for blocks the
second node has not seen. Reading in list order is not enough for that: a client that fails over per
request takes the head from the first endpoint and, when that one refuses the log range, the logs
from the next.

So the scan runs whole on one endpoint. `ChainClientManager.getEndpointClients` returns one client
per endpoint, in list order, each with the usual three retries. The scan uses the first, and moves
to the next only when the first fails any of its reads, starting again from that endpoint's own
head. An endpoint that refuses a log range gets a smaller one before it is given up on. What the
top-up decides afterwards is still read at a height an evidence margin below the one the scan
reached.

### What this means for an operator

This applies to every read through `ChainClientManager.getPublicClient`, during normal operation as
well as at startup.

- Every listed endpoint receives every read outside the quorum, apart from nonces and logs.
- The first-listed endpoint is not the one that answers. Two consecutive reads can come from nodes at
  different heights.
- The wallet client keeps the ordered `fallback`, so a transaction goes to the first endpoint, and
  to the next only when that one fails.
- The SDK's `IntentGateway` helper is built on the first-listed endpoint alone
  (`ContractInteractionService.primaryRpcUrl`), so the reads it makes do not race.
- `QuorumPublicClient` is unchanged. It is still the client that cross-checks answers.

A chain with a single endpoint keeps plain `http` with its retries.

## Chain ids are probed once, for all chains together

Every endpoint is asked for its chain id at startup, and `fetchChainId` gives each one ten seconds.

- `Simplex.start` runs that probe once and hands the result to the scanner it builds and to
  `bootFiller`, through `BootOptions.resolvedChains`. `bootFiller` probes on its own when it is not
  handed the chains.
- `OrderScanner.create` probes all its entries that omit a `chainId` together. The adds stay in
  order, so two entries that resolve to the same chain are still refused.

## Every endpoint that answers must agree on the chain

Startup waits for each endpoint's chain id, for up to the ten seconds `fetchChainId` allows. One that
cannot answer is logged and kept, as before. One that answers for another chain refuses the config.
Reads go to every endpoint, so a wrong-chain endpoint that was not waited for could answer them.

## Startup steps that do not depend on each other run together

- `IntentFiller.solverSelectionChains`, `DelegationService.setupDelegationOnChains` and the
  EntryPoint deposit top-ups run their chains concurrently. Delegation state is keyed by wallet and
  chain, so the chains do not share any.
- `IntentFiller.initialize` connects to Hyperbridge while it sets the chains up.
- `bootFiller` starts the limit-order lifecycle while the filler initializes.

Where two steps run together, both have settled before a failure is thrown, so a failed startup
does not return while the other step is still running.

The bundler EntryPoint check still comes first. It is a gate, and delegation uses the bundler.

## Startup does not wait for the first balance read

`bootFiller` starts the `BalanceProvider` without waiting for its first read, which covers every
chain and takes as long as the slowest one. Until it lands, `getSnapshot()` returns the `loading`
snapshot. The dashboard shows that state, and `inventoryOf` and `availableStablecoinLiquidity`
return `null` for anything that is not `fresh`, so no inventory snapshot or liquidity alert is made
from it.

The dashboard re-reads `/api/balances` every second while the snapshot is `loading`
(`balancePollMs` in `ui/src/operator/Operator.tsx`), then every 30 seconds.
