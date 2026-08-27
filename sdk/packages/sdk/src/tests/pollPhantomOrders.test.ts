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
				[
					{
						tokenA: { toHex: () => TOKEN_A },
						tokenB: { toHex: () => TOKEN_B },
						standardAmount: { toString: () => "1000000" },
					},
				],
			],
		},
	}
}

const unrelatedEvent = { event: { section: "balances", method: "Transfer", data: [] } }

/** A RuntimeVersion shaped the way `getBlockRegistry` compares them: codec fields with `.eq`. */
function runtimeVersion(specName: string, specVersion: number) {
	return {
		specName: { toString: () => specName, eq: (other: { toString(): string }) => other?.toString() === specName },
		specVersion: {
			toNumber: () => specVersion,
			eq: (other: { toNumber(): number }) => other?.toNumber?.() === specVersion,
		},
	}
}

type FakeRuntimeVersion = ReturnType<typeof runtimeVersion>

interface Harness {
	coprocessor: IntentsCoprocessor
	/** Blocks scanned, in order. */
	scanned: number[]
	/** The runtime version `api.at` was called with for each scanned block, positionally. */
	pinnedVersions: Array<FakeRuntimeVersion | undefined>
	setHead: (n: number) => void
	/** Registers an order in a block; a block can carry several, as the pallet writes them. */
	putOrder: (blockNumber: number, commitment: string) => void
	/** Makes the next `count` head reads throw, simulating an HTTP endpoint that is not answering. */
	failHeadReads: (count: number) => void
	/** Makes the next `count` head reads fail the way a limiter rejects one. */
	rateLimitHeadReads: (count: number) => void
	/** Upgrades the runtime, as seen by the next `state_getRuntimeVersion`. */
	setSpecVersion: (n: number) => void
	/** How many times the runtime version has been read from the node. */
	runtimeVersionReads: () => number
	/** True once anything has touched the websocket api — polling never should. */
	touchedWebsocket: () => boolean
}

function harness(initialHead: number, specName = "nexus"): Harness {
	let head = initialHead
	let headFailures = 0
	let rateLimitedHeads = 0
	let specVersion = 1
	let versionReads = 0
	let websocketTouched = false
	const ordersByBlock = new Map<number, string[]>()
	const scanned: number[] = []
	const pinnedVersions: Array<FakeRuntimeVersion | undefined> = []

	const getHeader = vi.fn(async () => {
		if (rateLimitedHeads > 0) {
			rateLimitedHeads -= 1
			throw new Error("[429]: Too Many Requests")
		}
		if (headFailures > 0) {
			headFailures -= 1
			throw new Error("rpc unavailable")
		}
		return { number: { toNumber: () => head } }
	})

	const getBlockHash = vi.fn(async (n: number) => `0xblock${n}`)

	const getRuntimeVersion = vi.fn(async () => {
		versionReads += 1
		return runtimeVersion(specName, specVersion)
	})

	const at = vi.fn(async (blockHash: string, knownVersion?: FakeRuntimeVersion) => {
		const blockNumber = Number(blockHash.replace("0xblock", ""))
		scanned.push(blockNumber)
		pinnedVersions.push(knownVersion)
		const commitments = ordersByBlock.get(blockNumber) ?? []
		const records = [unrelatedEvent, ...commitments.map(registeredEvent)]
		return { query: { system: { events: async () => records } } }
	})

	// Polling is HTTP-only, so the websocket stands in as a tripwire: any read routed to it shows
	// up as a touch rather than quietly succeeding.
	const websocket = new Proxy(
		{},
		{
			get: () => {
				websocketTouched = true
				throw new Error("polling must not touch the websocket")
			},
		},
	)

	const coprocessor = Object.create(IntentsCoprocessor.prototype) as IntentsCoprocessor
	Object.assign(coprocessor, {
		api: websocket,
		// Pre-resolved so nothing tries to open a real connection.
		httpApi: Promise.resolve({
			rpc: { chain: { getHeader, getBlockHash }, state: { getRuntimeVersion } },
			at,
			runtimeVersion: runtimeVersion(specName, specVersion),
		}),
	})

	return {
		coprocessor,
		scanned,
		pinnedVersions,
		setHead: (n) => {
			head = n
		},
		putOrder: (blockNumber, commitment) =>
			ordersByBlock.set(blockNumber, [...(ordersByBlock.get(blockNumber) ?? []), commitment]),
		failHeadReads: (count) => {
			headFailures = count
		},
		rateLimitHeadReads: (count) => {
			rateLimitedHeads = count
		},
		setSpecVersion: (n) => {
			specVersion = n
		},
		runtimeVersionReads: () => versionReads,
		touchedWebsocket: () => websocketTouched,
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

		const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), { intervalMs: 1000 })
		await tick(0)
		stop()

		expect(seen).toEqual([
			{
				commitment: COMMITMENT_A,
				chain: CHAIN,
				createdAt: 7,
				legs: [{ tokenA: TOKEN_A, tokenB: TOKEN_B, standardAmount: 1000000n }],
			},
		])
	})

	// The pallet registers every configured chain's order in one block, and bidding on them takes
	// one extrinsic — so they have to arrive together, not one callback at a time.
	it("delivers a block's orders in a single callback", async () => {
		const h = harness(100)
		h.putOrder(100, COMMITMENT_A)
		h.putOrder(100, COMMITMENT_B)
		const batches: PhantomOrderEvent[][] = []

		const stop = h.coprocessor.pollPhantomOrders((orders) => batches.push(orders), { intervalMs: 1000 })
		await tick(0)
		stop()

		expect(batches).toHaveLength(1)
		expect(batches[0].map((e) => e.commitment)).toEqual([COMMITMENT_A, COMMITMENT_B])
	})

	it("does not call back for a block with no orders", async () => {
		const h = harness(100)
		const batches: PhantomOrderEvent[][] = []

		const stop = h.coprocessor.pollPhantomOrders((orders) => batches.push(orders), { intervalMs: 1000 })
		await tick(0)
		stop()

		expect(h.scanned).toEqual([100])
		expect(batches).toEqual([])
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

		const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), { intervalMs: 1000, onError })
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

	// Every read goes over HTTP, so the websocket's state — connected or not — is irrelevant to
	// polling, and a socket outage cannot pause phantom bidding. The harness's websocket throws on
	// any access, so every other test in this file asserts the same thing implicitly.
	it("never reads over the websocket", async () => {
		const h = harness(100)
		h.putOrder(101, COMMITMENT_A)
		const seen: PhantomOrderEvent[] = []

		const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), { intervalMs: 1000 })
		await tick(0)
		h.setHead(101)
		await tick(1000)
		stop()

		expect(seen.map((e) => e.commitment)).toEqual([COMMITMENT_A])
		expect(h.touchedWebsocket()).toBe(false)
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

		const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), {
			intervalMs: 1000,
			lookbackBlocks: 3,
		})
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

	it("polls every block on gargantua when no interval is given", async () => {
		const h = harness(100, "gargantua")
		const stop = h.coprocessor.pollPhantomOrders(() => {})

		await tick(0)
		h.setHead(101)
		await tick(6_000)
		stop()

		expect(h.scanned).toEqual([100, 101])
	})

	it("polls every 15s on every other runtime when no interval is given", async () => {
		const h = harness(100)
		const stop = h.coprocessor.pollPhantomOrders(() => {})

		await tick(0)
		h.setHead(101)
		await tick(6_000)
		expect(h.scanned).toEqual([100])

		await tick(9_000)
		stop()

		expect(h.scanned).toEqual([100, 101])
	})

	// The cadence is not the request rate. `api.at(hash)` with nothing to go on fetches the header
	// and the runtime version at its parent before it can decode a block, so a scan that looks like
	// one request per block is four — enough to clear a per-second limit from a tick that averages
	// well under it. Naming the version removes both. These pin that the version is established,
	// that it is established once per tick rather than per block, and that it is dropped rather than
	// trusted the moment it might not be the block's.
	describe("request cost", () => {
		it("decodes blocks against a runtime version it has confirmed", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			h.setHead(102)
			await tick(1000)
			stop()

			expect(h.scanned).toEqual([100, 101, 102])
			expect(h.pinnedVersions.map((v) => v?.specVersion.toNumber())).toEqual([1, 1, 1])
		})

		it("reads the runtime version once per tick, not once per block", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			h.setHead(105)
			await tick(1000)
			stop()

			expect(h.scanned).toHaveLength(6)
			expect(h.runtimeVersionReads()).toBe(2)
		})

		it("does not read the runtime version on a tick with nothing to scan", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			expect(h.runtimeVersionReads()).toBe(1)

			// Three ticks with the head where it was: one head read each and nothing else.
			await tick(3000)
			stop()

			expect(h.runtimeVersionReads()).toBe(1)
		})

		// A version pinned across the block that changed it decodes the upgraded blocks against the
		// old metadata, and that failure is silent: the events come back in a shape the scan does
		// not recognise, so the block reads as empty and the cursor advances past the orders in it.
		it("resolves each block itself for the tick an upgrade landed in, and pins again after", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			expect(h.pinnedVersions.map((v) => v?.specVersion.toNumber())).toEqual([1])

			h.setSpecVersion(2)
			h.setHead(101)
			await tick(1000)
			// The reading disagrees with the last one, so this tick's block resolves its own.
			expect(h.pinnedVersions.map((v) => v?.specVersion.toNumber())).toEqual([1, undefined])

			h.setHead(102)
			await tick(1000)
			stop()

			// Two agreeing readings later, the new version is trusted.
			expect(h.pinnedVersions.map((v) => v?.specVersion.toNumber())).toEqual([1, undefined, 2])
		})

		it("caps a catch-up at 20 blocks by default", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			h.setHead(200)
			await tick(1000)
			stop()

			expect(h.scanned).toEqual([100, ...Array.from({ length: 20 }, (_, i) => 101 + i)])
		})
	})

	// A limiter that is already shedding load gains nothing from being asked again on schedule, and
	// the poll gains nothing either — the next window's budget goes on rejections instead of blocks.
	describe("rate limit backoff", () => {
		it("sits out a doubling number of ticks while the endpoint rejects", async () => {
			const h = harness(100)
			const onError = vi.fn()
			h.rateLimitHeadReads(2)

			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000, onError })

			await tick(0)
			expect(onError).toHaveBeenCalledTimes(1)

			// One tick sat out after the first rejection.
			await tick(1000)
			expect(onError).toHaveBeenCalledTimes(1)

			// Second attempt, rejected too — the backoff doubles to two ticks.
			await tick(1000)
			expect(onError).toHaveBeenCalledTimes(2)

			await tick(2000)
			expect(h.scanned).toEqual([])

			await tick(1000)
			stop()

			expect(h.scanned).toEqual([100])
		})

		it("starts over at one tick once a poll has got through", async () => {
			const h = harness(100)
			const onError = vi.fn()
			h.rateLimitHeadReads(2)

			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000, onError })

			// Rejected at 0 and 2000, backed off to two ticks, through at 5000.
			await tick(5000)
			expect(h.scanned).toEqual([100])

			// A fresh rejection is one tick again, not four.
			h.rateLimitHeadReads(1)
			h.setHead(101)
			await tick(1000)
			expect(onError).toHaveBeenCalledTimes(3)

			await tick(1000)
			expect(h.scanned).toEqual([100])

			await tick(1000)
			stop()

			expect(h.scanned).toEqual([100, 101])
		})

		// Only a limiter warrants the pause. An endpoint that is down answers the next tick just as
		// well as the fourth, and a bid window is short enough that waiting costs it.
		it("retries an ordinary failure on the very next tick", async () => {
			const h = harness(100)
			const onError = vi.fn()
			h.failHeadReads(3)

			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000, onError })

			await tick(2000)
			stop()

			expect(onError).toHaveBeenCalledTimes(3)
		})
	})

	it("keeps polling on the slower cadence when the runtime cannot be read", async () => {
		const h = harness(100)
		// A node that answers block reads but whose runtime version is unavailable.
		const api = (await (h.coprocessor as unknown as { httpApi: Promise<Record<string, unknown>> }).httpApi)!
		delete api.runtimeVersion

		const stop = h.coprocessor.pollPhantomOrders(() => {})

		await tick(0)
		h.setHead(101)
		await tick(6_000)
		expect(h.scanned).toEqual([100])

		await tick(9_000)
		stop()

		expect(h.scanned).toEqual([100, 101])
	})
})
