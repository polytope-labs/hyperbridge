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
 * Whether an error signals rate limiting — HTTP 429 (`HttpRequestError` with
 * `status: 429`) or an HTTP-200 JSON-RPC rate-limit error (`RpcRequestError`,
 * code/message). Used only to label excluded providers in diagnostics; it does
 * not change control flow (a rate-limited provider is excluded from the call
 * exactly like any other non-responder). Exported for tests.
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

/** Whether a getTransactionReceipt rejection means "no such receipt" (a valid answer, not a fault). */
function isReceiptNotFound(error: unknown): boolean {
	return error instanceof Error && error.name === "TransactionReceiptNotFoundError"
}

/**
 * Wraps multiple viem `PublicClient`s — one per configured RPC URL — and runs
 * selected read paths (`getLogs`, `getBlockNumber`, `getTransactionConfirmations`)
 * as a **BFT quorum**: a result is accepted only when `quorumThreshold(n)` of the
 * operator's endpoints agree on it.
 *
 * A provider that *answers with divergent data* is never special-cased away — it
 * simply fails to join the agreeing group, which is what makes a lying or reorged
 * endpoint detectable rather than authoritative.
 *
 * There is no pausing/ejection state: each call independently queries every
 * endpoint and forms the quorum from whoever answers. A chronically slow
 * endpoint costs one failed sub-request per call, never a shrunk quorum.
 *
 * The constructor validates that URLs resolve to distinct hostnames so a "quorum"
 * isn't secretly the same upstream in disguise.
 */
export class QuorumPublicClient {
	public readonly clients: PublicClient[]
	public readonly rpcUrls: string[]
	/** Endpoints that must agree on any result: `quorumThreshold(n)`. */
	public readonly threshold: number

	constructor(chainId: number, rpcUrls: string[]) {
		this.rpcUrls = validateRpcUrls(rpcUrls)
		this.threshold = quorumThreshold(this.rpcUrls.length)
		const chain = getViemChain(chainId) as Chain
		this.clients = this.rpcUrls.map((url) =>
			createPublicClient({
				chain,
				transport: http(url, {
					timeout: 30_000,
					retryCount: 3,
					retryDelay: 1000,
				}),
			}),
		)
	}

	get size(): number {
		return this.clients.length
	}

	/**
	 * Highest block a quorum of endpoints have all indexed: the highest B such
	 * that `threshold` endpoints report head ≥ B. Returns null when fewer than
	 * `threshold` endpoints have responded.
	 */
	private quorumHead(heads: readonly bigint[]): bigint | null {
		if (heads.length < this.threshold) return null
		const desc = (a: bigint, b: bigint) => (a > b ? -1 : a < b ? 1 : 0)
		return [...heads].sort(desc)[this.threshold - 1]
	}

	/**
	 * Settles per-client tasks incrementally and resolves the moment the
	 * evaluator can decide from the responses seen so far — the hot path never
	 * blocks on the slowest provider (a hung endpoint with a 30s timeout × 3
	 * retries would otherwise stall every confirmation poll). Stragglers settle
	 * out of band and are ignored. `tryDecide` returns undefined to keep
	 * waiting; once every task has settled, `finalize` must decide (it throws
	 * the QuorumError when the full response set still has no quorum). Early
	 * exit never weakens the trust model — a decision still requires the same
	 * quorum, it just doesn't wait for votes it no longer needs.
	 */
	private settleUntilQuorum<T, R>(
		tasks: Promise<T>[],
		tryDecide: (fulfilled: ReadonlyArray<{ idx: number; value: T }>) => R | undefined,
		finalize: (
			fulfilled: ReadonlyArray<{ idx: number; value: T }>,
			failures: ReadonlyArray<{ idx: number; error: unknown }>,
		) => R,
	): Promise<R> {
		return new Promise<R>((resolve, reject) => {
			const fulfilled: Array<{ idx: number; value: T }> = []
			const failures: Array<{ idx: number; error: unknown }> = []
			let outstanding = tasks.length
			let done = false

			const evaluate = () => {
				if (done) return
				try {
					const early = tryDecide(fulfilled)
					if (early !== undefined) {
						done = true
						resolve(early)
						return
					}
					if (outstanding === 0) {
						done = true
						resolve(finalize(fulfilled, failures))
					}
				} catch (error) {
					done = true
					reject(error)
				}
			}

			if (outstanding === 0) {
				evaluate()
				return
			}
			tasks.forEach((task, idx) => {
				task.then(
					(value) => {
						outstanding--
						fulfilled.push({ idx, value })
						evaluate()
					},
					(error) => {
						outstanding--
						failures.push({ idx, error })
						evaluate()
					},
				)
			})
		})
	}

	/**
	 * Highest block head backed by a full quorum. Throws {@link QuorumError}
	 * when fewer than `threshold` endpoints respond.
	 */
	async getBlockNumber(): Promise<bigint> {
		return this.settleUntilQuorum(
			this.clients.map((c) => c.getBlockNumber()),
			// Early exit as soon as a quorum is satisfiable. Late voters could
			// only raise the reported head; the earlier (lower) head is
			// conservative for every consumer (fewer confirmations counted,
			// smaller scan windows).
			(fulfilled) => this.quorumHead(fulfilled.map((f) => f.value)) ?? undefined,
			(fulfilled, failures) => {
				const head = this.quorumHead(fulfilled.map((f) => f.value))
				if (head === null) {
					throw new QuorumError(
						`Quorum not reached for getBlockNumber: ${this.describeResponders(fulfilled.length)}. ` +
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

		return this.settleUntilQuorum(
			this.clients.map(async (client): Promise<ReceiptProbe> => {
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
			}),
			// Early exit only on a POSITIVE quorum: an agreeing inclusion group can
			// only grow, so the first satisfiable quorum is final. Not-found votes
			// never decide early — "not confirmed" needs the full response set.
			(fulfilled) => aggregateConfirmations(toViews(fulfilled), this.threshold) ?? undefined,
			(fulfilled, failures) => {
				const confirmations = aggregateConfirmations(toViews(fulfilled), this.threshold)
				if (confirmations === null) {
					throw new QuorumError(
						`Quorum not reached for getTransactionConfirmations(${hash}): no inclusion agreed on by a ` +
							`quorum (${this.threshold}). ${this.describeResponders(fulfilled.length)}. ` +
							this.formatFailures(failures.map(({ error }) => ({ idx: -1, error }))),
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

		return this.settleUntilQuorum(
			this.clients.map((client) =>
				client.getLogs<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>(params),
			) as Promise<ResultType>[],
			// A quorum-meeting group is unique (it contains a BFT majority, and
			// two disjoint groups can't both hold one), so the first satisfiable
			// agreement is final — no need to wait out stragglers that could
			// only join or lose.
			(fulfilled) => {
				for (const group of groupOf(fulfilled).values()) {
					if (group.providerIdxs.length >= this.threshold) return group.result
				}
				return undefined
			},
			(fulfilled, failures) => {
				const groups = groupOf(fulfilled)
				for (const group of groups.values()) {
					if (group.providerIdxs.length >= this.threshold) return group.result
				}
				const responders = [...groups.values()].reduce((n, g) => n + g.providerIdxs.length, 0)
				throw new QuorumError(
					`Quorum not reached for getLogs(${describeLogsParams(params)}): no result agreed on by a ` +
						`quorum (${this.threshold}). ${this.describeResponders(responders)}. ` +
						this.formatFailures(failures.map(({ idx, error }) => ({ idx, error }))),
				)
			},
		)
	}

	private describeResponders(count: number): string {
		return `responders: ${count}/${this.size}`
	}

	private formatFailures(failures: readonly { idx: number; error: unknown }[]): string {
		if (failures.length === 0) return "No provider errors."
		const parts = failures.map((f) => {
			const url = f.idx >= 0 ? this.rpcUrls[f.idx] : "unknown"
			const label = isRateLimited(f.error) ? " [rate-limited]" : ""
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
function describeLogsParams(params: {
	address?: unknown
	fromBlock?: unknown
	toBlock?: unknown
}): string {
	const address = Array.isArray(params.address) ? `${params.address.length} addresses` : String(params.address ?? "any")
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
