import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { IntentsCoprocessor, type PhantomOrderEvent } from "@/chains/intentsCoprocessor"

// Polling replaced a system.events subscription because a dropped socket silently stopped delivering
// phantom orders — polkadot-js reconnects the transport but does not reliably re-establish storage
// subscriptions, and anything emitted while disconnected was gone. The property that makes a block
// cursor an actual fix, rather than a different way to lose orders, is that it advances only past
// blocks whose events were really read, so an outage delays orders instead of dropping them. That is
// what most of these assert.

const COMMITMENT_A = `0x${"aa".repeat(32)}`
const COMMITMENT_B = `0x${"bb".repeat(32)}`
const TOKEN_A = `0x${"11".repeat(20)}`
const TOKEN_B = `0x${"22".repeat(20)}`

const CHAIN = "EVM-8453"
const CHAIN_HEX = `0x${Buffer.from(CHAIN, "utf8").toString("hex")}`

/** A PhantomOrderRegistered record shaped the way polkadot-js decodes it. */
function registeredEvent(commitment: string) {
	return {
		event: {
			section: "intentsCoprocessor",
			method: "PhantomOrderRegistered",
			data: [
				{ toHex: () => commitment },
				{ toHex: () => CHAIN_HEX },
				{ toNumber: () => 7 },
				{ toHex: () => TOKEN_A },
				{ toHex: () => TOKEN_B },
				{ toString: () => "1000000" },
			],
		},
	}
}

const unrelatedEvent = { event: { section: "balances", method: "Transfer", data: [] } }

interface Harness {
	coprocessor: IntentsCoprocessor
	/** Blocks scanned, in order. */
	scanned: number[]
	setHead: (n: number) => void
	/** Registers an order in a block; blocks without one still scan clean. */
	putOrder: (blockNumber: number, commitment: string) => void
	/** Makes the next `count` head reads throw, simulating a dropped connection. */
	failHeadReads: (count: number) => void
}

function harness(initialHead: number): Harness {
	let head = initialHead
	let headFailures = 0
	const ordersByBlock = new Map<number, string>()
	const scanned: number[] = []

	const getHeader = vi.fn(async () => {
		if (headFailures > 0) {
			headFailures -= 1
			throw new Error("websocket disconnected")
		}
		return { number: { toNumber: () => head } }
	})

	const getBlockHash = vi.fn(async (n: number) => `0xblock${n}`)

	const at = vi.fn(async (blockHash: string) => {
		const blockNumber = Number(blockHash.replace("0xblock", ""))
		scanned.push(blockNumber)
		const commitment = ordersByBlock.get(blockNumber)
		const records = commitment ? [unrelatedEvent, registeredEvent(commitment)] : [unrelatedEvent]
		return { query: { system: { events: async () => records } } }
	})

	const coprocessor = Object.create(IntentsCoprocessor.prototype) as IntentsCoprocessor
	Object.assign(coprocessor, { api: { rpc: { chain: { getHeader, getBlockHash } }, at } })

	return {
		coprocessor,
		scanned,
		setHead: (n) => {
			head = n
		},
		putOrder: (blockNumber, commitment) => ordersByBlock.set(blockNumber, commitment),
		failHeadReads: (count) => {
			headFailures = count
		},
	}
}

/** Advances fake timers and lets the awaited scan settle. */
const tick = (ms: number) => vi.advanceTimersByTimeAsync(ms)

describe("pollPhantomOrders", () => {
	beforeEach(() => vi.useFakeTimers())
	afterEach(() => vi.useRealTimers())

	it("emits an order registered in the head block, fully decoded", async () => {
		const h = harness(100)
		h.putOrder(100, COMMITMENT_A)
		const seen: PhantomOrderEvent[] = []

		const stop = h.coprocessor.pollPhantomOrders((e) => seen.push(e), { intervalMs: 1000 })
		await tick(0)
		stop()

		expect(seen).toEqual([
			{
				commitment: COMMITMENT_A,
				chain: CHAIN,
				createdAt: 7,
				tokenA: TOKEN_A,
				tokenB: TOKEN_B,
				standardAmount: 1000000n,
			},
		])
	})

	it("scans each block exactly once as the head advances", async () => {
		const h = harness(100)
		const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

		await tick(0)
		h.setHead(103)
		await tick(1000)
		stop()

		expect(h.scanned).toEqual([100, 101, 102, 103])
	})

	it("does not rescan when the head has not moved", async () => {
		const h = harness(100)
		const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

		await tick(5000)
		stop()

		expect(h.scanned).toEqual([100])
	})

	// The subscription's failure mode: orders registered while disconnected were lost outright.
	it("delivers orders registered during an outage once the connection recovers", async () => {
		const h = harness(100)
		const seen: PhantomOrderEvent[] = []
		const onError = vi.fn()

		const stop = h.coprocessor.pollPhantomOrders((e) => seen.push(e), { intervalMs: 1000, onError })
		await tick(0)

		// Two ticks fail while the chain moves on and registers an order at 102.
		h.putOrder(102, COMMITMENT_B)
		h.setHead(103)
		h.failHeadReads(2)
		await tick(2000)
		expect(seen).toHaveLength(0)
		expect(onError).toHaveBeenCalledTimes(2)

		await tick(1000)
		stop()

		expect(seen.map((e) => e.commitment)).toEqual([COMMITMENT_B])
		expect(h.scanned).toEqual([100, 101, 102, 103])
	})

	it("caps how many blocks a single poll scans and catches up over later ticks", async () => {
		const h = harness(100)
		const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000, maxBlocksPerPoll: 2 })

		await tick(0)
		h.setHead(106)
		await tick(1000)
		expect(h.scanned).toEqual([100, 101, 102])

		await tick(1000)
		stop()

		expect(h.scanned).toEqual([100, 101, 102, 103, 104])
	})

	it("starts lookbackBlocks behind the head so a restart mid-window still bids", async () => {
		const h = harness(100)
		h.putOrder(98, COMMITMENT_A)
		const seen: PhantomOrderEvent[] = []

		const stop = h.coprocessor.pollPhantomOrders((e) => seen.push(e), { intervalMs: 1000, lookbackBlocks: 3 })
		await tick(0)
		stop()

		expect(h.scanned).toEqual([97, 98, 99, 100])
		expect(seen.map((e) => e.commitment)).toEqual([COMMITMENT_A])
	})

	it("stops scanning once stopped", async () => {
		const h = harness(100)
		const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

		await tick(0)
		stop()
		h.setHead(110)
		await tick(5000)

		expect(h.scanned).toEqual([100])
	})
})
