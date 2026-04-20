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
 * Wraps multiple `PublicClient`s — one per configured RPC URL — and runs selected
 * read paths (currently `getLogs` and `getBlockNumber`) as a quorum.
 *
 * For `getLogs`, every underlying client is queried in parallel. All must succeed
 * **and** return byte-identical logs; otherwise the batch fails. This protects log
 * scans from a single provider silently returning stale, filtered, or fabricated
 * events.
 *
 * For `getBlockNumber`, the minimum of all returned heads is used, so scans never
 * advance past a block that every provider has indexed.
 *
 * The constructor validates that URLs resolve to distinct hostnames so a "quorum"
 * isn't secretly the same upstream in disguise.
 */
export class QuorumPublicClient {
	public readonly clients: PublicClient[]
	public readonly rpcUrls: string[]

	constructor(chainId: number, rpcUrls: string[]) {
		this.rpcUrls = validateRpcUrls(rpcUrls)
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
	 * Lowest block head across all providers. If any provider throws, the batch throws.
	 */
	async getBlockNumber(): Promise<bigint> {
		const results = await Promise.all(this.clients.map((c) => c.getBlockNumber()))
		return results.reduce((min, n) => (n < min ? n : min), results[0])
	}

	/**
	 * Fetches logs from every provider in parallel. Fails if any provider throws or
	 * if results diverge.
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

		const resultsPerClient: ResultType[] = await Promise.all(
			this.clients.map(async (client, idx): Promise<ResultType> => {
				try {
					return await client.getLogs<TAbiEvent, TAbiEvents, TStrict, TFromBlock, TToBlock>(params)
				} catch (error) {
					throw new QuorumError(
						`getLogs failed on provider ${this.rpcUrls[idx]}: ${(error as Error).message ?? error}`,
						error,
					)
				}
			}),
		)

		if (resultsPerClient.length === 1) {
			return resultsPerClient[0]
		}

		const canonical = canonicalizeLogs(resultsPerClient[0])
		for (let i = 1; i < resultsPerClient.length; i++) {
			const other = canonicalizeLogs(resultsPerClient[i])
			if (canonical !== other) {
				throw new QuorumError(
					`Quorum mismatch for getLogs: providers ${this.rpcUrls[0]} and ${this.rpcUrls[i]} returned different logs`,
				)
			}
		}

		return resultsPerClient[0]
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
 */
function canonicalizeLogs<T extends ComparableLog>(logs: readonly T[]): string {
	const projected = logs.map(projectForComparison)
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
