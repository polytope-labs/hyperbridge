import { isNil } from "lodash-es"

import type { RetryConfig } from "@/types"
import { retryPromise, sleep } from "@/utils"
import { AbortSignalInternal } from "@/utils/exceptions"

import type { ClientContext } from "./types"

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
