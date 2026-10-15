# 2026-10-15 — A transient bundler failure no longer skips an order

`estimateGasFillPost` requires the bundler's own estimate (see
`2026-10-01-no-bid-without-a-bundler-estimate.md`), so one failed `eth_estimateUserOperationGas`
used to cost the order. On 2026-10-15 a $298 BSC to Base order was skipped and refunded because
rundler's upstream RPC dropped one connection and rundler answered
`-32603 internal error: rpc provider error`.

## What is retried

`GasEstimator.estimateFillOrder` and `GasEstimator.estimateBidPreVerificationGas` in
`@hyperbridge/sdk` send `eth_estimateUserOperationGas` through `retryTransientBundlerErrors`
(`sdk/packages/sdk/src/protocols/intents/bundlerRetry.ts`): at most 3 attempts, waiting 250 ms
and then 750 ms (`BUNDLER_ESTIMATE_RETRY_DELAYS_MS`), so at most 1 s of waiting before the order
is skipped as before.

`CryptoUtils.sendBundler` now throws a `BundlerRpcError` that carries the JSON-RPC `code` and
`data`, the HTTP status and a `transport` flag. Its message is unchanged for JSON-RPC errors
(`Bundler error: <message>`). `isTransientBundlerError` retries:

- a request or body that failed in transit (`fetch failed`, connection reset or refused, timeouts);
- JSON-RPC `-32603` (internal error), unless the message names an `AA..` code;
- HTTP 429 or 5xx, unless the body is a JSON-RPC error with a deterministic code.

It never retries an `AA..` EntryPoint revert, the ERC-4337 range `-32500` to `-32599` (`-32500`,
`-32501`, `-32502`, `-32521` and the rest), `-32700` or `-32600` to `-32602` (invalid params,
including rundler's `Unsupported entry point`), whatever the HTTP status. Any other JSON-RPC code
is retried only on an HTTP 429 or 5xx, and an error that did not come from `sendBundler` never is.
Those fail on the first attempt, as before.

`eth_sendUserOperation` is not retried. `Bid.execute` sends it once through `sendBundler`, and
Simplex's `UserOpSender` uses its own client, whose contract is that a rejected send was never
submitted, so the caller may fall back to a native transaction. A send whose response was lost
may still be in the mempool, so an automatic resend is not safe on either path.

## Logging

Both params types take `onBundlerRetry(event: BundlerRetryEvent)`, called before each retry with
`attempt`, `maxAttempts`, `delayMs` and `error`. Without it the SDK writes a `console.warn`.
`ContractInteractionService` passes one that logs, through the `contract-service` logger:

```
WARN Bundler gas estimate failed transiently, retrying  { orderId, attempt, maxAttempts, delayMs, err }
```
