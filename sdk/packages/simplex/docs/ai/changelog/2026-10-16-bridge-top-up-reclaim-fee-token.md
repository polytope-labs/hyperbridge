# 2026-10-16 — A BRIDGE top-up reclaim pays the cancel's relayer fee in the fee token

A cross-chain reclaim always pays the cancel's relayer fee in the source host's fee token. It
decodes the `relayerFee` from the SDK's `cancelOrder` calldata, resets the gateway's allowance to
0, approves it for the fee, and calls `cancelOrder` with value 0. The SDK's native quote is never
sent: the filler wallet holds stablecoins, not native, and a sponsored op carrying native value
reverts while the paymaster still charges for it.

A wallet short of the fee token fails before anything is sent, with a message naming the
shortfall: `Not enough USDC on EVM-8453 to pay the cancel's relayer fee: have 0.1, need 0.31`.
The check compares the wallet's balance with the fee alone; the paymaster's gas reserve is not
added, so it blocks only a cancel that is certain to revert.

Recovery retries after 1, 2, 4, 8 and 16 min (`RECLAIM_RETRY_DELAYS_MS`) and stops after six
consecutive failures, about 31 min, returning the top-up to `unfilled` with the error so Start
recovery can resume it. The count restarts on a newly recorded step, on an operator restart, and
after an attempt that ran 10 min or more before failing (a finality wait, not a fast failure). The
in-memory count also restarts with the process. The timeline of a stopped recovery keeps the steps
it reached and marks the one it stopped at: `Recovery stopped: <reason> — Start recovery to resume`.
A recovery that failed before recording any step (for example every destination read timing out)
is recognized by its `Recovery stopped` error and marks its first row instead of showing only
`Order Expired`.

Once Hyperbridge has finalized the cancellation, a retry or restart goes straight to the refund
claim and does not run the SDK cancel again, which would send a second cancel. The SDK reports the
claim calldata once, so the finalized step and `claimCalldata` are saved in one write, and the
cancel's GET request is kept as `getCommitment`. The calldata is proven at a fixed Hyperbridge
height, so a claim that reverts clears it, and a missing or cleared `claimCalldata` is rebuilt with
`queryGetRequestWithStatus(getCommitment)` instead of replaying the old bytes. `getCommitment` is
read back from the cancel's receipt when a record lacks it. A sponsored cancel's receipt is its
whole bundle, so only a `GetRequestEvent` emitted by the source chain's host for a request sent by
the IntentGateway counts; another op's GET request in the same bundle is skipped, and a receipt
with none fails as before.
