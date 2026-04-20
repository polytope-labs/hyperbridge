import {
	createPublicClient,
	http,
	type AbiEvent,
	type BlockNumber,
	type BlockTag,
	type Chain,
	type GetLogsParameters,
	type GetLogsReturnType,
	type PublicClient,
} from "viem"
import { getViemChain } from "@hyperbridge/sdk"
import { validateRpcUrls } from "./FillerConfigService"

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

/**
 * Wraps multiple `PublicClient`s — one per configured RPC URL — and runs selected
 * read paths (currently `getLogs` and `getBlockNumber`) as a Byzantine-fault-tolerant
 * quorum.
 *
 * A batch succeeds when **more than two thirds** of providers return the same
 * result (see {@link quorumThreshold}). Providers that time out, error, or return
 * a divergent result are tolerated up to the BFT bound; beyond that the batch
 * fails loudly — no silent fall-through to a single provider's view of the chain.
 *
 * For `getLogs`, every underlying client is queried in parallel and results are
 * grouped by a canonical serialisation. The largest agreement group must reach
 * the quorum threshold or the batch fails.
 *
 * For `getBlockNumber`, every client is queried in parallel and the returned head
 * is the highest block at which at least {@link quorumThreshold} providers have
 * indexed up to that block — so the block scanner never advances past a cursor
 * that lacks BFT-level support.
 *
 * The constructor validates that URLs resolve to distinct hostnames so a "quorum"
 * isn't secretly the same upstream in disguise.
 */
export class QuorumPublicClient {
	public readonly clients: PublicClient[]
	public readonly rpcUrls: string[]
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
	 * Highest block head that at least {@link quorumThreshold} providers have
	 * indexed. Throws a {@link QuorumError} if fewer than that many providers
	 * respond successfully.
	 */
	async getBlockNumber(): Promise<bigint> {
		const settled = await Promise.allSettled(this.clients.map((c) => c.getBlockNumber()))

		const successes: bigint[] = []
		const failures: { idx: number; error: unknown }[] = []
		for (let idx = 0; idx < settled.length; idx++) {
			const outcome = settled[idx]
			if (outcome.status === "fulfilled") {
				successes.push(outcome.value)
			} else {
				failures.push({ idx, error: outcome.reason })
			}
		}

		if (successes.length < this.threshold) {
			throw new QuorumError(
				`Quorum not reached for getBlockNumber: only ${successes.length}/${this.clients.length} ` +
					`providers succeeded (need ${this.threshold}). ${this.formatFailures(failures)}`,
			)
		}

		// Highest block at which ≥threshold providers have indexed: sort descending
		// and take the threshold-th value. With heads [120, 118, 117, 115, 110] and
		// threshold=3, this picks 117 — three providers (120, 118, 117) all have
		// heads ≥ 117.
		const descending = [...successes].sort((a, b) => (a > b ? -1 : a < b ? 1 : 0))
		return descending[this.threshold - 1]
	}

	/**
	 * Fetches logs from every provider in parallel and returns the result once a
	 * quorum — {@link quorumThreshold} providers — agree. Fails with
	 * {@link QuorumError} otherwise.
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
			this.clients.map((client) =>
				client.getLogs<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>(params),
			),
		)

		const groups = new Map<string, { result: ResultType; count: number; providerIdxs: number[] }>()
		const failures: { idx: number; error: unknown }[] = []
		for (let idx = 0; idx < settled.length; idx++) {
			const outcome = settled[idx]
			if (outcome.status === "fulfilled") {
				const result = outcome.value as ResultType
				const key = canonicalizeLogs(result)
				const existing = groups.get(key)
				if (existing) {
					existing.count += 1
					existing.providerIdxs.push(idx)
				} else {
					groups.set(key, { result, count: 1, providerIdxs: [idx] })
				}
			} else {
				failures.push({ idx, error: outcome.reason })
			}
		}

		let winner: { result: ResultType; count: number; providerIdxs: number[] } | undefined
		for (const group of groups.values()) {
			if (!winner || group.count > winner.count) winner = group
		}

		if (!winner || winner.count < this.threshold) {
			const largest = winner?.count ?? 0
			throw new QuorumError(
				`Quorum not reached for getLogs: largest agreeing group had ${largest}/${this.clients.length} ` +
					`providers (need ${this.threshold}). ${this.formatFailures(failures)}`,
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
