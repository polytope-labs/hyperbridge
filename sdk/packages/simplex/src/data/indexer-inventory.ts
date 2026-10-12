import { formatUnits } from "viem"
import type { InventoryReading } from "@/orderbook/profitability"
import type { Logger } from "@/services/Logger"
import type { TokenDescriber } from "./recorder"

/** How long one request to the indexer may take before the summary goes on without it. */
const READ_TIMEOUT_MS = 10_000
/**
 * How old a moment has to be before its reading is kept. The indexer follows finalized blocks,
 * so a reading of the last few minutes can still change as it catches up; an older one cannot.
 */
const SETTLED_AFTER_MS = 10 * 60_000
/** Instants asked for in one request. Each is one aliased query, and a month of days is sixty. */
const BATCH = 40
/** Readings kept. A year of daily snapshots and bucket starts is well under this. */
const MAX_KEPT = 2_000

/** One `SolverInventory` row: a token on a chain, wallet and vaults together, in the token's own units. */
interface InventoryRow {
	chain: string
	tokenAddress: string
	balance: string
}

export interface IndexerInventoryOptions {
	indexerUrl: string
	/** The solver's EVM address, which is what the indexer files its balances under. */
	solver: string
	/** Names a token and says how many decimals it has, so a raw balance becomes whole tokens. */
	describeToken: TokenDescriber
	logger: Logger
	fetch?: typeof fetch
}

/**
 * What the solver held at past moments, as the indexer has it.
 *
 * The indexer keeps a running balance per token and chain from every Transfer, and answers for
 * any moment since it first saw the solver. So two readings say what moved between them without
 * anything having to be recorded here: that is how a deposit or a withdrawal, which no fill or
 * snapshot shows, is accounted for.
 *
 * Best effort throughout. The indexer being slow, down or behind costs the summary nothing but
 * that knowledge, and it falls back to the fills and sends it has on record.
 */
export class IndexerInventory {
	private kept = new Map<number, InventoryReading>()
	private described = new Map<string, { symbol: string; decimals: number }>()
	private fetch: typeof fetch

	constructor(private options: IndexerInventoryOptions) {
		this.fetch = options.fetch ?? fetch
	}

	/**
	 * A reading for each of `instants` the indexer could answer for. Never rejects: an instant it
	 * could not be asked about is simply missing from the answer.
	 */
	async at(instants: readonly number[], now: number): Promise<InventoryReading[]> {
		const readings: InventoryReading[] = []
		const wanted: number[] = []
		for (const instant of new Set(instants)) {
			const kept = this.kept.get(instant)
			if (kept) readings.push(kept)
			else wanted.push(instant)
		}

		try {
			for (let index = 0; index < wanted.length; index += BATCH) {
				for (const reading of await this.read(wanted.slice(index, index + BATCH))) {
					readings.push(reading)
					if (reading.at <= now - SETTLED_AFTER_MS) this.keep(reading)
				}
			}
		} catch (err) {
			this.options.logger.warn(
				{ err: err instanceof Error ? err.message : String(err) },
				"Could not read inventory from the indexer; carrying records by fills and sends alone",
			)
		}
		return readings
	}

	private keep(reading: InventoryReading): void {
		if (this.kept.size >= MAX_KEPT) this.kept.clear()
		this.kept.set(reading.at, reading)
	}

	/** One request for several instants, each its own aliased query at that moment in history. */
	private async read(instants: readonly number[]): Promise<InventoryReading[]> {
		// The indexer keeps history by block timestamp, which is what `blockHeight` takes: milliseconds.
		const queries = instants.map(
			(instant, index) =>
				`r${index}: solverInventories(filter: { solver: { equalTo: $solver } }, blockHeight: "${Math.floor(instant)}", first: 1000) { nodes { chain tokenAddress balance } }`,
		)
		const response = await this.fetch(this.options.indexerUrl, {
			method: "POST",
			headers: { "content-type": "application/json" },
			body: JSON.stringify({
				query: `query SolverInventory($solver: String!) {\n${queries.join("\n")}\n}`,
				variables: { solver: this.options.solver.toLowerCase() },
			}),
			signal: AbortSignal.timeout(READ_TIMEOUT_MS),
		})
		if (!response.ok) throw new Error(`Indexer responded ${response.status}`)
		const body = (await response.json()) as {
			data?: Record<string, { nodes: InventoryRow[] } | null>
			errors?: Array<{ message: string }>
		}
		if (body.errors?.length) throw new Error(body.errors.map((error) => error.message).join("; "))

		const readings: InventoryReading[] = []
		for (const [index, instant] of instants.entries()) {
			const rows = body.data?.[`r${index}`]?.nodes
			if (rows) readings.push(await this.readingOf(instant, rows))
		}
		return readings
	}

	private async readingOf(at: number, rows: readonly InventoryRow[]): Promise<InventoryReading> {
		const balances: Record<string, number> = {}
		const chains: Record<string, string[]> = {}
		for (const row of rows) {
			const token = await this.describe(row.chain, row.tokenAddress)
			// A token this filler cannot name is not part of the inventory it reports.
			if (!token) continue
			let amount: number
			try {
				amount = Number(formatUnits(BigInt(row.balance), token.decimals))
			} catch {
				continue
			}
			balances[token.symbol] = (balances[token.symbol] ?? 0) + amount
			chains[token.symbol] = [...(chains[token.symbol] ?? []), row.chain]
		}
		return { at, balances, chains }
	}

	/** A token's symbol and decimals, remembered once both are known. */
	private async describe(chain: string, token: string): Promise<{ symbol: string; decimals: number } | null> {
		const key = `${chain}:${token.toLowerCase()}`
		const known = this.described.get(key)
		if (known) return known
		const { symbol, decimals } = await this.options.describeToken(chain, token)
		if (!symbol || decimals === null) return null
		const description = { symbol, decimals }
		this.described.set(key, description)
		return description
	}
}
