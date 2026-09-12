import { HttpProvider } from "@polkadot/api"
import type { TokenBucket } from "./rateLimiter"

/**
 * jsonrpsee's reply when the node was started with `--rpc-disable-batch-requests`. It comes back as
 * a single error object rather than an array, under HTTP 200.
 */
const BATCHES_NOT_SUPPORTED_CODE = -32005

/** jsonrpsee's reply when the batch exceeded `--rpc-max-batch-request-len`. Also a single object. */
const TOO_BIG_BATCH_REQUEST_CODE = -32010

/**
 * Most calls in one HTTP request. Kept well under any plausible `--rpc-max-batch-request-len` and
 * small enough that one request stays a reasonable unit of work to retry.
 */
const DEFAULT_MAX_BATCH_SIZE = 32

/** One queued call, waiting for the flush that will carry it. */
interface PendingCall {
	id: number
	method: string
	params: unknown[]
	resolve: (value: unknown) => void
	reject: (err: Error) => void
}

interface JsonRpcError {
	code: number
	message: string
	data?: unknown
}

interface JsonRpcResponse {
	id?: number | null
	jsonrpc?: string
	result?: unknown
	error?: JsonRpcError
}

/** Reproduces polkadot-js's `RpcError`: callers read `code` off it, and match on the message. */
function rpcError({ code, message, data }: JsonRpcError): Error {
	const suffix = data === undefined ? "" : `: ${typeof data === "string" ? data : JSON.stringify(data)}`
	const error = new Error(`${code}: ${message}${suffix}`) as Error & { code?: number; data?: unknown }
	error.code = code
	error.data = data
	return error
}

/**
 * An `HttpProvider` that coalesces concurrent calls into a single JSON-RPC 2.0 batch request.
 *
 * The point is the request count, not the call count. Rate limits in front of a node are counted in
 * HTTP requests, and a burst of concurrent calls — one offchain read per configured chain on a
 * phantom order interval, one block hash per block in a scan — is one request's worth of traffic
 * arriving as N. Calls queued within the same macrotask go out together, and the token bucket is
 * charged once per request rather than once per call, because one request is what the endpoint
 * counts.
 *
 * This does nothing for callers that await each call before making the next: there is never more
 * than one in flight to coalesce. Sequential code has to be made concurrent to benefit, which is a
 * change at the call site, not here.
 *
 * A lone call is sent as a plain request object rather than a one-element array, so the common case
 * is byte-for-byte what the base provider would have sent and does not depend on the server
 * supporting batches at all.
 *
 * Substrate enables batching by default (`sc-cli` resolves `BatchRequestConfig::Unlimited` unless
 * the operator passes `--rpc-disable-batch-requests` or `--rpc-max-batch-request-len`), and both
 * refusals are handled: the first disables batching for the life of the provider, the second halves
 * the batch size and retries. Neither loses a call.
 */
export class BatchingHttpProvider extends HttpProvider {
	readonly #endpoint: string
	readonly #headers: Record<string, string>
	readonly #limiter: TokenBucket
	#maxBatchSize: number
	#batchingSupported = true

	#pending: PendingCall[] = []
	#flushTimer: ReturnType<typeof setTimeout> | null = null
	#flushing = false
	#nextId = 1

	constructor(
		endpoint: string,
		headers: Record<string, string>,
		limiter: TokenBucket,
		maxBatchSize: number = DEFAULT_MAX_BATCH_SIZE,
	) {
		// Capacity 0 keeps the base's response cache off, as it has been since 2026-08-21. Nothing
		// here reaches the base's own `send`, but the constructor argument is the documented way to
		// say "no cache" and costs nothing to keep honest.
		super(endpoint, headers, 0)
		this.#endpoint = endpoint
		this.#headers = headers
		this.#limiter = limiter
		this.#maxBatchSize = maxBatchSize
	}

	/**
	 * `isCacheable` is accepted for interface compatibility and ignored: this provider has no
	 * response cache, which is deliberate — see the note in `IntentsCoprocessor.http`.
	 */
	override async send<T>(method: string, params: unknown[], _isCacheable?: boolean): Promise<T> {
		return new Promise<T>((resolve, reject) => {
			this.#pending.push({
				id: this.#nextId++,
				method,
				params,
				resolve: resolve as (value: unknown) => void,
				reject,
			})
			// A full batch goes now rather than waiting out the window: the window exists to collect
			// a burst, and a full batch has collected one.
			if (this.#pending.length >= this.#maxBatchSize) this.#flushNow()
			else this.#scheduleFlush()
		})
	}

	override clone(): BatchingHttpProvider {
		return new BatchingHttpProvider(this.#endpoint, this.#headers, this.#limiter, this.#maxBatchSize)
	}

	/** Calls waiting for a flush. Exposed for tests and diagnostics. */
	get queued(): number {
		return this.#pending.length
	}

	/**
	 * Collect for the rest of this macrotask, then send.
	 *
	 * A macrotask rather than a microtask because the bursts worth catching are not synchronous:
	 * `Promise.all` over a set of reads starts them in one synchronous run, but each then advances
	 * through several microtask turns of its own before reaching the provider. A microtask flush
	 * would fire between those turns and split one burst across several requests.
	 */
	#scheduleFlush(): void {
		// One request in flight at a time. Overlapping flushes interleave against a `maxBatchSize`
		// that a refusal can shrink underneath them, which fragments a burst into more requests than
		// it needs — the opposite of the point. Whatever arrives meanwhile is simply carried by the
		// next flush, which is also what makes each request as full as it can be.
		if (this.#flushTimer || this.#flushing) return
		const timer = setTimeout(() => {
			this.#flushTimer = null
			void this.#flush()
		}, 0)
		;(timer as unknown as { unref?: () => void }).unref?.()
		this.#flushTimer = timer
	}

	#flushNow(): void {
		// A flush already running will pick these up when it finishes; starting a second one here is
		// exactly the overlap `#scheduleFlush` refuses.
		if (this.#flushing) return
		if (this.#flushTimer) {
			clearTimeout(this.#flushTimer)
			this.#flushTimer = null
		}
		void this.#flush()
	}

	async #flush(): Promise<void> {
		if (this.#flushing) return
		const calls = this.#pending.splice(0, this.#maxBatchSize)
		if (calls.length === 0) return

		this.#flushing = true
		try {
			// One token per HTTP request, not per call. The bucket models the endpoint's limiter,
			// and what that counts is requests.
			await this.#limiter.acquire()
			await this.#post(calls)
		} finally {
			this.#flushing = false
			// Anything that arrived meanwhile, plus anything a refusal put back.
			if (this.#pending.length > 0) this.#scheduleFlush()
		}
	}

	async #post(calls: PendingCall[]): Promise<void> {
		const single = calls.length === 1
		const payload = calls.map(({ id, method, params }) => ({ id, jsonrpc: "2.0", method, params }))
		const body = JSON.stringify(single ? payload[0] : payload)

		let parsed: JsonRpcResponse | JsonRpcResponse[]
		try {
			const response = await fetch(this.#endpoint, {
				body,
				headers: {
					Accept: "application/json",
					"Content-Type": "application/json",
					...this.#headers,
				},
				method: "POST",
			})
			// Matches the base provider's message, which is what a 429 is recognised by upstream.
			if (!response.ok) throw new Error(`[${response.status}]: ${response.statusText}`)
			parsed = JSON.parse(await response.text()) as JsonRpcResponse | JsonRpcResponse[]
		} catch (err) {
			const error = err instanceof Error ? err : new Error(String(err))
			// The base provider appends the request that failed; with a batch, name them all.
			error.message = `${error.message}\nFailed HTTP Request: ${JSON.stringify(
				calls.map(({ method, params }) => ({ method, params })),
			)}`
			for (const call of calls) call.reject(error)
			return
		}

		if (Array.isArray(parsed)) {
			this.#settleBatch(calls, parsed)
			return
		}

		// A single object where a batch was sent is the server refusing the batch as a whole.
		if (!single) {
			this.#handleBatchRefusal(calls, parsed)
			return
		}
		this.#settle(calls[0], parsed)
	}

	#settleBatch(calls: PendingCall[], responses: JsonRpcResponse[]): void {
		const byId = new Map<number, JsonRpcResponse>()
		for (const response of responses) {
			if (typeof response?.id === "number") byId.set(response.id, response)
		}
		for (const call of calls) {
			const response = byId.get(call.id)
			if (response) this.#settle(call, response)
			else call.reject(new Error(`No response for ${call.method} in batch reply`))
		}
	}

	#settle(call: PendingCall, response: JsonRpcResponse): void {
		if (response?.error) {
			call.reject(rpcError(response.error))
			return
		}
		if (!response || response.result === undefined) {
			call.reject(new Error("No result found in jsonrpc response"))
			return
		}
		call.resolve(response.result)
	}

	/**
	 * The server rejected the batch itself rather than any call in it. Both forms are recoverable
	 * without losing a call, and neither should ever surface to a caller as a failure.
	 */
	#handleBatchRefusal(calls: PendingCall[], response: JsonRpcResponse): void {
		const code = response?.error?.code

		if (code === BATCHES_NOT_SUPPORTED_CODE) {
			// Nothing about this changes while the process runs, so stop trying.
			this.#batchingSupported = false
			this.#maxBatchSize = 1
			this.#requeue(calls)
			return
		}

		if (code === TOO_BIG_BATCH_REQUEST_CODE) {
			this.#maxBatchSize = Math.max(1, Math.floor(this.#maxBatchSize / 2))
			this.#requeue(calls)
			return
		}

		const error = response?.error
			? rpcError(response.error)
			: new Error("Malformed batch reply: neither an array nor an error")
		for (const call of calls) call.reject(error)
	}

	/** Puts calls back at the head of the queue, so a refused batch keeps its place in line. */
	#requeue(calls: PendingCall[]): void {
		this.#pending.unshift(...calls)
		this.#scheduleFlush()
	}

	/** Whether batches are still being attempted. Exposed for tests and diagnostics. */
	get batchingSupported(): boolean {
		return this.#batchingSupported
	}
}
