import {
	createPublicClient,
	http,
	type AbiEvent,
	type BlockNumber,
	type BlockTag,
	type Chain,
	type GetLogsParameters,
	type GetLogsReturnType,
	type Hash,
	type PublicClient,
} from "viem"
import { getViemChain } from "@hyperbridge/sdk"
import { validateRpcUrls } from "./FillerConfigService"
import { moduleLogger, type Logger, type LoggerContext } from "./Logger"

/**
 * Standard BFT threshold for a set of `n` equal peers: `floor(2n/3) + 1`, i.e.
 * strictly more than two thirds, tolerating up to `floor((n-1)/3)` faults.
 *
 *  - n=1: 1   n=2: 2   n=3: 3   n=4: 3   n=5: 4   n=7: 5
 */
export function quorumThreshold(numProviders: number): number {
	return Math.floor((2 * numProviders) / 3) + 1
}

/** JSON-RPC error codes providers use to signal rate limiting (HTTP 200 body). */
const RATE_LIMIT_RPC_CODES = new Set([-32005, -32097, -32016, 429])

/**
 * How long a rate-limited endpoint sits out. Long enough for a per-minute
 * quota to reset, short enough that a healthy endpoint is not benched over one
 * burst. Exported for tests.
 */
export const RATE_LIMIT_SUSPENSION_MS = 5 * 60_000

/**
 * What one quorum call is allowed to spend.
 *
 * `deadlineMs` is the ceiling on the call itself: past it, the endpoints still
 * outstanding are treated as failures, so they stop voting and the call is
 * decided over the endpoints that answered, or fails with a QuorumError naming
 * them. Early exit decides as soon as the endpoints still out could not change
 * the outcome; the deadline decides when one of them never answers.
 *
 * `timeoutMs` bounds one endpoint's attempt, and there is only ever one: the
 * transports are built with `retryCount: 0`, so an endpoint's whole
 * contribution to a call is a single request. That is what keeps the two
 * numbers independent — a task cannot approach `deadlineMs` by retrying inside
 * itself, so a deadline that fires means something outside the budget went
 * wrong rather than an endpoint using the budget as designed.
 */
export interface QuorumBudget {
	timeoutMs: number
	deadlineMs: number
}

/**
 * Scanning: a poll every few seconds, one request per endpoint. A read that has
 * not answered within seconds is already stale, and it holds the scan mutex
 * while it waits — which is how a slow minority used to stall a whole chain.
 */
export const SCAN_BUDGET: QuorumBudget = {
	timeoutMs: 5_000,
	deadlineMs: 12_000,
}

/**
 * Confirmation polling: `getTransactionConfirmations` makes two requests in
 * sequence per endpoint (head, then receipt), so the scan budget would cut off
 * a slow but honest endpoint mid-sequence. Roomier on both counts, and still
 * bounded — the poller is deciding whether money is safe to pay out, so it can
 * afford to wait longer than a scan, but not forever.
 */
export const CONFIRMATION_BUDGET: QuorumBudget = {
	timeoutMs: 10_000,
	deadlineMs: 30_000,
}

/**
 * A provider answering with something that is not JSON at all — a plain-text
 * throttle notice, an HTML error page — is not going to answer the next poll
 * correctly either, so it sits out briefly. Shorter than the rate-limit bench
 * because the cause is unknown; long enough to stop it being queried on every
 * 3-second scan. (BSC's thirdweb endpoint returns its quota notice this way:
 * viem surfaces it as a JSON parse error, so the rate-limit classifier never
 * matched it and the endpoint was re-queried until it timed out, every scan.)
 */
export const MALFORMED_RESPONSE_SUSPENSION_MS = 30_000

/**
 * An endpoint whose head is behind the quorum's cannot answer for a range at
 * that head — it fails the query deterministically until it catches up. Benched
 * only briefly: lag is transient, and over-benching shrinks the voter set,
 * which is the one thing the quorum exists to protect.
 */
export const STALE_HEAD_SUSPENSION_MS = 15_000

/**
 * Whether an error looks rate-limit related — HTTP 429, a rate-limit-ish
 * JSON-RPC code, or rate-limit text anywhere in the message chain. Deliberately
 * loose: it only labels failures in diagnostics, where a false positive costs a
 * misleading tag. Suspension decisions use {@link isSuspendableRateLimit},
 * which is stricter. Exported for tests.
 */
export function isRateLimited(error: unknown): boolean {
	let current: unknown = error
	for (let depth = 0; depth < 6 && current instanceof Error; depth++) {
		const e = current as {
			status?: number
			code?: number
			message?: string
			details?: string
			shortMessage?: string
			metaMessages?: string[]
			cause?: unknown
		}
		if (e.status === 429) return true
		if (typeof e.code === "number" && RATE_LIMIT_RPC_CODES.has(e.code)) return true
		const text = [e.message, e.details, e.shortMessage, ...(e.metaMessages ?? [])].filter(Boolean).join(" ")
		if (/\b429\b|too many requests|rate.?limit/i.test(text)) return true
		current = e.cause
	}
	return false
}

/** JSON-RPC codes that specifically mean request-rate throttling, nothing else. */
const SUSPENDABLE_RPC_CODES = new Set([-32016, -32097, 429])

/**
 * Whether an error is unambiguously a request-rate limit — the only failures
 * worth benching an endpoint over. Stricter than {@link isRateLimited} on two
 * counts, both learned the hard way:
 *
 * - `-32005` alone does NOT qualify. EIP-1474 defines it as generic "limit
 *   exceeded", and Infura returns it for `eth_getLogs` queries over its 10k
 *   result cap — a deterministic property of the query, not a throttle.
 *   Benching on it would suspend a healthy endpoint every time a catch-up scan
 *   crosses a busy range, and re-bench it on every retry. It still qualifies
 *   when its text says it is a throttle ("too many requests").
 * - The free-text match must not read the request URL, or a key containing
 *   "ratelimit" benches its endpoint on ANY failure. viem folds `metaMessages`
 *   — which embed the URL — into `message` itself, so `message` is only
 *   consulted when the error carries no `metaMessages` (plain and synthetic
 *   errors); real viem errors put the provider's own words in `details` and
 *   `shortMessage`, which are always read. The bare `429` pattern is dropped
 *   for the same reason. (The first cut skipped `metaMessages` but still read
 *   `message`, which contains them — caught in review, verified end-to-end.)
 *
 * Exported for tests.
 */
export function isSuspendableRateLimit(error: unknown): boolean {
	let current: unknown = error
	for (let depth = 0; depth < 6 && current instanceof Error; depth++) {
		const e = current as {
			status?: number
			code?: number
			message?: string
			details?: string
			shortMessage?: string
			metaMessages?: unknown
			cause?: unknown
		}
		if (e.status === 429) return true
		if (typeof e.code === "number" && SUSPENDABLE_RPC_CODES.has(e.code)) return true
		const text = [e.details, e.shortMessage, e.metaMessages ? undefined : e.message].filter(Boolean).join(" ")
		if (/too many requests|rate.?limit/i.test(text)) return true
		current = e.cause
	}
	return false
}

/**
 * How long to bench, honouring the provider's own `Retry-After` when it sent
 * one (clamped to [1s, {@link RATE_LIMIT_SUSPENSION_MS}]) — a per-second
 * limiter answering `Retry-After: 1` should not cost five minutes of degraded
 * quorum. Absent a header, the full window applies.
 */
function suspensionMsFor(error: unknown): number {
	let current: unknown = error
	for (let depth = 0; depth < 6 && current instanceof Error; depth++) {
		const headers = (current as { headers?: { get?: (name: string) => string | null } }).headers
		const raw = typeof headers?.get === "function" ? headers.get("retry-after") : undefined
		if (raw) {
			const seconds = Number(raw)
			if (Number.isFinite(seconds)) {
				return Math.min(Math.max(seconds * 1000, 1_000), RATE_LIMIT_SUSPENSION_MS)
			}
		}
		current = (current as { cause?: unknown }).cause
	}
	return RATE_LIMIT_SUSPENSION_MS
}

/**
 * Whether the provider answered with something that is not JSON. viem reports
 * these as parse failures, so no status code or JSON-RPC code is available —
 * the message is all there is. Exported for tests.
 */
export function isMalformedResponse(error: unknown): boolean {
	let current: unknown = error
	for (let depth = 0; depth < 6 && current instanceof Error; depth++) {
		const e = current as { message?: string; details?: string; shortMessage?: string; cause?: unknown }
		const text = [e.details, e.shortMessage, e.message].filter(Boolean).join(" ")
		if (/is not valid JSON|Unexpected token|invalid json|Unexpected end of JSON input/i.test(text)) return true
		current = e.cause
	}
	return false
}

/**
 * Whether the provider is behind the block range it was asked for. Distinct
 * from a malformed query: the same request succeeds on endpoints that have
 * caught up, and will succeed here too, later. Exported for tests.
 */
export function isStaleHead(error: unknown): boolean {
	let current: unknown = error
	for (let depth = 0; depth < 6 && current instanceof Error; depth++) {
		const e = current as { message?: string; details?: string; shortMessage?: string; cause?: unknown }
		const text = [e.details, e.shortMessage, e.message].filter(Boolean).join(" ")
		if (
			/beyond the latest block|header not found|block not found|unknown block|not yet (been )?indexed/i.test(text)
		)
			return true
		current = e.cause
	}
	return false
}

/**
 * Whether a failure is worth benching the endpoint over, and for how long.
 *
 * The three cases share one property: the endpoint will keep failing the same
 * way for a while, so re-querying it on the next poll only spends the call's
 * time budget. Everything else — a bad query, a one-off 5xx, a hiccup — stays
 * per-call, as the class doc promises. Exported for tests.
 */
export function benchFor(error: unknown): { durationMs: number; reason: string } | null {
	if (isSuspendableRateLimit(error)) return { durationMs: suspensionMsFor(error), reason: "rate-limited" }
	if (isMalformedResponse(error))
		return { durationMs: MALFORMED_RESPONSE_SUSPENSION_MS, reason: "returned a malformed response" }
	if (isStaleHead(error)) return { durationMs: STALE_HEAD_SUSPENSION_MS, reason: "behind the quorum head" }
	return null
}

/** Whether a getTransactionReceipt rejection means "no such receipt" (a valid answer, not a fault). */
function isReceiptNotFound(error: unknown): boolean {
	return error instanceof Error && error.name === "TransactionReceiptNotFoundError"
}

/**
 * Wraps multiple viem `PublicClient`s — one per configured RPC URL — and runs
 * selected read paths (`getLogs`, `getBlockNumber`, `getTransactionConfirmations`)
 * as a **BFT quorum**: a result is accepted only when `quorumThreshold(m)` of the
 * `m` endpoints that answered agree on it.
 *
 * A provider that *answers with divergent data* is never special-cased away — it
 * simply fails to join the agreeing group, which is what makes a lying or reorged
 * endpoint detectable rather than authoritative.
 *
 * A provider that *fails* does not vote. A failed request says nothing about the
 * chain, so it must not outvote the endpoints that agree: the threshold guards
 * against endpoints that answer wrongly, and every answer still counts. Six
 * endpoints with two failing decide over the other four, three of which must
 * agree. The price, worth stating plainly, is that a call can be decided by fewer
 * endpoints than the operator configured; at the limit, one endpoint answering
 * while the rest fail decides alone. The voters are still exclusively the
 * operator's own.
 *
 * The bench is the one state this class keeps, and {@link benchFor} decides who
 * goes on it: a request-rate limit ({@link isSuspendableRateLimit}), a response
 * that is not JSON ({@link isMalformedResponse}), or a head behind the quorum's
 * ({@link isStaleHead}). All three share the property that earns it — the
 * endpoint will keep failing the same way for a while — so a benched endpoint is
 * not asked for its window: asking would spend its quota, or hold the call open
 * for an answer already known to fail. When every endpoint is benched, all are
 * queried again; there is nobody left to prefer. Every other failure is per
 * call, and the endpoint is asked again next time.
 *
 * The constructor validates that URLs resolve to distinct hostnames so a "quorum"
 * isn't secretly the same upstream in disguise.
 */
export class QuorumPublicClient {
	public readonly clients: PublicClient[]
	public readonly rpcUrls: string[]
	/**
	 * The quorum bar when every endpoint answers: `quorumThreshold(n)` over the
	 * full set. Each call's bar is `quorumThreshold` over the endpoints that answer
	 * it, so this is the most it can be.
	 */
	public readonly threshold: number
	private readonly logger?: Logger

	/**
	 * Rate-limit bench state, shared across every instance and keyed by endpoint
	 * URL. The bench describes the ENDPOINT, not the wrapper: the scanner and the
	 * confirmation poller construct separate clients over the same URLs, and a
	 * throttle one of them observed is a throttle for both. Instance state would
	 * halve the relief — and evaporate whenever `setRpcUrls` rebuilds a client.
	 */
	private static benchedUntil = new Map<string, number>()

	/** Test hook: forgets every bench. Production never calls it. */
	static clearAllSuspensions(): void {
		QuorumPublicClient.benchedUntil.clear()
	}

	/** What this client's calls may spend; see {@link QuorumBudget}. */
	private readonly deadlineMs: number

	constructor(chainId: number, rpcUrls: string[], loggers?: LoggerContext, budget: QuorumBudget = SCAN_BUDGET) {
		this.rpcUrls = validateRpcUrls(rpcUrls)
		this.threshold = quorumThreshold(this.rpcUrls.length)
		this.logger = loggers ? moduleLogger(loggers, "quorum-client") : undefined
		this.deadlineMs = budget.deadlineMs
		const chain = getViemChain(chainId) as Chain
		this.clients = this.rpcUrls.map((url) =>
			createPublicClient({
				chain,
				transport: http(url, {
					timeout: budget.timeoutMs,
					// No transport-level retry. viem's retry sleeps for the provider's
					// `Retry-After` verbatim and uncapped, which reaches the socket
					// before {@link suspensionMsFor} can apply its clamp — and even
					// without that header, retrying inside the task is what makes a
					// task's worst case (timeout, backoff, timeout) approach the
					// deadline that is supposed to bound it. One attempt per endpoint
					// per call; `retryPromise` above the quorum owns the retry policy.
					retryCount: 0,
				}),
			}),
		)
	}

	get size(): number {
		return this.clients.length
	}

	/** Endpoints currently benched, by URL. See {@link benchFor} for the reasons. */
	suspended(): string[] {
		const now = Date.now()
		return this.rpcUrls.filter((url) => (QuorumPublicClient.benchedUntil.get(url) ?? 0) > now)
	}

	/**
	 * The endpoints this call queries: everyone not benched. A benched endpoint is
	 * dropped outright: it answers nothing while throttled, so asking it could only
	 * cost a request. With every endpoint benched, everyone is queried again; there
	 * is nobody left to prefer.
	 */
	private participants(): {
		queried: Array<{ idx: number; client: PublicClient }>
		skipped: number
		allBenched: boolean
	} {
		const now = Date.now()
		const all = this.clients.map((client, idx) => ({ idx, client }))
		const active = all.filter(({ idx }) => (QuorumPublicClient.benchedUntil.get(this.rpcUrls[idx]) ?? 0) <= now)
		const allBenched = active.length === 0
		const queried = allBenched ? all : active
		return { queried, skipped: all.length - queried.length, allBenched }
	}

	/**
	 * Records a task failure against its endpoint. Only the failures
	 * {@link benchFor} names carry state: everything else stays per-call, as the
	 * class doc promises.
	 * The bench is the one silent state transition this class has, so it warns —
	 * an operator whose bar just dropped from 2-of-2 to 1-of-1 should not need a
	 * QuorumError to find out.
	 */
	private noteFailure(idx: number, error: unknown): void {
		const bench = benchFor(error)
		if (!bench) return
		const url = this.rpcUrls[idx]
		QuorumPublicClient.benchedUntil.set(url, Date.now() + bench.durationMs)
		const remaining = this.size - this.suspended().length
		this.logger?.warn(
			{
				url,
				reason: bench.reason,
				durationMs: bench.durationMs,
				queriedAfter: remaining > 0 ? remaining : this.size,
				quorumAfter: quorumThreshold(remaining > 0 ? remaining : this.size),
				of: this.size,
			},
			`Endpoint ${bench.reason}; suspending it and recomputing the quorum bar over the rest`,
		)
	}

	/**
	 * Highest block a quorum of endpoints have all indexed: the highest B such
	 * that `threshold` endpoints report head ≥ B. Returns null when fewer than
	 * `threshold` endpoints have responded.
	 */
	private quorumHead(heads: readonly bigint[], threshold: number): bigint | null {
		if (heads.length < threshold) return null
		const desc = (a: bigint, b: bigint) => (a > b ? -1 : a < b ? 1 : 0)
		return [...heads].sort(desc)[threshold - 1]
	}

	/**
	 * Settles per-client tasks incrementally and resolves the moment the
	 * evaluator can decide from the responses seen so far — the hot path never
	 * blocks on the slowest provider (a hung endpoint with a 30s timeout × 3
	 * retries would otherwise stall every confirmation poll). Stragglers settle
	 * out of band and are ignored. `tryDecide` returns undefined to keep
	 * waiting; once every task has settled, `finalize` must decide (it throws
	 * the QuorumError when the full response set still has no quorum).
	 *
	 * Both are handed the bar: the BFT threshold over the endpoints that have not
	 * failed, answered or still out. A failure lowers it rather than counting
	 * against it. Counting the pending endpoints keeps an early decision to a bar
	 * no lower than the full response set would get, so a vote it did not wait for
	 * could never have reversed it.
	 *
	 * `deadlineMs` bounds the other direction: when the bar is NOT reachable
	 * from the fast responders, the call used to wait out every straggler, and
	 * the caller's own retry loop multiplied that into minutes of a chain not
	 * being scanned at all, silently. At the deadline the outstanding endpoints
	 * are treated as failures — named in the QuorumError, so the operator sees
	 * which ones held the call up — and `finalize` decides over the endpoints
	 * that answered.
	 */
	private settleUntilQuorum<T, R>(
		tasks: Array<{ idx: number; task: Promise<T> }>,
		tryDecide: (fulfilled: ReadonlyArray<{ idx: number; value: T }>, threshold: number) => R | undefined,
		finalize: (
			fulfilled: ReadonlyArray<{ idx: number; value: T }>,
			failures: ReadonlyArray<{ idx: number; error: unknown }>,
			threshold: number,
		) => R,
	): Promise<R> {
		return new Promise<R>((resolve, reject) => {
			const fulfilled: Array<{ idx: number; value: T }> = []
			const failures: Array<{ idx: number; error: unknown }> = []
			const settled = new Set<number>()
			let outstanding = tasks.length
			let done = false
			let timer: ReturnType<typeof setTimeout> | undefined

			const finish = (action: () => void) => {
				done = true
				if (timer) clearTimeout(timer)
				action()
			}

			const evaluate = () => {
				if (done) return
				try {
					const threshold = quorumThreshold(tasks.length - failures.length)
					const early = tryDecide(fulfilled, threshold)
					if (early !== undefined) {
						finish(() => resolve(early))
						return
					}
					if (outstanding === 0) {
						const value = finalize(fulfilled, failures, threshold)
						finish(() => resolve(value))
					}
				} catch (error) {
					finish(() => reject(error))
				}
			}

			if (outstanding === 0) {
				evaluate()
				return
			}

			timer = setTimeout(() => {
				if (done) return
				// Not benched: slow is not the same as broken, and a deadline says
				// nothing about which endpoint was at fault. The names go into the
				// error instead.
				const stragglers = tasks
					.filter(({ idx }) => !settled.has(idx))
					.map(({ idx }) => ({
						idx,
						error: new Error(`no answer within the ${this.deadlineMs}ms quorum deadline`),
					}))
				try {
					const value = finalize(fulfilled, [...failures, ...stragglers], quorumThreshold(fulfilled.length))
					finish(() => resolve(value))
				} catch (error) {
					finish(() => reject(error))
				}
			}, this.deadlineMs)
			// A pending quorum read must never be the reason the process stays alive.
			timer.unref?.()
			tasks.forEach(({ idx, task }) => {
				task.then(
					(value) => {
						settled.add(idx)
						outstanding--
						fulfilled.push({ idx, value })
						evaluate()
					},
					(error) => {
						// Recorded even for stragglers that settle after the call decided:
						// a rate limit learned late still spares the endpoint next call.
						this.noteFailure(idx, error)
						settled.add(idx)
						outstanding--
						failures.push({ idx, error })
						evaluate()
					},
				)
			})
		})
	}

	/**
	 * Highest block head backed by a quorum of the endpoints that answer. Throws
	 * {@link QuorumError} when none do.
	 */
	async getBlockNumber(): Promise<bigint> {
		const { queried, skipped, allBenched } = this.participants()
		return this.settleUntilQuorum(
			queried.map(({ idx, client }) => ({ idx, task: client.getBlockNumber() })),
			// Early exit as soon as a quorum is satisfiable. Late voters could
			// only raise the reported head; the earlier (lower) head is
			// conservative for every consumer (fewer confirmations counted,
			// smaller scan windows).
			(fulfilled, threshold) =>
				this.quorumHead(
					fulfilled.map((f) => f.value),
					threshold,
				) ?? undefined,
			(fulfilled, failures, threshold) => {
				const head = this.quorumHead(
					fulfilled.map((f) => f.value),
					threshold,
				)
				if (head === null) {
					throw new QuorumError(
						`Quorum not reached for getBlockNumber: ${this.describeResponders(fulfilled.length, queried.length, skipped, allBenched)}. ` +
							this.formatFailures(failures.map(({ idx, error }) => ({ idx, error }))),
					)
				}
				return head
			},
		)
	}

	/**
	 * Confirmation count for a transaction under the quorum.
	 *
	 * Every endpoint is asked for the receipt and its head. A "receipt not found"
	 * answer is a valid **no** vote — the endpoint is responsive but does not see
	 * the transaction (not yet propagated, or reorged out) — so it counts toward
	 * responsiveness but joins no inclusion group. The transaction is confirmed
	 * only when a quorum agrees on the same `(blockHash, blockNumber)`; the depth
	 * is then the quorum head of that agreeing group. A minority still serving a
	 * reorged/fabricated receipt can never reach the quorum.
	 *
	 * Throws {@link QuorumError} when no inclusion reaches the quorum — including
	 * the ordinary window where the tx is not yet mined on enough endpoints.
	 */
	async getTransactionConfirmations({ hash }: { hash: Hash }): Promise<bigint> {
		type ReceiptProbe = { head: bigint; receipt: { blockHash: string; blockNumber: bigint } | null }
		const toViews = (fulfilled: ReadonlyArray<{ idx: number; value: ReceiptProbe }>): ReceiptView[] => {
			const views: ReceiptView[] = []
			for (const { value } of fulfilled) {
				if (!value.receipt) continue
				views.push({
					blockHash: value.receipt.blockHash,
					blockNumber: value.receipt.blockNumber,
					head: value.head,
				})
			}
			return views
		}

		const { queried, skipped, allBenched } = this.participants()
		return this.settleUntilQuorum(
			queried.map(({ idx, client }) => ({
				idx,
				task: (async (): Promise<ReceiptProbe> => {
					// Head first: a failure here means the endpoint is unresponsive.
					const head = await client.getBlockNumber()
					let receipt: { blockHash: string; blockNumber: bigint } | null = null
					try {
						const r = await client.getTransactionReceipt({ hash })
						receipt = { blockHash: r.blockHash, blockNumber: r.blockNumber }
					} catch (error) {
						if (!isReceiptNotFound(error)) throw error
						// not-found: a valid "no" vote — keep the endpoint as responsive.
					}
					return { head, receipt }
				})(),
			})),
			// Early exit only on a POSITIVE quorum: an agreeing inclusion group can
			// only grow, so the first satisfiable quorum is final. Not-found votes
			// never decide early — "not confirmed" needs the full response set.
			(fulfilled, threshold) => aggregateConfirmations(toViews(fulfilled), threshold) ?? undefined,
			(fulfilled, failures, threshold) => {
				const confirmations = aggregateConfirmations(toViews(fulfilled), threshold)
				if (confirmations === null) {
					throw new QuorumError(
						`Quorum not reached for getTransactionConfirmations(${hash}): no inclusion agreed on by a ` +
							`quorum (${threshold}). ${this.describeResponders(fulfilled.length, queried.length, skipped, allBenched)}. ` +
							this.formatFailures(failures.map(({ idx, error }) => ({ idx, error }))),
					)
				}
				return confirmations
			},
		)
	}

	/**
	 * Fetches logs from every endpoint in parallel and returns the result once a
	 * quorum agrees. Fails with {@link QuorumError} otherwise.
	 *
	 * Generics mirror `PublicClient.getLogs` so caller-side inference (event decoding,
	 * strict mode, pending vs mined) is preserved end-to-end.
	 */
	async getLogs<
		const TAbiEvent extends AbiEvent | undefined = undefined,
		const TAbiEvents extends readonly AbiEvent[] | readonly unknown[] | undefined = TAbiEvent extends AbiEvent
			? [TAbiEvent]
			: undefined,
		TStrict extends boolean | undefined = undefined,
		TFromBlock extends BlockNumber | BlockTag | undefined = undefined,
		TToBlock extends BlockNumber | BlockTag | undefined = undefined,
	>(
		params: GetLogsParameters<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>,
	): Promise<GetLogsReturnType<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>> {
		type ResultType = GetLogsReturnType<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>

		const groupOf = (fulfilled: ReadonlyArray<{ idx: number; value: ResultType }>) => {
			const groups = new Map<string, { result: ResultType; providerIdxs: number[] }>()
			for (const { idx, value } of fulfilled) {
				const key = canonicalizeLogs(value)
				const existing = groups.get(key)
				if (existing) existing.providerIdxs.push(idx)
				else groups.set(key, { result: value, providerIdxs: [idx] })
			}
			return groups
		}

		const { queried, skipped, allBenched } = this.participants()
		return this.settleUntilQuorum(
			queried.map(({ idx, client }) => ({
				idx,
				task: client.getLogs<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>(
					params,
				) as Promise<ResultType>,
			})),
			// A quorum-meeting group is unique (it contains a BFT majority, and
			// two disjoint groups can't both hold one), so the first satisfiable
			// agreement is final — no need to wait out stragglers that could
			// only join or lose.
			(fulfilled, threshold) => {
				for (const group of groupOf(fulfilled).values()) {
					if (group.providerIdxs.length >= threshold) return group.result
				}
				return undefined
			},
			(fulfilled, failures, threshold) => {
				const groups = groupOf(fulfilled)
				for (const group of groups.values()) {
					if (group.providerIdxs.length >= threshold) return group.result
				}
				const responders = [...groups.values()].reduce((n, g) => n + g.providerIdxs.length, 0)
				throw new QuorumError(
					`Quorum not reached for getLogs(${describeLogsParams(params)}): no result agreed on by a ` +
						`quorum (${threshold}). ${this.describeResponders(responders, queried.length, skipped, allBenched)}. ` +
						this.formatFailures(failures.map(({ idx, error }) => ({ idx, error }))),
				)
			},
		)
	}

	/**
	 * `skipped` is the participants() snapshot for THIS call, not the suspension
	 * state at throw time — a call that rides out its deadline can outlive a
	 * bench window, and the message must explain
	 * who was excluded when the call started, not who would be excluded now.
	 */
	private describeResponders(count: number, queried: number, skipped: number, allBenched: boolean): string {
		const note = allBenched
			? ` (all ${this.size} endpoints benched; queried anyway)`
			: skipped > 0
				? ` (${skipped}/${this.size} benched)`
				: ""
		return `responders: ${count}/${queried} queried${note}`
	}

	private formatFailures(failures: readonly { idx: number; error: unknown }[]): string {
		if (failures.length === 0) return "No provider errors."
		const parts = failures.map((f) => {
			const url = f.idx >= 0 ? this.rpcUrls[f.idx] : "unknown"
			const label = isRateLimited(f.error)
				? " [rate-limited]"
				: isMalformedResponse(f.error)
					? " [malformed response]"
					: isStaleHead(f.error)
						? " [behind the head]"
						: ""
			return `${url}${label}: ${providerMessage(f.error)}`
		})
		return `Failures (${failures.length}): ${parts.join("; ")}`
	}
}

/**
 * The provider's own complaint, not viem's wrapper.
 *
 * viem maps every JSON-RPC code to a fixed sentence — a `-32602` always reads
 * "Invalid parameters were provided to the RPC method", whether the endpoint
 * objected to a block range, a topic count or an address. What it actually said
 * is on `details`, so walk the cause chain and take the innermost one.
 */
function providerMessage(error: unknown): string {
	if (!(error instanceof Error)) return String(error)
	let detail: string | undefined
	let current: unknown = error
	for (let depth = 0; depth < 6 && current instanceof Error; depth++) {
		const e = current as { details?: string; cause?: unknown }
		if (typeof e.details === "string" && e.details.trim()) detail = e.details.trim()
		current = e.cause
	}
	return detail ? `${error.message} (${detail})` : error.message
}

/** The parts of a getLogs request an operator needs to see when one is rejected. */
function describeLogsParams(params: { address?: unknown; fromBlock?: unknown; toBlock?: unknown }): string {
	const address = Array.isArray(params.address)
		? `${params.address.length} addresses`
		: String(params.address ?? "any")
	const from = params.fromBlock
	const to = params.toBlock
	// The span is the usual culprit: public endpoints cap eth_getLogs ranges well
	// below the 1000 blocks the scanner asks for when it is catching up.
	const span = typeof from === "bigint" && typeof to === "bigint" ? `, ${to - from + 1n} blocks` : ""
	return `${address}, ${String(from ?? "?")}..${String(to ?? "?")}${span}`
}

/** One endpoint's receipt answer for {@link aggregateConfirmations}. */
export interface ReceiptView {
	blockHash: string
	blockNumber: bigint
	head: bigint
}

/**
 * BFT aggregation for {@link QuorumPublicClient.getTransactionConfirmations}.
 *
 * Groups the receipt-holders by `(blockHash, blockNumber)` and looks for a group
 * of at least `quorum` endpoints agreeing on that inclusion. The depth is then
 * the group's *quorum head*: the highest block that `quorum` members of the
 * group have all indexed (mirroring {@link QuorumPublicClient.getBlockNumber}),
 * so the count never advances on fewer endpoints than the quorum bound.
 *
 * Returns `null` when no inclusion reaches the quorum. Exported for tests.
 */
export function aggregateConfirmations(views: readonly ReceiptView[], quorum: number): bigint | null {
	const groups = new Map<string, ReceiptView[]>()
	for (const view of views) {
		const key = `${view.blockHash}:${view.blockNumber}`
		const group = groups.get(key)
		if (group) group.push(view)
		else groups.set(key, [view])
	}

	const desc = (a: bigint, b: bigint) => (a > b ? -1 : a < b ? 1 : 0)
	for (const group of groups.values()) {
		if (group.length < quorum) continue
		const heads = group.map((v) => v.head).sort(desc)
		const head = heads[quorum - 1]
		const receiptBlock = group[0].blockNumber
		const confirmations = head - receiptBlock + 1n
		return confirmations > 0n ? confirmations : 0n
	}
	return null
}

/**
 * Minimum structural shape the canonicaliser needs. Every viem `Log` variant —
 * whether decoded against an ABI or not, strict or loose, pending or mined —
 * satisfies this constraint, so the generic parameter lets us accept the concrete
 * `GetLogsReturnType<...>` element type without widening or casting.
 */
interface ComparableLog {
	address: string
	blockHash: string | null
	blockNumber: bigint | null
	data: string
	logIndex: number | null
	removed: boolean
	transactionHash: string | null
	transactionIndex: number | null
	readonly topics: readonly string[]
}

/**
 * Produces a stable, order-invariant string representation of a log batch.
 *
 * Only the consensus-relevant JSON-RPC fields of each log are included —
 * `address`, `blockHash`, `blockNumber`, `data`, `logIndex`, `removed`, `topics`,
 * `transactionHash`, `transactionIndex`. Provider-added extras such as
 * `blockTimestamp` (not part of the JSON-RPC spec, but returned by some nodes)
 * are deliberately dropped, because two providers that agree on the actual event
 * state must still be considered in quorum even if one attaches debug metadata
 * that the other does not.
 *
 * Logs are sorted by (blockNumber, logIndex, transactionHash) before serialising
 * so providers that return the same events in different order still produce an
 * identical key. BigInt values are emitted as strings through the JSON replacer
 * because `JSON.stringify` would otherwise throw.
 */
function canonicalizeLogs(logs: readonly unknown[]): string {
	const projected = logs.map((log) => projectForComparison(log as ComparableLog))
	projected.sort(compareLogs)
	return JSON.stringify(projected, bigIntReplacer)
}

function projectForComparison(log: ComparableLog) {
	return {
		address: log.address.toLowerCase(),
		blockHash: log.blockHash,
		blockNumber: log.blockNumber,
		data: log.data,
		logIndex: log.logIndex,
		removed: log.removed,
		topics: log.topics,
		transactionHash: log.transactionHash,
		transactionIndex: log.transactionIndex,
	}
}

function compareLogs(a: ReturnType<typeof projectForComparison>, b: ReturnType<typeof projectForComparison>): number {
	const aBlock = a.blockNumber ?? -1n
	const bBlock = b.blockNumber ?? -1n
	if (aBlock !== bBlock) return aBlock < bBlock ? -1 : 1

	const aIdx = a.logIndex ?? -1
	const bIdx = b.logIndex ?? -1
	if (aIdx !== bIdx) return aIdx - bIdx

	const aTx = a.transactionHash ?? ""
	const bTx = b.transactionHash ?? ""
	if (aTx !== bTx) return aTx < bTx ? -1 : 1
	return 0
}

function bigIntReplacer(_key: string, value: unknown): unknown {
	return typeof value === "bigint" ? value.toString() : value
}

export class QuorumError extends Error {
	public readonly cause?: unknown
	constructor(message: string, cause?: unknown) {
		super(message)
		this.name = "QuorumError"
		if (cause !== undefined) {
			this.cause = cause
		}
	}
}
