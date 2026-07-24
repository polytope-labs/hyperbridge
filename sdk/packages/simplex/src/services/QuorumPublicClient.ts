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
 * Standard BFT threshold for a set of `n` equal peers: `floor(2n/3) + 1`, i.e.
 * strictly more than two thirds, tolerating up to `floor((n-1)/3)` faults.
 *
 *  - n=1: 1   n=2: 2   n=3: 3   n=4: 3   n=5: 4   n=7: 5
 *
 * Used here for the *operator* tier only (see {@link QuorumPublicClient}); the
 * public tier uses a flat witness floor, not a BFT fraction.
 */
export function quorumThreshold(numProviders: number): number {
	return Math.floor((2 * numProviders) / 3) + 1
}

/** Public registry endpoints that must corroborate the operator's view per call. */
const REQUIRED_PUBLIC_WITNESSES = 2

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
 * Wraps multiple `PublicClient`s — one per configured RPC URL — and runs selected
 * read paths (`getLogs`, `getBlockNumber`, `getTransactionConfirmations`) as a
 * **weighted quorum with two tiers**.
 *
 * The first `operatorCount` URLs are the *operator's own* endpoints; the rest are
 * public-registry endpoints. Operator endpoints are load-bearing and public ones
 * are corroborating witnesses, so a result is accepted only when:
 *
 *  1. a BFT quorum of the operator set agrees on it — `quorumThreshold(operatorCount)`
 *     operator endpoints returning the same value — AND
 *  2. at least `min(2, publicCount)` public endpoints agree with them.
 *
 * This makes operator failures **intolerable** (if the operator endpoints can't
 * form their own quorum the call fails loudly, never proceeding on public
 * endpoints alone) while public failures are **tolerable** (any two agreeing
 * witnesses suffice; the rest may be down, throttled, or lagging). A provider
 * that *answers with divergent data* is never special-cased away — it simply
 * fails to join the agreeing group, which is what makes a lying or reorged
 * endpoint detectable rather than authoritative.
 *
 * There is no pausing/ejection state: each call independently queries every
 * endpoint and forms the quorum from whoever answers. A chronically slow public
 * endpoint costs one failed sub-request per call, never a shrunk quorum.
 *
 * The constructor validates that URLs resolve to distinct hostnames so a "quorum"
 * isn't secretly the same upstream in disguise.
 */
export class QuorumPublicClient {
	public readonly clients: PublicClient[]
	public readonly rpcUrls: string[]
	/** Number of leading URLs that are the operator's own endpoints. */
	public readonly operatorCount: number
	/** Number of trailing URLs that are public-registry endpoints. */
	public readonly publicCount: number
	/** Operator endpoints that must agree on any result (BFT over the operator set). */
	public readonly operatorQuorum: number
	/** Public endpoints that must corroborate: `min(2, publicCount)`. */
	public readonly requiredPublic: number
	/** Minimum agreeing endpoints for any call (operatorQuorum + requiredPublic). Diagnostic. */
	public readonly threshold: number

	private logger = getLogger("quorum")

	constructor(chainId: number, rpcUrls: string[], operatorCount?: number) {
		this.rpcUrls = validateRpcUrls(rpcUrls)
		this.operatorCount = Math.min(Math.max(operatorCount ?? rpcUrls.length, 1), this.rpcUrls.length)
		this.publicCount = this.rpcUrls.length - this.operatorCount
		this.operatorQuorum = quorumThreshold(this.operatorCount)
		this.requiredPublic = Math.min(REQUIRED_PUBLIC_WITNESSES, this.publicCount)
		this.threshold = this.operatorQuorum + this.requiredPublic
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

	private isOperator(idx: number): boolean {
		return idx < this.operatorCount
	}

	/**
	 * Whether a set of endpoints that agree on some value satisfies both tiers:
	 * a BFT quorum of operators plus the public witness floor.
	 */
	private meetsQuorum(agreeingIdxs: readonly number[]): boolean {
		let op = 0
		let pub = 0
		for (const idx of agreeingIdxs) {
			if (this.isOperator(idx)) op++
			else pub++
		}
		return op >= this.operatorQuorum && pub >= this.requiredPublic
	}

	/**
	 * Highest block a valid quorum of endpoints have all indexed: the highest B
	 * such that `operatorQuorum` operators AND `requiredPublic` public endpoints
	 * report head ≥ B. Returns null when either tier can't be met.
	 */
	private tieredHead(heads: readonly { idx: number; head: bigint }[]): bigint | null {
		const desc = (a: bigint, b: bigint) => (a > b ? -1 : a < b ? 1 : 0)
		const opHeads = heads.filter((h) => this.isOperator(h.idx)).map((h) => h.head).sort(desc)
		const pubHeads = heads.filter((h) => !this.isOperator(h.idx)).map((h) => h.head).sort(desc)
		if (opHeads.length < this.operatorQuorum) return null
		if (pubHeads.length < this.requiredPublic) return null
		const bOp = opHeads[this.operatorQuorum - 1]
		if (this.requiredPublic === 0) return bOp
		const bPub = pubHeads[this.requiredPublic - 1]
		return bOp < bPub ? bOp : bPub
	}

	/**
	 * Highest block head backed by a full tiered quorum. Throws {@link QuorumError}
	 * when the operator or public tier cannot be met from the responders.
	 */
	async getBlockNumber(): Promise<bigint> {
		const settled = await Promise.allSettled(this.clients.map((c) => c.getBlockNumber()))

		const heads: { idx: number; head: bigint }[] = []
		const failures: { idx: number; error: unknown }[] = []
		for (let idx = 0; idx < settled.length; idx++) {
			const outcome = settled[idx]
			if (outcome.status === "fulfilled") heads.push({ idx, head: outcome.value })
			else failures.push({ idx, error: outcome.reason })
		}

		const head = this.tieredHead(heads)
		if (head === null) {
			throw new QuorumError(
				`Quorum not reached for getBlockNumber: ${this.describeResponders(heads.map((h) => h.idx))}. ` +
					this.formatFailures(failures),
			)
		}
		return head
	}

	/**
	 * Confirmation count for a transaction under the tiered quorum.
	 *
	 * Every endpoint is asked for the receipt and its head. A "receipt not found"
	 * answer is a valid **no** vote — the endpoint is responsive but does not see
	 * the transaction (not yet propagated, or reorged out) — so it counts toward
	 * responsiveness but joins no inclusion group. The transaction is confirmed
	 * only when a tiered quorum (operator BFT quorum + public witness floor) agree
	 * on the same `(blockHash, blockNumber)`; the depth is then the tiered head of
	 * that agreeing group. A minority still serving a reorged/fabricated receipt
	 * can neither reach the quorum nor, because operator agreement is mandatory,
	 * substitute for the operator's own view.
	 *
	 * Throws {@link QuorumError} when no inclusion reaches the tiered quorum —
	 * including the ordinary window where the tx is not yet mined on enough
	 * endpoints.
	 */
	async getTransactionConfirmations({ hash }: { hash: Hash }): Promise<bigint> {
		const settled = await Promise.allSettled(
			this.clients.map(async (client, idx) => {
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
				return { idx, head, receipt }
			}),
		)

		const views: ReceiptView[] = []
		const responders: number[] = []
		const failures: { idx: number; error: unknown }[] = []
		for (const outcome of settled) {
			if (outcome.status !== "fulfilled") {
				failures.push({ idx: -1, error: outcome.reason })
				continue
			}
			const { idx, head, receipt } = outcome.value
			responders.push(idx)
			if (receipt) {
				views.push({
					isOperator: this.isOperator(idx),
					blockHash: receipt.blockHash,
					blockNumber: receipt.blockNumber,
					head,
				})
			}
		}

		const confirmations = aggregateConfirmations(views, this.operatorQuorum, this.requiredPublic)
		if (confirmations === null) {
			throw new QuorumError(
				`Quorum not reached for getTransactionConfirmations(${hash}): no inclusion agreed on by an ` +
					`operator quorum (${this.operatorQuorum}) plus ${this.requiredPublic} public witness(es). ` +
					`${this.describeResponders(responders)}. ${this.formatFailures(failures)}`,
			)
		}
		return confirmations
	}

	/**
	 * Fetches logs from every endpoint in parallel and returns the result once a
	 * tiered quorum agrees. Fails with {@link QuorumError} otherwise.
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

		const settled = await Promise.allSettled(
			this.clients.map((client) => client.getLogs<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>(params)),
		)

		const groups = new Map<string, { result: ResultType; providerIdxs: number[] }>()
		const failures: { idx: number; error: unknown }[] = []
		for (let idx = 0; idx < settled.length; idx++) {
			const outcome = settled[idx]
			if (outcome.status === "fulfilled") {
				const result = outcome.value as ResultType
				const key = canonicalizeLogs(result)
				const existing = groups.get(key)
				if (existing) existing.providerIdxs.push(idx)
				else groups.set(key, { result, providerIdxs: [idx] })
			} else {
				failures.push({ idx, error: outcome.reason })
			}
		}

		// A quorum-meeting group is unique: it contains an operator BFT majority,
		// and two disjoint groups can't both hold one.
		for (const group of groups.values()) {
			if (this.meetsQuorum(group.providerIdxs)) return group.result
		}

		const responders = [...groups.values()].flatMap((g) => g.providerIdxs)
		throw new QuorumError(
			`Quorum not reached for getLogs: no result agreed on by an operator quorum (${this.operatorQuorum}) ` +
				`plus ${this.requiredPublic} public witness(es). ${this.describeResponders(responders)}. ` +
				this.formatFailures(failures),
		)
	}

	/** Human-readable split of which responders were operator vs public. */
	private describeResponders(idxs: readonly number[]): string {
		let op = 0
		let pub = 0
		for (const idx of idxs) {
			if (this.isOperator(idx)) op++
			else pub++
		}
		return `responders: ${op}/${this.operatorCount} operator, ${pub}/${this.publicCount} public`
	}

	private formatFailures(failures: readonly { idx: number; error: unknown }[]): string {
		if (failures.length === 0) return "No provider errors."
		const parts = failures.map((f) => {
			const url = f.idx >= 0 ? this.rpcUrls[f.idx] : "unknown"
			const message = f.error instanceof Error ? f.error.message : String(f.error)
			const label = isRateLimited(f.error) ? " [rate-limited]" : ""
			return `${url}${label}: ${message}`
		})
		return `Failures (${failures.length}): ${parts.join("; ")}`
	}
}

/** One endpoint's receipt answer, tagged with its tier, for {@link aggregateConfirmations}. */
export interface ReceiptView {
	isOperator: boolean
	blockHash: string
	blockNumber: bigint
	head: bigint
}

/**
 * Tiered BFT aggregation for {@link QuorumPublicClient.getTransactionConfirmations}.
 *
 * Groups the receipt-holders by `(blockHash, blockNumber)` and looks for a group
 * satisfying both tiers — `operatorQuorum` operator endpoints and `requiredPublic`
 * public endpoints agreeing on that inclusion. The depth is then the group's
 * *tiered head*: the highest block that `operatorQuorum` operators and
 * `requiredPublic` public members of the group have all indexed (mirroring
 * {@link QuorumPublicClient.getBlockNumber}), so the count never advances on
 * fewer endpoints than the quorum bound.
 *
 * Returns `null` when no inclusion reaches the tiered quorum. Exported for tests.
 */
export function aggregateConfirmations(
	views: readonly ReceiptView[],
	operatorQuorum: number,
	requiredPublic: number,
): bigint | null {
	const groups = new Map<string, ReceiptView[]>()
	for (const view of views) {
		const key = `${view.blockHash}:${view.blockNumber}`
		const group = groups.get(key)
		if (group) group.push(view)
		else groups.set(key, [view])
	}

	const desc = (a: bigint, b: bigint) => (a > b ? -1 : a < b ? 1 : 0)
	for (const group of groups.values()) {
		const opHeads = group.filter((v) => v.isOperator).map((v) => v.head).sort(desc)
		const pubHeads = group.filter((v) => !v.isOperator).map((v) => v.head).sort(desc)
		if (opHeads.length < operatorQuorum || pubHeads.length < requiredPublic) continue

		const bOp = opHeads[operatorQuorum - 1]
		const bPub = requiredPublic > 0 ? pubHeads[requiredPublic - 1] : null
		const head = bPub === null ? bOp : bOp < bPub ? bOp : bPub
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
