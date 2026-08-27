/**
 * The phantom order poll's ranged read, against a real node.
 *
 * This exists because a bug shipped through a full unit suite and only surfaced in the filler E2E,
 * as zero bids. `getPhantomOrdersInRange` asked `state_queryStorage` for the events key in a form
 * polkadot-js could not decode against, so the events came back as undecoded bytes — or, on the
 * next attempt at a fix, as a silently empty vector. No RPC ever failed. Nothing that mocks the api
 * catches that class of fault, because it lives in what polkadot-js does with a real reply.
 *
 * So this asserts the two things a mock cannot: that the ranged read decodes at all, and that it
 * agrees block-for-block with the per-block path it is an optimisation over.
 *
 * Requires a hyperbridge simnode (manual seal):
 *   ./target/debug/hyperbridge simnode --chain gargantua-1000 --rpc-port 9990 --tmp \
 *     --rpc-methods=unsafe --rpc-cors=all --pool-type=single-state
 *
 * Run with: SIMNODE_URL=ws://127.0.0.1:9990 pnpm --filter @hyperbridge/simplex test:phantom-range
 */
import { describe, it, expect, beforeAll, afterAll } from "vitest"
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { IntentsCoprocessor, type PhantomOrderEvent } from "@hyperbridge/sdk"
import type { HexString } from "@hyperbridge/sdk"

const SIMNODE_URL = process.env.SIMNODE_URL || "ws://127.0.0.1:9990"
const ETH0_CONSENSUS_ID = "0x45544830"
/** Generation cadence, and a bid window strictly shorter than it, as the pallet requires. */
const INTERVAL_BLOCKS = 5
const BID_WINDOW = 2

// biome-ignore lint/suspicious/noExplicitAny: polkadot API types
type AnyApi = any

async function rpc(api: ApiPromise, method: string, params: unknown[] = []): Promise<AnyApi> {
	return (api as AnyApi)._rpcCore.provider.send(method, params)
}

async function createBlock(api: ApiPromise): Promise<void> {
	const block = await rpc(api, "engine_createBlock", [true, false])
	await rpc(api, "engine_finalizeBlock", [block.hash])
}

/**
 * Seals a sudo call and fails loudly if the *inner* call did.
 *
 * `sudo` dispatches the inner call and reports its result in a `Sudid` event; the outer extrinsic
 * succeeds either way. Checking only `ExtrinsicFailed` therefore reports success for a call that
 * did nothing, which is how `phantom-e2e.simnode.test.ts` came to configure nothing and then assert
 * against an unconfigured chain.
 */
async function sudoAndSeal(api: ApiPromise, label: string, call: AnyApi): Promise<void> {
	const alice = new Keyring({ type: "sr25519" }).addFromUri("//Alice")
	const tx = api.tx.sudo.sudo(call)
	await tx.signAsync(alice)
	await api.rpc.author.submitExtrinsic(tx as AnyApi)
	await createBlock(api)
	const at = await api.at((await api.rpc.chain.getHeader()).hash)
	const events = (await at.query.system.events()) as AnyApi
	const sudid = [...events].find((r: AnyApi) => r.event.section === "sudo" && r.event.method === "Sudid")
	const result = sudid?.event.data[0].toString() ?? ""
	if (result.includes('"err"')) throw new Error(`${label} failed inside sudo: ${result}`)
}

describe("phantom order ranged scan (simnode)", () => {
	let api: ApiPromise
	let coprocessor: IntentsCoprocessor

	beforeAll(async () => {
		api = await ApiPromise.create({
			provider: new WsProvider(SIMNODE_URL),
			typesBundle: { spec: { gargantua: { hasher: keccakAsU8a } } },
		})
		coprocessor = IntentsCoprocessor.fromApi(api)

		// The bid window must be strictly shorter than the generation interval
		// (`PhantomBidWindowNotShorterThanInterval`), or the config call is rejected.
		const id = { state_id: { Evm: 8453 }, consensus_state_id: ETH0_CONSENSUS_ID }
		const heightKey = api.query.ismp.latestStateMachineHeight.key(id)
		await sudoAndSeal(
			api,
			"seed state machine height",
			api.tx.system.setStorage([[heightKey, api.createType("u64", 1_000_000n).toHex()]]),
		)
		await sudoAndSeal(api, "set bid window", api.tx.intentsCoprocessor.setPhantomBidWindow(BID_WINDOW))
		await sudoAndSeal(
			api,
			"set phantom order config",
			api.tx.intentsCoprocessor.setPhantomOrderConfig({
				chains: new Map([
					[
						id,
						[
							{
								token_a: "0x0101010101010101010101010101010101010101",
								token_b: "0x0202020202020202020202020202020202020202",
								standard_amount: 1_000_000_000_000_000_000n,
								standard_amount_b: 1_000_000_000_000_000_000n,
							},
						],
					],
				]),
				interval_blocks: INTERVAL_BLOCKS,
			}),
		)
	}, 120_000)

	afterAll(async () => {
		await coprocessor.disconnect()
		await api.disconnect()
	})

	it("decodes phantom orders from a ranged read, and agrees with the per-block read", async () => {
		// A few generation blocks to scan across.
		const from = (await api.rpc.chain.getHeader()).number.toNumber() + 1
		for (let i = 0; i < INTERVAL_BLOCKS * 2 + 2; i++) await createBlock(api)
		const to = (await api.rpc.chain.getHeader()).number.toNumber()

		const fromHash = (await api.rpc.chain.getBlockHash(from)).toHex() as HexString
		const toHash = (await api.rpc.chain.getBlockHash(to)).toHex() as HexString

		// Ground truth: what the chain actually emitted over the range. Asserting against this,
		// rather than against zero, is what makes the decode's correctness observable — the bug
		// returned an empty list, which any "no error" check would have called a pass.
		let onChain = 0
		for (let n = from; n <= to; n++) {
			const at = await api.at(await api.rpc.chain.getBlockHash(n))
			const events = (await at.query.system.events()) as AnyApi
			onChain += [...events].filter(
				(r: AnyApi) =>
					r.event.section === "intentsCoprocessor" && r.event.method === "PhantomOrderRegistered",
			).length
		}
		expect(onChain).toBeGreaterThan(0)

		const ranged = await coprocessor.getPhantomOrdersInRange(fromHash, toHash)
		const rangedOrders = ranged.flat()

		// The decode has to find every order the chain emitted — not merely "some".
		expect(rangedOrders.length).toBe(onChain)
		for (const order of rangedOrders) {
			expect(order.chain).toBe("EVM-8453")
			expect(order.commitment).toMatch(/^0x[0-9a-f]{64}$/)
			expect(order.legs.length).toBeGreaterThan(0)
		}

		// And it must agree with the path it replaces, block for block.
		const perBlock: PhantomOrderEvent[] = []
		for (let n = from; n <= to; n++) {
			perBlock.push(...(await coprocessor.getPhantomOrdersInBlock(n)))
		}

		expect(rangedOrders.map((o) => o.commitment).sort()).toEqual(perBlock.map((o) => o.commitment).sort())
	}, 120_000)

	it("delivers orders through the poll itself", async () => {
		const seen: PhantomOrderEvent[] = []
		const errors: unknown[] = []
		const stop = coprocessor.pollPhantomOrders((orders) => seen.push(...orders), {
			intervalMs: 300,
			onError: (err) => errors.push(err),
		})

		try {
			// Seal past the poll's starting cursor so it has new blocks to find orders in.
			for (let i = 0; i < INTERVAL_BLOCKS * 2 + 2; i++) {
				await createBlock(api)
				await new Promise((resolve) => setTimeout(resolve, 400))
			}
			await new Promise((resolve) => setTimeout(resolve, 2_000))
		} finally {
			stop()
		}

		expect(errors).toEqual([])
		expect(seen.length).toBeGreaterThan(0)
		expect(seen.every((order) => order.chain === "EVM-8453")).toBe(true)
	}, 120_000)
})
