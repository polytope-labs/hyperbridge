import { isNil } from "lodash-es"

import type { IChain } from "@/chain"
import { type RequestStatusWithMetadata, type RetryConfig, TimeoutStatus } from "@/types"
import { retryPromise, sleep } from "@/utils"
import { AbortSignalInternal } from "@/utils/exceptions"

import type { ClientContext } from "."

/**
 * Executes an async operation with exponential backoff retry, using the context's
 * default retry config as the base and letting callers override individual fields.
 */
export async function withRetry<T>(
	ctx: ClientContext,
	operation: () => Promise<T>,
	overrides: Partial<RetryConfig> = {},
): Promise<T> {
	return retryPromise(operation, { ...ctx.defaultRetryConfig, ...overrides })
}

/**
 * Sleeps for `duration` ms, logging a trace line.
 */
export function sleepFor(ctx: ClientContext, duration: number): Promise<void> {
	ctx.logger.trace(`Sleeping for ${duration}ms`)
	return sleep(duration)
}

/**
 * Sleeps for one poll interval.
 */
export function sleepForInterval(ctx: ClientContext): Promise<void> {
	return sleepFor(ctx, ctx.config.pollInterval)
}

/**
 * Shared watcher used by both POST and GET status streams: yields a single
 * `PENDING_TIMEOUT` event once `chain.timestamp()` passes `timeoutTimestamp`.
 * Yields nothing if `timeoutTimestamp === 0n` (no timeout configured).
 */
export async function* timeoutStream(
	ctx: ClientContext,
	timeoutTimestamp: bigint,
	chain: IChain,
): AsyncGenerator<RequestStatusWithMetadata, void> {
	const logger = ctx.logger.withTag("[timeoutStream()]")
	if (timeoutTimestamp === 0n) return

	let timestamp = await chain.timestamp()
	while (timestamp < timeoutTimestamp) {
		logger.trace("Comparing timeout timestamps", { control: timeoutTimestamp, latest: timestamp })
		const diff = BigInt(timeoutTimestamp) - BigInt(timestamp)
		await sleepFor(ctx, Number(diff))
		timestamp = await chain.timestamp()
	}

	yield {
		status: TimeoutStatus.PENDING_TIMEOUT,
		metadata: { blockHash: "0x", blockNumber: 0, transactionHash: "0x" },
	}
}

/**
 * Repeatedly invokes `params.promise()` with `params.predicate` (default: `isNil`)
 * deciding when to keep waiting. Throws {@link AbortSignalInternal} if the signal
 * trips. Returns the first value for which `predicate` is false.
 */
export async function waitOrAbort<T>(
	ctx: ClientContext,
	params: {
		signal: AbortSignal
		promise: () => Promise<T>
		predicate?: (value: T) => boolean
	},
): Promise<NonNullable<T>> {
	const { predicate = (value) => isNil(value) } = params

	const assertNotAborted = () => {
		if (params.signal.aborted) {
			throw new AbortSignalInternal("Terminated request in 'waitOrAbort'")
		}
	}

	while (true) {
		assertNotAborted()
		await sleepForInterval(ctx)
		assertNotAborted()
		const value = await params.promise()
		assertNotAborted()

		if (predicate(value)) continue
		return value as NonNullable<T>
	}
}
