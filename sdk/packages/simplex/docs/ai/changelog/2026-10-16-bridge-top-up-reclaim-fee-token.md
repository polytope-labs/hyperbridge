# 2026-10-16 — A BRIDGE top-up reclaim pays the cancel's relayer fee in the fee token

A cross-chain reclaim always pays the cancel's relayer fee in the source host's fee token. It
decodes the `relayerFee` from the SDK's `cancelOrder` calldata, approves the gateway for it, and
calls `cancelOrder` with value 0. The SDK's native quote is never sent: the filler wallet holds
stablecoins, not native, and a sponsored op carrying native value reverts while the paymaster
still charges for it.

A wallet short of the fee token fails before anything is sent, with a message naming the
shortfall: `Not enough USDC on EVM-8453 to pay the cancel's relayer fee: have 0.1, need 0.31`.
The check compares the wallet's balance with the fee alone; the paymaster's gas reserve is not
added, so it blocks only a cancel that is certain to revert.

Recovery retries every 60 s and stops after five consecutive failures, returning the top-up to
`unfilled` with the error so Start recovery can resume it. The count restarts on a newly recorded
step, on an operator restart, and after an attempt that ran 10 min or more before failing (a
finality wait, not a fast failure). The in-memory count also restarts with the process.

Once Hyperbridge has finalized the cancellation, a retry or restart goes straight to the refund
claim and does not run the SDK cancel again, which would send a second cancel.
