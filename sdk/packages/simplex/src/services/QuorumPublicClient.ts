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
import { getLogger } from "./Logger"

/**
 * Minimum number of providers that must agree for a batch to succeed. Uses the
 * standard BFT formula `floor(2N/3) + 1` — strictly more than two thirds — so the
 * threshold tolerates up to `floor((N-1)/3)` faulty providers:
 *
 *  - N=1: 1 (no redundancy)
 *  - N=2: 2 (both must agree)
 *  - N=3: 3 (all must agree; one fault would bring the honest count to 2 ≤ 2/3·3)
 *  - N=4: 3 (tolerates 1 fault)
 *  - N=5: 4 (tolerates 1 fault)
 *  - N=7: 5 (tolerates 2 faults)
 *
 * Exported for documentation / diagnostics; not part of a public contract.
 */
export function quorumThreshold(numProviders: number): number {
	return Math.floor((2 * numProviders) / 3) + 1
}

/** How long an endpoint that errors (anything other than a 429) is paused. */
const PAUSE_MS = 5 * 60_000

/** Whether an error is an HTTP 429 rate limit (checked through viem's cause chain). */
function isRateLimited(error: unknown): boolean {
	let current: unknown = error
	for (let depth = 0; depth < 5 && current instanceof Error; depth++) {
		if ((current as { status?: number }).status === 429) return true
		if (/\b429\b|too many requests/i.test(current.message)) return true
		current = (current as { cause?: unknown }).cause
	}
	return false
}

/**
 * Wraps multiple `PublicClient`s — one per configured RPC URL — and runs selected
 * read paths (`getLogs`, `getBlockNumber`, `getTransactionConfirmations`) as a
 * Byzantine-fault-tolerant quorum.
 *
 * Every call fires all non-paused providers in parallel and requires more than
 * two thirds of the *responders* to agree (see {@link quorumThreshold}).
 * Failures are handled by kind:
 *
 *  - **HTTP 429 (rate limit)** — the endpoint is healthy, just throttling us:
 *    it is excluded from that call's quorum and nothing else.
 *  - **Any other error** (timeout, 5xx, connection refused) — the endpoint is
 *    unhealthy: it is paused for {@link PAUSE_MS} so we stop wasting requests
 *    on it, then retried.
 *
 * One floor applies: the first `operatorCount` URLs are the operator's own
 * endpoints, and the per-call threshold never drops below what that set alone
 * would require (`quorumThreshold(operatorCount)`) — so missing providers can
 * never shrink the guarantee below the operator's configuration. A provider
 * that *answers* with divergent data is never excluded: it stays in the count
 * and simply gets outvoted, which is what makes lying detectable.
 *
 * The constructor validates that URLs resolve to distinct hostnames so a "quorum"
 * isn't secretly the same upstream in disguise.
 */
export class QuorumPublicClient {
	public readonly clients: PublicClient[]
	public readonly rpcUrls: string[]
	/** BFT threshold over the FULL provider set (the maximum a call can require). */
	public readonly threshold: number
	/** Number of leading URLs that are the operator's own (sets the threshold floor). */
	public readonly operatorCount: number
	/** Minimum agreement any call requires: the operator set's own BFT threshold. */
	private readonly operatorFloor: number
	/** Per-provider pause-until timestamps (ms); 0 = active. */
	private readonly pausedUntil: number[]
	private logger = getLogger("quorum")

	constructor(chainId: number, rpcUrls: string[], operatorCount?: number) {
		this.rpcUrls = validateRpcUrls(rpcUrls)
		this.operatorCount = Math.min(Math.max(operatorCount ?? rpcUrls.length, 1), this.rpcUrls.length)
		this.operatorFloor = quorumThreshold(this.operatorCount)
		this.threshold = quorumThreshold(this.rpcUrls.length)
		this.pausedUntil = this.rpcUrls.map(() => 0)
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

	/** Per-call threshold: BFT over the responders, floored at the operator set's own bound. */
	private thresholdFor(responderCount: number): number {
		return Math.max(quorumThreshold(responderCount), this.operatorFloor)
	}

	/** Indices of providers not currently paused. */
	private activeIndices(): number[] {
		const now = Date.now()
		const active: number[] = []
		for (let idx = 0; idx < this.clients.length; idx++) {
			if (now >= this.pausedUntil[idx]) active.push(idx)
		}
		return active
	}

	/**
	 * Applies the failure policy after a batch: 429s are call-local noise;
	 * anything else pauses the endpoint for {@link PAUSE_MS}.
	 */
	private handleFailures(failures: readonly { idx: number; error: unknown }[]): void {
		const now = Date.now()
		for (const { idx, error } of failures) {
			if (isRateLimited(error)) continue
			this.pausedUntil[idx] = now + PAUSE_MS
			this.logger.warn(
				{ url: this.rpcUrls[idx], pauseMs: PAUSE_MS, error: error instanceof Error ? error.message : String(error) },
				"RPC endpoint errored — paused",
			)
		}
	}

	/**
	 * Highest block head that a BFT threshold of responding providers have
	 * indexed. Throws a {@link QuorumError} when too few respond to satisfy the
	 * operator-floored threshold.
	 */
	async getBlockNumber(): Promise<bigint> {
		const indices = this.activeIndices()
		const settled = await Promise.allSettled(indices.map((idx) => this.clients[idx].getBlockNumber()))

		const successes: bigint[] = []
		const failures: { idx: number; error: unknown }[] = []
		for (let i = 0; i < settled.length; i++) {
			const outcome = settled[i]
			if (outcome.status === "fulfilled") {
				successes.push(outcome.value)
			} else {
				failures.push({ idx: indices[i], error: outcome.reason })
			}
		}
		this.handleFailures(failures)

		const threshold = this.thresholdFor(successes.length)
		if (successes.length < threshold) {
			throw new QuorumError(
				`Quorum not reached for getBlockNumber: only ${successes.length}/${indices.length} ` +
					`providers responded (need ${threshold}). ${this.formatFailures(failures)}`,
			)
		}

		// Highest block at which ≥threshold providers have indexed: sort descending
		// and take the threshold-th value. With heads [120, 118, 117, 115, 110] and
		// threshold=3, this picks 117 — three providers (120, 118, 117) all have
		// heads ≥ 117.
		const descending = [...successes].sort((a, b) => (a > b ? -1 : a < b ? 1 : 0))
		return descending[threshold - 1]
	}

	/**
	 * Confirmation count for a transaction, counted with BFT-quorum semantics.
	 *
	 * Every non-paused provider is asked for the transaction receipt and its chain
	 * head in parallel. At least a BFT threshold of them must agree on *where*
	 * the transaction landed — the receipt's `(blockHash, blockNumber)` pair — so
	 * a minority of providers serving a reorged or fabricated inclusion cannot
	 * influence the count. The head used for counting is the highest block that
	 * at least the threshold of *agreeing* providers have indexed, mirroring
	 * {@link getBlockNumber}. Confirmations follow viem's convention:
	 * `head - receiptBlock + 1`, floored at 0.
	 *
	 * Throws {@link QuorumError} when too few providers return an agreeing
	 * receipt — including the window where the transaction is not yet mined on a
	 * quorum of providers.
	 */
	async getTransactionConfirmations({ hash }: { hash: Hash }): Promise<bigint> {
		const indices = this.activeIndices()
		const settled = await Promise.allSettled(
			indices.map(async (idx) => {
				const client = this.clients[idx]
				const [receipt, head] = await Promise.all([
					client.getTransactionReceipt({ hash }),
					client.getBlockNumber(),
				])
				return {
					blockHash: receipt.blockHash,
					blockNumber: receipt.blockNumber,
					head,
				} satisfies ProviderReceiptView
			}),
		)

		const views: ProviderReceiptView[] = []
		const failures: { idx: number; error: unknown }[] = []
		for (let i = 0; i < settled.length; i++) {
			const outcome = settled[i]
			if (outcome.status === "fulfilled") {
				views.push(outcome.value)
			} else {
				failures.push({ idx: indices[i], error: outcome.reason })
			}
		}
		this.handleFailures(failures)

		const threshold = this.thresholdFor(views.length)
		const confirmations = aggregateConfirmations(views, threshold)
		if (confirmations === null) {
			throw new QuorumError(
				`Quorum not reached for getTransactionConfirmations(${hash}): no receipt agreed on by ` +
					`${threshold}/${indices.length} providers. ${this.formatFailures(failures)}`,
			)
		}
		return confirmations
	}

	/**
	 * Fetches logs from every active provider in parallel and returns the result
	 * once a BFT threshold of them agree. Fails with {@link QuorumError} otherwise.
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

		const indices = this.activeIndices()
		const settled = await Promise.allSettled(
			indices.map((idx) =>
				this.clients[idx].getLogs<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>(params),
			),
		)

		const groups = new Map<string, { result: ResultType; count: number; providerIdxs: number[] }>()
		const failures: { idx: number; error: unknown }[] = []
		for (let i = 0; i < settled.length; i++) {
			const outcome = settled[i]
			if (outcome.status === "fulfilled") {
				const result = outcome.value as ResultType
				const key = canonicalizeLogs(result)
				const existing = groups.get(key)
				if (existing) {
					existing.count += 1
					existing.providerIdxs.push(indices[i])
				} else {
					groups.set(key, { result, count: 1, providerIdxs: [indices[i]] })
				}
			} else {
				failures.push({ idx: indices[i], error: outcome.reason })
			}
		}
		this.handleFailures(failures)

		let winner: { result: ResultType; count: number; providerIdxs: number[] } | undefined
		let responderCount = 0
		for (const group of groups.values()) {
			responderCount += group.count
			if (!winner || group.count > winner.count) winner = group
		}

		const threshold = this.thresholdFor(responderCount)
		if (!winner || winner.count < threshold) {
			const largest = winner?.count ?? 0
			throw new QuorumError(
				`Quorum not reached for getLogs: largest agreeing group had ${largest}/${responderCount} ` +
					`responders (need ${threshold}). ${this.formatFailures(failures)}`,
			)
		}

		return winner.result
	}

	private formatFailures(failures: readonly { idx: number; error: unknown }[]): string {
		if (failures.length === 0) return "No provider errors."
		const parts = failures.map((f) => {
			const err = f.error
			const message = err instanceof Error ? err.message : String(err)
			return `${this.rpcUrls[f.idx]}: ${message}`
		})
		return `Failures (${failures.length}): ${parts.join("; ")}`
	}
}

/** One provider's answer to "where is this transaction, and how far is your head?". */
export interface ProviderReceiptView {
	blockHash: string
	blockNumber: bigint
	head: bigint
}

/**
 * BFT aggregation for {@link QuorumPublicClient.getTransactionConfirmations}.
 *
 * Groups provider views by the receipt's `(blockHash, blockNumber)` identity and
 * requires the largest group to reach `threshold`. The confirmation count is then
 * derived from the threshold-th highest head *within the agreeing group*: at
 * least `threshold` providers that agree on the inclusion block have indexed up
 * to that head, so the count never advances on the say-so of fewer providers
 * than the quorum bound.
 *
 * Returns `null` when no receipt identity reaches the threshold (the caller
 * turns this into a {@link QuorumError}). Exported for unit testing.
 */
export function aggregateConfirmations(views: readonly ProviderReceiptView[], threshold: number): bigint | null {
	const groups = new Map<string, ProviderReceiptView[]>()
	for (const view of views) {
		const key = `${view.blockHash}:${view.blockNumber}`
		const group = groups.get(key)
		if (group) {
			group.push(view)
		} else {
			groups.set(key, [view])
		}
	}

	let winner: ProviderReceiptView[] | undefined
	for (const group of groups.values()) {
		if (!winner || group.length > winner.length) winner = group
	}
	if (!winner || winner.length < threshold) return null

	const headsDescending = winner.map((v) => v.head).sort((a, b) => (a > b ? -1 : a < b ? 1 : 0))
	const quorumHead = headsDescending[threshold - 1]
	const receiptBlock = winner[0].blockNumber

	const confirmations = quorumHead - receiptBlock + 1n
	return confirmations > 0n ? confirmations : 0n
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
 *
 * Accepts `readonly unknown[]` because TypeScript cannot structurally prove that
 * an arbitrary `GetLogsReturnType<...>` — whose element type is a highly generic
 * `Log<...>` variant including discriminated-union `args`/`eventName`/`topics`
 * fields — is assignable to `readonly ComparableLog[]`, even though viem's
 * runtime shape always satisfies it. The per-log projection narrows to the
 * concrete fields we actually read.
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
