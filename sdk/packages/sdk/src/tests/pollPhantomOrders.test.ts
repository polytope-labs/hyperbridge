import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { xxhashAsU8a } from "@polkadot/util-crypto"
import { u8aConcat, u8aToHex } from "@polkadot/util"
import { IntentsCoprocessor, type PhantomOrderEvent } from "@/chains/intentsCoprocessor"

// Polling replaced a system.events subscription because a dropped socket silently stopped delivering
// phantom orders — polkadot-js reconnects the transport but does not reliably re-establish storage
// subscriptions, and anything emitted while disconnected was gone. The property that makes a block
// cursor an actual fix, rather than a different way to lose orders, is that it advances only past
// blocks whose events were really read, so an outage delays orders instead of dropping them. That is
// what most of these assert — bounded by the lag limit, past which the backlog is deliberately
// abandoned, because an order that far behind can no longer be bid on by anyone.

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
	/** Every hash and events read in the order it was issued, as `hash:<n>` / `events:<n>`. */
	issued: string[]
	/** The `[from, to]` block numbers of each `state_queryStorage` call. */
	rangeCalls: Array<[number, number]>
	/** The raw arguments each `state_queryStorage` call was made with. */
	queryStorageArgs: unknown[][]
	/** Makes the ranged reply come back undecoded, as it does when the key carries no metadata. */
	breakRangeDecoding: () => void
	/**
	 * Marks a block as encoding its events byte-for-byte like its predecessor's, which is what makes
	 * `state_queryStorage` omit it from the reply.
	 */
	repeatEventsAt: (blockNumber: number) => void
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

/**
 * The key the poll must ask for: `twox_128("System") ++ twox_128("Events")`, computed the same way
 * the coprocessor computes it. A key that does not match is the failure this models — the node's
 * change set is keyed by it, so asking for anything else finds nothing.
 */
const EVENTS_KEY = u8aToHex(u8aConcat(xxhashAsU8a("System", 128), xxhashAsU8a("Events", 128)))

/** Encodes a block number into the fake stored value, so the decode can be mapped back to a block. */
const encodedEventsFor = (blockNumber: number) => `0xblockevents${blockNumber}`

const blockOf = (hash: string) => Number(hash.replace("0xblock", ""))

function harness(
	initialHead: number,
	{ specName = "nexus", rangeQueries = true }: { specName?: string; rangeQueries?: boolean } = {},
): Harness {
	let head = initialHead
	let headFailures = 0
	let rateLimitedHeads = 0
	let specVersion = 1
	let versionReads = 0
	let websocketTouched = false
	const ordersByBlock = new Map<number, string[]>()
	const scanned: number[] = []
	const pinnedVersions: Array<FakeRuntimeVersion | undefined> = []
	const issued: string[] = []
	const rangeCalls: Array<[number, number]> = []
	const queryStorageArgs: unknown[][] = []
	const repeats = new Set<number>()
	let rangeDecodes = true

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

	// polkadot-js returns a codec, and the poll reads `.toHex()` off it.
	const getBlockHash = vi.fn(async (n: number) => {
		issued.push(`hash:${n}`)
		return { toHex: () => `0xblock${n}` }
	})

	const getRuntimeVersion = vi.fn(async () => {
		versionReads += 1
		return runtimeVersion(specName, specVersion)
	})

	/**
	 * Stands in for `state_queryStorage.raw`, which answers with the node's own JSON: one change set
	 * per block whose value differs from the block before it, each keyed by the storage key that was
	 * asked for. A key that matches nothing yields no change — silently, which is why the poll has to
	 * name the right one.
	 */
	const queryStorageRaw = vi.fn(async (keys: string[], fromHash: string, toHash: string) => {
		if (!rangeQueries) {
			const error = new Error("Method not found: state_queryStorage") as Error & { code?: number }
			error.code = -32601
			throw error
		}
		const from = blockOf(fromHash)
		const to = blockOf(toHash)
		rangeCalls.push([from, to])
		queryStorageArgs.push([keys, fromHash, toHash])

		const sets: Array<{ block: string; changes: Array<[string, string | null]> }> = []
		for (let blockNumber = from; blockNumber <= to; blockNumber++) {
			if (repeats.has(blockNumber)) continue
			issued.push(`range:${blockNumber}`)
			scanned.push(blockNumber)
			// A node reports changes for the keys that exist. Ask for a key that is not the events
			// key and it reports none — which is how a wrong key produces silence, not an error.
			sets.push({
				block: `0xblock${blockNumber}`,
				changes: keys.includes(EVENTS_KEY) ? [[EVENTS_KEY, encodedEventsFor(blockNumber)]] : [],
			})
		}
		return sets
	})

	/** Stands in for `registry.createType("Vec<EventRecord>", value)`. */
	const createType = vi.fn((_type: string, value: string) => {
		if (!rangeDecodes) return new Uint8Array([1, 2, 3]) // undecodable, as a bad type would give
		const blockNumber = Number(String(value).replace("0xblockevents", ""))
		const commitments = ordersByBlock.get(blockNumber) ?? []
		return [unrelatedEvent, ...commitments.map(registeredEvent)]
	})

	const at = vi.fn(async (blockHash: string, knownVersion?: FakeRuntimeVersion) => {
		const blockNumber = Number(blockHash.replace("0xblock", ""))
		issued.push(`events:${blockNumber}`)
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
			rpc: {
				chain: { getHeader, getBlockHash },
				state: { getRuntimeVersion, queryStorage: Object.assign(vi.fn(), { raw: queryStorageRaw }) },
			},
			registry: { createType },
			at,
			runtimeVersion: runtimeVersion(specName, specVersion),
		}),
	})

	return {
		coprocessor,
		scanned,
		pinnedVersions,
		issued,
		rangeCalls,
		queryStorageArgs,
		breakRangeDecoding: () => {
			rangeDecodes = false
		},
		repeatEventsAt: (blockNumber) => repeats.add(blockNumber),
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

	// The failure this prevents, seen on mainnet: a filler's cursor slipped behind during a spell of
	// rate limiting and never recovered, because the cursor gains at most `maxBlocksPerPoll` a tick.
	// It kept bidding — on orders three thousand blocks old, whose window had closed hours before —
	// so it looked alive, reserved a deposit per bid, and was counted in no pool for nine hours.
	describe("abandoning a backlog too old to bid on", () => {
		it("skips ahead to the head and reports the range it dropped", async () => {
			const h = harness(100)
			const onSkip = vi.fn()
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000, onSkip })

			await tick(0)
			expect(h.scanned).toEqual([100])

			h.setHead(200)
			await tick(1000)
			stop()

			// Nothing from 101 to 199 is scanned: those orders can no longer be bid on.
			expect(h.scanned).toEqual([100, 200])
			expect(onSkip).toHaveBeenCalledWith({ from: 101, to: 199, head: 200 })
		})

		// Short of the limit the cursor still walks every block, so an ordinary hiccup loses nothing.
		it("catches up block by block while the backlog is still biddable", async () => {
			const h = harness(100)
			const onSkip = vi.fn()
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000, onSkip })

			await tick(0)
			h.setHead(105)
			await tick(1000)
			stop()

			expect(h.scanned).toEqual([100, 101, 102, 103, 104, 105])
			expect(onSkip).not.toHaveBeenCalled()
		})

		// A cold start lands exactly one block behind the head, which must never read as a backlog.
		it("does not mistake a cold start for a backlog", async () => {
			const h = harness(1000)
			const onSkip = vi.fn()
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000, onSkip })

			await tick(0)
			stop()

			expect(h.scanned).toEqual([1000])
			expect(onSkip).not.toHaveBeenCalled()
		})
	})

	// A restart starts at the head and reaches back for nothing. The window of an order registered
	// before it came up has all but closed, so the only bids it could still place are the late ones
	// this poll now exists to avoid.
	it("starts at the head, ignoring orders registered before it came up", async () => {
		const h = harness(100)
		h.putOrder(98, COMMITMENT_A)
		const seen: PhantomOrderEvent[] = []

		const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), { intervalMs: 1000 })
		await tick(0)
		stop()

		expect(h.scanned).toEqual([100])
		expect(seen).toEqual([])
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
		const h = harness(100, { specName: "gargantua" })
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

	// What a tick costs when the node will not serve `state_queryStorage` and the poll reads block by
	// block. `api.at(hash)` with nothing to go on fetches the header and the runtime version at its
	// parent before it can decode a block, so a scan that looks like one request per block is four —
	// enough to clear a per-second limit from a tick that averages well under it. Naming the version
	// removes both. These pin that the version is established, that it is established once per tick
	// rather than per block, and that it is dropped rather than trusted the moment it might not be
	// the block's.
	describe("per-block fallback", () => {
		it("decodes blocks against a runtime version it has confirmed", async () => {
			const h = harness(100, { rangeQueries: false })
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			h.setHead(102)
			await tick(1000)
			stop()

			expect(h.scanned).toEqual([100, 101, 102])
			expect(h.pinnedVersions.map((v) => v?.specVersion.toNumber())).toEqual([1, 1, 1])
		})

		it("reads the runtime version once per tick, not once per block", async () => {
			const h = harness(100, { rangeQueries: false })
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			h.setHead(105)
			await tick(1000)
			stop()

			expect(h.scanned).toHaveLength(6)
			expect(h.runtimeVersionReads()).toBe(2)
		})

		it("does not read the runtime version on a tick with nothing to scan", async () => {
			const h = harness(100, { rangeQueries: false })
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
			const h = harness(100, { rangeQueries: false })
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

		// `chain_getBlockHash` takes no historic block hash, so concurrent calls cannot race each
		// other's registry state the way concurrent `api.at` calls would — which makes it the half
		// of the pair that parallelises safely, and therefore the half the provider can batch.
		it("fetches a range's block hashes in one wave, before reading any events", async () => {
			const h = harness(100, { rangeQueries: false })
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			h.issued.length = 0
			h.setHead(103)
			await tick(1000)
			stop()

			expect(h.issued).toEqual(["hash:101", "hash:102", "hash:103", "events:101", "events:102", "events:103"])
		})
	})

	// A backlog inside the lag limit, so this is about the per-tick cap and nothing else.
	it("caps a catch-up at 10 blocks by default", async () => {
		const h = harness(100)
		const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

		await tick(0)
		h.setHead(140)
		await tick(1000)
		stop()

		expect(h.scanned).toEqual([100, ...Array.from({ length: 10 }, (_, i) => 101 + i)])
		expect(h.rangeCalls).toEqual([
			[100, 100],
			[101, 110],
		])
	})

	// `state_queryStorage` reads a whole range in one call, so a scan's request cost stops depending
	// on how many blocks it covers. Two things about the RPC shape these have to pin: it answers with
	// diffs rather than one entry per block, and a node running `--rpc-methods=safe` refuses it.
	describe("ranged scan", () => {
		// The bug this file exists to prevent a repeat of: the range read was asking for
		// `system.events.key()`, a bare hex string. polkadot-js takes a `StorageKey`'s `meta` from a
		// function input and has none for a string, so the value's type fell back to `Raw` and the
		// events came back as undecoded bytes. Nothing threw at the RPC layer — the poll simply
		// found no phantom orders in any block, and no filler ever bid.
		it("asks for the System.Events key, computed rather than looked up", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			stop()

			const [keys] = h.queryStorageArgs[0] as [string[]]
			expect(keys).toEqual([EVENTS_KEY])
			// Which is what the node keys its change sets by, so the events are actually found.
			expect(h.scanned).toEqual([100])
		})

		// Losing the fast path costs requests. Losing every bid, which is what a reply the poll
		// cannot decode did before this, costs the thing the poll exists for.
		it("drops to per-block reads, loudly, when the ranged reply will not decode", async () => {
			const h = harness(100)
			h.breakRangeDecoding()
			h.putOrder(101, COMMITMENT_A)
			const seen: PhantomOrderEvent[] = []
			const onError = vi.fn()

			const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), {
				intervalMs: 1000,
				onError,
			})
			await tick(0)
			h.setHead(101)
			await tick(1000)
			stop()

			// Reported rather than swallowed, but the poll kept working and the order arrived.
			expect(onError).toHaveBeenCalledTimes(1)
			expect(seen.map((e) => e.commitment)).toEqual([COMMITMENT_A])
			expect(h.rangeCalls).toHaveLength(1)
		})

		it("reads a whole range in a single call", async () => {
			const h = harness(100)
			h.putOrder(102, COMMITMENT_A)
			const seen: PhantomOrderEvent[] = []

			const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), { intervalMs: 1000 })
			await tick(0)
			h.setHead(104)
			await tick(1000)
			stop()

			expect(h.rangeCalls).toEqual([
				[100, 100],
				[101, 104],
			])
			expect(seen.map((e) => e.commitment)).toEqual([COMMITMENT_A])
		})

		it("reads no block's events on its own, and asks only for the range's two bounds", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			h.issued.length = 0
			h.setHead(105)
			await tick(1000)
			stop()

			// Six blocks covered by one ranged read, and two hashes rather than six.
			expect(h.issued).toEqual(["hash:101", "hash:105", ...[101, 102, 103, 104, 105].map((n) => `range:${n}`)])
			// `api.at` is never reached, so no per-block registry resolution happens at all.
			expect(h.pinnedVersions).toEqual([])
		})

		// `query_storage_unfiltered` emits a change set only where the value differs from the block
		// before, so a block encoding identically to its predecessor is simply absent. Safe here
		// because a phantom commitment is derived from the block number, so a block that registered
		// orders can never encode like another one — absent provably means no orders.
		it("passes over a block the node omitted as unchanged", async () => {
			const h = harness(100)
			h.putOrder(101, COMMITMENT_A)
			h.repeatEventsAt(102)
			const seen: PhantomOrderEvent[] = []

			const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), { intervalMs: 1000 })
			await tick(0)
			h.setHead(103)
			await tick(1000)

			expect(h.scanned).toEqual([100, 101, 103])
			expect(seen.map((e) => e.commitment)).toEqual([COMMITMENT_A])

			// The cursor still moved past the omitted block: nothing is re-read.
			h.setHead(104)
			await tick(1000)
			stop()

			expect(h.rangeCalls.at(-1)).toEqual([104, 104])
		})

		// `--rpc-methods=safe`. The poll must keep working, and must stop asking.
		it("switches to per-block reads for good when the node refuses the method", async () => {
			const h = harness(100, { rangeQueries: false })
			h.putOrder(101, COMMITMENT_A)
			const seen: PhantomOrderEvent[] = []
			const onError = vi.fn()

			const stop = h.coprocessor.pollPhantomOrders((orders) => seen.push(...orders), {
				intervalMs: 1000,
				onError,
			})
			await tick(0)
			h.setHead(102)
			await tick(1000)
			stop()

			// The refusal is handled, not reported.
			expect(onError).not.toHaveBeenCalled()
			expect(seen.map((e) => e.commitment)).toEqual([COMMITMENT_A])
			expect(h.scanned).toEqual([100, 101, 102])
			// Asked once, on the first tick, and never again.
			expect(h.rangeCalls).toEqual([])
			expect(h.issued.filter((entry) => entry.startsWith("range:"))).toEqual([])
		})

		// The reply is decoded against the api's connect-time registry, so the confirmed version has
		// to still be that one. An upgrade tick has no confirmed version at all, and every tick after
		// it has one the registry no longer matches.
		it("stays on the per-block path once the runtime has moved past the api's registry", async () => {
			const h = harness(100)
			const stop = h.coprocessor.pollPhantomOrders(() => {}, { intervalMs: 1000 })

			await tick(0)
			expect(h.rangeCalls).toEqual([[100, 100]])

			h.setSpecVersion(2)
			h.setHead(102)
			await tick(1000)
			h.setHead(103)
			await tick(1000)
			stop()

			// Nothing ranged after the upgrade; the blocks still all got scanned.
			expect(h.rangeCalls).toEqual([[100, 100]])
			expect(h.scanned).toEqual([100, 101, 102, 103])
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
