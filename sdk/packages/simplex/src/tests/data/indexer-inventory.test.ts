import { describe, expect, it, vi } from "vitest"
import { IndexerInventory } from "@/data/indexer-inventory"
import { LoggerContext } from "@/services/Logger"

const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const USDC_BSC = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d"
const CNGN = "0x46c85152bfe9f96829aa94755d9f915f9b10ef5f"
const UNKNOWN = "0x00000000000000000000000000000000000000ff"
const SOLVER = "0xCE319986ca4d5d0893751A628D0DB3dC8FC91d62"

const NOW = Date.parse("2026-10-05T12:00:00Z")
const DAY = 86_400_000

/** USDC has six decimals on Base and eighteen on BNB Chain, which is why balances cannot be added raw. */
const tokens: Record<string, { symbol: string; decimals: number }> = {
	[USDC_BASE]: { symbol: "USDC", decimals: 6 },
	[USDC_BSC]: { symbol: "USDC", decimals: 18 },
	[CNGN]: { symbol: "cNGN", decimals: 6 },
}

type Row = { chain: string; tokenAddress: string; balance: string }

/** An indexer that answers each aliased query with the rows `rowsAt` gives for its moment. */
function indexer(rowsAt: (instant: number) => Row[] | null) {
	const requests: Array<{ query: string; variables: { solver: string } }> = []
	const fetch = vi.fn(async (_url: unknown, init?: RequestInit) => {
		const body = JSON.parse(String(init?.body))
		requests.push(body)
		const data: Record<string, { nodes: Row[] } | null> = {}
		for (const match of body.query.matchAll(/(r\d+): solverInventories\([^)]*blockHeight: "(\d+)"/g)) {
			const rows = rowsAt(Number(match[2]))
			data[match[1]] = rows ? { nodes: rows } : null
		}
		return new Response(JSON.stringify({ data }))
	})
	const reader = new IndexerInventory({
		indexerUrl: "https://indexer.test/",
		solver: SOLVER,
		describeToken: async (_chain, token) => tokens[token.toLowerCase()] ?? { symbol: null, decimals: null },
		logger: new LoggerContext({ level: "error" }).get("inventory"),
		fetch: fetch as unknown as typeof globalThis.fetch,
	})
	return { reader, fetch, requests }
}

const held: Row[] = [
	{ chain: "EVM-8453", tokenAddress: USDC_BASE, balance: "250000000" },
	{ chain: "EVM-56", tokenAddress: USDC_BSC, balance: "750000000000000000000" },
	{ chain: "EVM-8453", tokenAddress: CNGN, balance: "1500000000000" },
	{ chain: "EVM-8453", tokenAddress: UNKNOWN, balance: "42" },
]

describe("IndexerInventory", () => {
	it("asks for every instant in one request, each at its own moment in history", async () => {
		const { reader, requests } = indexer(() => held)
		const earlier = NOW - 3 * DAY

		await reader.at([earlier, NOW], NOW)

		expect(requests).toHaveLength(1)
		// The solver is filed under its lowercased address, and history is kept by millisecond.
		expect(requests[0].variables).toEqual({ solver: SOLVER.toLowerCase() })
		expect(requests[0].query).toContain(`blockHeight: "${earlier}"`)
		expect(requests[0].query).toContain(`blockHeight: "${NOW}"`)
		expect(requests[0].query).toContain("filter: { solver: { equalTo: $solver } }")
	})

	it("adds a token up across chains in whole tokens, and says which chains it counted", async () => {
		const { reader } = indexer(() => held)

		const [reading] = await reader.at([NOW], NOW)

		// 250 USDC at six decimals on Base and 750 at eighteen on BNB Chain.
		expect(reading.balances).toEqual({ USDC: 1000, cNGN: 1_500_000 })
		expect(reading.chains).toEqual({ USDC: ["EVM-8453", "EVM-56"], cNGN: ["EVM-8453"] })
		// A token the filler cannot name is left out rather than guessed at.
		expect(Object.keys(reading.balances)).not.toContain(UNKNOWN)
	})

	it("reads a moment before the solver was followed as holding nothing it can compare", async () => {
		const { reader } = indexer(() => [])

		expect(await reader.at([NOW - 30 * DAY], NOW)).toEqual([{ at: NOW - 30 * DAY, balances: {}, chains: {} }])
	})

	it("keeps a settled reading, and asks again only for a moment still within the indexer's lag", async () => {
		const { reader, requests } = indexer(() => held)
		const settled = NOW - DAY

		await reader.at([settled, NOW], NOW)
		const again = await reader.at([settled, NOW], NOW)

		expect(again).toHaveLength(2)
		expect(requests).toHaveLength(2)
		// The second request leaves out the reading it already has.
		expect(requests[1].query).not.toContain(`blockHeight: "${settled}"`)
		expect(requests[1].query).toContain(`blockHeight: "${NOW}"`)
	})

	it("splits a long list of instants over several requests", async () => {
		const { reader, requests } = indexer(() => held)
		const instants = Array.from({ length: 61 }, (_, day) => NOW - day * DAY)

		expect(await reader.at(instants, NOW)).toHaveLength(61)
		expect(requests).toHaveLength(2)
	})

	it("answers with what it has when the indexer cannot be reached", async () => {
		const { reader, fetch } = indexer(() => held)
		const settled = NOW - DAY
		await reader.at([settled], NOW)

		fetch.mockRejectedValueOnce(new Error("offline"))
		const readings = await reader.at([settled, NOW], NOW)

		// The summary goes on with the reading that was kept and without the one that was not.
		expect(readings.map((reading) => reading.at)).toEqual([settled])
	})

	it("treats an error from the indexer as no answer, not as a failure", async () => {
		const reader = new IndexerInventory({
			indexerUrl: "https://indexer.test/",
			solver: SOLVER,
			describeToken: async () => ({ symbol: null, decimals: null }),
			logger: new LoggerContext({ level: "error" }).get("inventory"),
			fetch: (async () =>
				new Response(
					JSON.stringify({ errors: [{ message: "statement timeout" }] }),
				)) as unknown as typeof fetch,
		})

		await expect(reader.at([NOW], NOW)).resolves.toEqual([])
	})

	it("leaves out an instant the indexer gave no answer for", async () => {
		const { reader } = indexer((instant) => (instant === NOW ? held : null))

		const readings = await reader.at([NOW - DAY, NOW], NOW)
		expect(readings.map((reading) => reading.at)).toEqual([NOW])
	})
})
