;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }

const records = new Map<string, any>()
const rows = (entity: string) =>
	[...records.entries()].filter(([key]) => key.startsWith(`${entity}:`)).map(([, row]) => row)
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => records.set(`${entity}:${id}`, { ...props })),
	getByFields: jest.fn(async (entity: string, filters: [string, string, any][], options: any) =>
		rows(entity)
			.filter((row) => filters.every(([field, , value]) => row[field] === value))
			.sort((a, b) => a.id.localeCompare(b.id))
			.slice(options.offset ?? 0, (options.offset ?? 0) + options.limit),
	),
}

jest.mock("@/constants", () => ({ ENV_CONFIG: {} }))
jest.mock("@/utils/safeFetch", () => ({ safeFetch: jest.fn() }))
jest.mock("@/yield-vault-addresses", () => ({
	YIELD_VAULT_ADDRESSES: {
		"EVM-8453": { "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913": [] },
		"EVM-42161": { "0xaf88d065e77c8cc2239327c5edb3a432268e5831": [] },
	},
}))

import {
	FETCH_TIMEOUT_MS,
	LIVE_SLACK_MS,
	MAX_WATCHLIST_PER_CHAIN,
	pollSolverWatchlist,
	resetSolverWatchlistPoll,
} from "@/services/solverWatchlist.service"
import { safeFetch } from "@/utils/safeFetch"

const fetchMock = jest.mocked(safeFetch)
const URL = "https://orderbook.example/solvers"
const NOW = Date.parse("2026-09-14T12:00:00Z")
const A = `0x${"a".repeat(40)}`
const B = `0x${"b".repeat(40)}`
const C = `0x${"c".repeat(40)}`

function respond(body: unknown, { status = 200, etag }: { status?: number; etag?: string } = {}) {
	fetchMock.mockResolvedValueOnce({
		ok: status >= 200 && status < 300,
		status,
		statusText: "",
		headers: etag ? { etag } : {},
		json: async () => body,
		text: async () => JSON.stringify(body),
	})
}

const watchlist = (chains: Record<string, string[]>) => ({
	chains: Object.entries(chains).map(([chain, solvers]) => ({
		chain,
		solvers: solvers.map((address) => ({ address })),
	})),
})

const poll = (blockNumber = 1_000n, blockTime = new Date(NOW - 6_000)) =>
	pollSolverWatchlist({ url: URL, blockNumber, blockTime, now: NOW })

const requests = (chain = "EVM-8453") =>
	rows("SolverDiscoveryRequest")
		.filter((row) => row.chain === chain)
		.map((row) => [row.solver, row.version])
		.sort()

beforeEach(() => {
	records.clear()
	jest.clearAllMocks()
	fetchMock.mockReset()
	resetSolverWatchlistPoll()
})

test("does not poll while the node is replaying history", async () => {
	for (let i = 0; i < 1_000; i++) {
		await poll(BigInt(i), new Date(NOW - LIVE_SLACK_MS - 6_000 * (1_000 - i)))
	}
	await pollSolverWatchlist({ url: URL, blockNumber: 1n, blockTime: undefined, now: NOW })

	expect(fetchMock).not.toHaveBeenCalled()
})

test("does nothing when no watchlist URL is configured", async () => {
	await pollSolverWatchlist({ blockNumber: 1n, blockTime: new Date(NOW), now: NOW })

	expect(fetchMock).not.toHaveBeenCalled()
})

test("an unreachable orderbook is logged, never thrown, and the next block retries", async () => {
	fetchMock.mockRejectedValueOnce(new Error("connect ECONNREFUSED"))
	await expect(poll(1_000n)).resolves.toBeUndefined()
	expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("ECONNREFUSED"))

	respond({}, { status: 503 })
	await expect(poll(1_001n)).resolves.toBeUndefined()

	respond(watchlist({ "EVM-8453": [A] }))
	await poll(1_002n)

	expect(fetchMock).toHaveBeenCalledTimes(3)
	expect(fetchMock).toHaveBeenLastCalledWith(URL, expect.objectContaining({ timeoutMs: FETCH_TIMEOUT_MS }))
	expect(requests()).toEqual([[A, 1]])
})

test("queues unseen solvers as one versioned batch per chain, skipping chains it does not track", async () => {
	respond(watchlist({ "EVM-8453": [A, B.toUpperCase().replace("0X", "0x")], "EVM-42161": [A], "EVM-999": [C] }))
	await poll(1_000n)

	expect(requests()).toEqual([
		[A, 1],
		[B, 1],
	])
	expect(requests("EVM-42161")).toEqual([[A, 1]])
	expect(requests("EVM-999")).toEqual([])
	expect(records.get("SolverWatchlist:EVM-8453")).toMatchObject({ version: 1, requestCount: 2, blockNumber: 1_000n })

	respond(watchlist({ "EVM-8453": [A, B, C], "EVM-42161": [A] }))
	await poll(1_001n)

	expect(requests()).toEqual([
		[A, 1],
		[B, 1],
		[C, 2],
	])
	expect(records.get("SolverWatchlist:EVM-8453")).toMatchObject({ version: 2, requestCount: 3 })
	// Nothing new for this chain, so its version stands.
	expect(records.get("SolverWatchlist:EVM-42161")).toMatchObject({ version: 1 })
})

test("sends the last ETag and treats 304 as unchanged", async () => {
	respond(watchlist({ "EVM-8453": [A] }), { etag: '"v1"' })
	await poll(1_000n)
	expect(fetchMock.mock.calls[0][1]?.headers).not.toHaveProperty("if-none-match")

	respond(undefined, { status: 304 })
	await poll(1_001n)

	expect(fetchMock.mock.calls[1][1]?.headers).toMatchObject({ "if-none-match": '"v1"' })
	expect(records.get("SolverWatchlist:EVM-8453")).toMatchObject({ version: 1 })
})

test("does not keep the ETag of a response it failed to apply", async () => {
	respond(watchlist({ "EVM-8453": [A] }), { etag: '"v1"' })
	jest.mocked(store.set).mockRejectedValueOnce(new Error("db down"))
	await expect(poll(1_000n)).rejects.toThrow("db down")

	respond(watchlist({ "EVM-8453": [A] }), { etag: '"v1"' })
	await poll(1_001n)

	expect(fetchMock.mock.calls[1][1]?.headers).not.toHaveProperty("if-none-match")
	expect(requests()).toEqual([[A, 1]])
})

test("caps each chain and ignores the rest with a warning", async () => {
	const solvers = Array.from(
		{ length: MAX_WATCHLIST_PER_CHAIN + 3 },
		(_, i) => `0x${i.toString(16).padStart(40, "0")}`,
	)
	respond(watchlist({ "EVM-8453": solvers }))

	await poll(1_000n)

	expect(rows("SolverDiscoveryRequest")).toHaveLength(MAX_WATCHLIST_PER_CHAIN)
	expect(logger.warn).toHaveBeenCalledWith(
		expect.stringContaining(`ignoring all past the first ${MAX_WATCHLIST_PER_CHAIN}`),
	)
})

test("drops malformed addresses and rejects a body of the wrong shape", async () => {
	respond({ chains: [{ chain: "EVM-8453", solvers: [{ address: "0x1234" }, { address: 42 }, {}, { address: A }] }] })
	await poll(1_000n)
	expect(requests()).toEqual([[A, 1]])

	respond({ solvers: [A] })
	await poll(1_001n)
	expect(logger.warn).toHaveBeenCalledWith(expect.stringContaining("unrecognised body"))
	expect(records.get("SolverWatchlist:EVM-8453")).toMatchObject({ version: 1 })
})
