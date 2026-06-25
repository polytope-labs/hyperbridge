/**
 * End-to-end integration test for the phantom order bid lifecycle.
 *
 * Tests the full governance → on_initialize → bid flow: governance sets the
 * phantom order config, the runtime hook generates a commitment each interval,
 * fillers place bids via the SDK, and the bids are discoverable via
 * intents_getBidsForOrder RPC.
 *
 * Requires a running hyperbridge simnode (WITHOUT --instant):
 *   cargo build -p hyperbridge
 *   ./target/debug/hyperbridge simnode --chain gargantua-1000 --rpc-port 9990 --tmp
 *
 * Run with:
 *   SIMNODE_URL=ws://127.0.0.1:9990 pnpm test:phantom-e2e
 */
import { describe, it, expect, beforeAll, afterAll } from "vitest"
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { IntentsCoprocessor, encodeUserOpScale } from "@hyperbridge/sdk"
import type { HexString, PackedUserOperation } from "@hyperbridge/sdk"

const SIMNODE_URL = process.env.SIMNODE_URL || "ws://127.0.0.1:9990"

function makeUserOp(callData: string = "0x"): HexString {
	const userOp: PackedUserOperation = {
		sender: "0x0000000000000000000000000000000000000001" as HexString,
		nonce: 0n,
		initCode: "0x" as HexString,
		callData: callData as HexString,
		accountGasLimits: "0x00000000000000000000000000007530000000000000000000000000000f4240" as HexString,
		preVerificationGas: 21000n,
		gasFees: "0x00000000000000000000000000000001000000000000000000000000000f4240" as HexString,
		paymasterAndData: "0x" as HexString,
		signature: "0x" as HexString,
	}
	return encodeUserOpScale(userOp)
}

async function rpc(api: ApiPromise, method: string, params: any[] = []): Promise<any> {
	return (api as any)._rpcCore.provider.send(method, params)
}

async function createBlock(api: ApiPromise): Promise<void> {
	const block = await rpc(api, "engine_createBlock", [true, false])
	await rpc(api, "engine_finalizeBlock", [block.hash])
}

async function submitAndSeal(
	api: ApiPromise,
	extrinsic: any,
	signer: any,
): Promise<{ success: boolean; error?: string }> {
	await extrinsic.signAsync(signer)
	const txHash = extrinsic.hash.toHex()
	await api.rpc.author.submitExtrinsic(extrinsic)
	await createBlock(api)
	const header = await api.rpc.chain.getHeader()
	const apiAt = await api.at(header.hash)
	const block = await api.rpc.chain.getBlock(header.hash)
	const extrinsicIndex = block.block.extrinsics.findIndex((ext: any) => ext.hash.toHex() === txHash)
	const events: any[] = (await apiAt.query.system.events()) as any
	for (const { phase, event } of events) {
		if (
			phase.isApplyExtrinsic &&
			phase.asApplyExtrinsic.toNumber() === extrinsicIndex &&
			event.section === "system" &&
			event.method === "ExtrinsicFailed"
		) {
			return { success: false, error: `Dispatch error: ${event.data[0].toString()}` }
		}
	}
	return { success: true }
}

async function sudoAndSeal(api: ApiPromise, call: any): Promise<void> {
	const keyring = new Keyring({ type: "sr25519" })
	const alice = keyring.addFromUri("//Alice")
	const sudoCall = api.tx.sudo.sudo(call)
	const result = await submitAndSeal(api, sudoCall, alice)
	if (!result.success) throw new Error(result.error || "sudo call failed")
}

// Consensus state id used for the EVM destination chain in these tests.
const ETH0_CONSENSUS_ID = "0x45544830"

/**
 * Seeds a confirmed height for the destination chain in `Ismp::LatestStateMachineHeight` via sudo
 * `system.setStorage`. The on_initialize hook reads this to set the phantom order deadline; a bare
 * simnode has no external consensus, so without it generation is skipped.
 */
async function seedStateMachineHeight(api: ApiPromise, chainId: number, height: bigint): Promise<void> {
	const id = { state_id: { Evm: chainId }, consensus_state_id: ETH0_CONSENSUS_ID }
	const key = api.query.ismp.latestStateMachineHeight.key(id)
	const value = api.createType("u64", height).toHex()
	await sudoAndSeal(api, api.tx.system.setStorage([[key, value]]))
}

/**
 * Submits a `set_phantom_order_config` governance call and seals a block.
 * Uses a single token pair of zero-address tokens as a probe.
 */
async function setPhantomOrderConfig(api: ApiPromise, chainId: number, intervalBlocks: number): Promise<void> {
	const config = {
		chain: { state_id: { Evm: chainId }, consensus_state_id: ETH0_CONSENSUS_ID },
		token_pairs: [
			{
				token_a: "0x0101010101010101010101010101010101010101",
				token_b: "0x0202020202020202020202020202020202020202",
				standard_amount: 1_000_000_000_000_000_000n,
			},
		],
		interval_blocks: intervalBlocks,
	}
	await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomOrderConfig(config))
}

/**
 * Reads the first active phantom commitment from `CurrentPhantomOrder` storage at
 * the latest block. Returns null when the storage slot is empty.
 *
 * `CurrentPhantomOrder` is a `BoundedVec<(H256, PhantomOrderInfo), _>`, so the bytes start
 * with a one byte compact length before the first entry's H256 commitment.
 */
async function getActivePhantomCommitment(api: ApiPromise): Promise<HexString | null> {
	const storageKey = api.query.intentsCoprocessor.currentPhantomOrder.key()
	const raw: any = await api.rpc.state.getStorage(storageKey)
	if (!raw) return null
	const hex: string = raw.toHex()
	if (!hex || hex === "0x" || hex.length < 68) return null
	return `0x${hex.slice(4, 68)}` as HexString
}

describe("Phantom Order E2E (simnode)", () => {
	let api: ApiPromise
	let coprocessor: IntentsCoprocessor
	let bobFiller: IntentsCoprocessor
	let charlieFiller: IntentsCoprocessor
	let daveFiller: IntentsCoprocessor

	beforeAll(async () => {
		api = await ApiPromise.create({
			provider: new WsProvider(SIMNODE_URL),
			typesBundle: {
				spec: {
					gargantua: { hasher: keccakAsU8a },
				},
			},
		})

		coprocessor = IntentsCoprocessor.fromApi(api, "//Alice")
		bobFiller = IntentsCoprocessor.fromApi(api, "//Bob")
		charlieFiller = IntentsCoprocessor.fromApi(api, "//Charlie")
		daveFiller = IntentsCoprocessor.fromApi(api, "//Dave")

		// Fund Charlie and Dave — gargantua-1000 genesis only includes Alice and Bob
		const keyring = new Keyring({ type: "sr25519" })
		const alice = keyring.addFromUri("//Alice")
		const charlieAddress = keyring.addFromUri("//Charlie").address
		const daveAddress = keyring.addFromUri("//Dave").address
		await submitAndSeal(api, api.tx.balances.transferKeepAlive(charlieAddress, 10_000_000_000_000_000_000n), alice)
		await submitAndSeal(api, api.tx.balances.transferKeepAlive(daveAddress, 10_000_000_000_000_000_000n), alice)

		// Reset bid window to a generous default so tests start from a clean state.
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(100))

		// Seed a confirmed destination height so on_initialize can generate phantom orders.
		await seedStateMachineHeight(api, 8453, 1_000_000n)
	}, 60_000)

	afterAll(async () => {
		await api.disconnect()
	})

	it("setPhantomOrderConfig() triggers on_initialize which stores a commitment", async () => {
		await setPhantomOrderConfig(api, 8453, 10)

		// on_initialize fires in the next block.
		await createBlock(api)

		const storageKey = api.query.intentsCoprocessor.currentPhantomOrder.key()
		const raw: any = await api.rpc.state.getStorage(storageKey)
		expect(raw).not.toBeNull()

		const hex: string = raw.toHex()
		expect(hex.length).toBeGreaterThanOrEqual(68)

		// BoundedVec: [compact len (1)] [H256 (32)] [u32 LE (4)] [compact chain len] [chain bytes]
		const bytes = Buffer.from(hex.slice(2), "hex")
		const chainLen = bytes[37] >> 2
		const storedChain = bytes.slice(38, 38 + chainLen).toString("utf8")
		expect(storedChain).toBe("EVM-8453")
	}, 30_000)

	it("submitBid() places a filler bid visible via getBidsForOrder()", async () => {
		await setPhantomOrderConfig(api, 8453, 10)
		await createBlock(api)

		const commitment = await getActivePhantomCommitment(api)
		expect(commitment).not.toBeNull()

		const bidPromise = bobFiller.submitBid(commitment!, makeUserOp("0x0001"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const result = await bidPromise

		console.log("submitBid result:", result)
		expect(result.success).toBe(true)

		const bids = await coprocessor.getBidsForOrder(commitment!)
		console.log("bids:", bids.length, bids.map((b) => b.filler))
		expect(bids.length).toBe(1)
	}, 60_000)

	it("multiple fillers can bid on the same phantom order", async () => {
		await setPhantomOrderConfig(api, 8453, 10)
		await createBlock(api)

		const commitment = await getActivePhantomCommitment(api)
		expect(commitment).not.toBeNull()

		const bobPromise = bobFiller.submitBid(commitment!, makeUserOp("0x00bb"))
		const charliePromise = charlieFiller.submitBid(commitment!, makeUserOp("0x00cc"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)

		const [bobResult, charlieResult] = await Promise.all([bobPromise, charliePromise])
		console.log("Bob bid result:", bobResult)
		console.log("Charlie bid result:", charlieResult)

		expect(bobResult.success).toBe(true)
		expect(charlieResult.success).toBe(true)

		const bids = await coprocessor.getBidsForOrder(commitment!)
		console.log("bids count:", bids.length)
		expect(bids.length).toBe(2)
	}, 60_000)

	it("duplicate bid from same filler is rejected with DuplicatePhantomBid", async () => {
		await setPhantomOrderConfig(api, 8453, 10)
		await createBlock(api)

		const commitment = await getActivePhantomCommitment(api)
		expect(commitment).not.toBeNull()

		// First bid from Bob — should succeed.
		const firstPromise = bobFiller.submitBid(commitment!, makeUserOp("0x0001"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const firstResult = await firstPromise
		expect(firstResult.success).toBe(true)

		// Second bid from the same account — should fail.
		const dupPromise = bobFiller.submitBid(commitment!, makeUserOp("0x0002"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const dupResult = await dupPromise

		console.log("Duplicate bid result:", dupResult)
		expect(dupResult.success).toBe(false)
		expect(dupResult.error).toMatch(/DuplicatePhantomBid/i)
	}, 60_000)

	it("bid is rejected after the bid window closes", async () => {
		// Block N: set bid window to 1.
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(1))

		// Block N+1: set config (on_initialize at N+1 has no config yet).
		await setPhantomOrderConfig(api, 8453, 10)

		// Block N+2: on_initialize fires, phantom created at N+2.
		await createBlock(api)
		const commitment = await getActivePhantomCommitment(api)
		expect(commitment).not.toBeNull()

		// Block N+3: advance empty block — window still open (N+3 <= N+2+1).
		await createBlock(api)

		// Submit bid; it will be included in block N+4 (window closed: N+4 > N+3).
		const bidPromise = bobFiller.submitBid(commitment!, makeUserOp("0x00ff"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const result = await bidPromise

		console.log("Post-window bid result:", result)
		expect(result.success).toBe(false)
		expect(result.error).toMatch(/PhantomOrderBidWindowClosed/i)

		// Reset window so subsequent tests are unaffected.
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(100))
	}, 60_000)

	it("on_initialize replaces the commitment on each interval", async () => {
		// interval_blocks=1 means the hook re-fires every block.
		await setPhantomOrderConfig(api, 8453, 1)

		await createBlock(api)
		const c1 = await getActivePhantomCommitment(api)
		expect(c1).not.toBeNull()

		await createBlock(api)
		const c2 = await getActivePhantomCommitment(api)
		expect(c2).not.toBeNull()

		expect(c1).not.toBe(c2)
	}, 60_000)

	it("full flow: three fillers bid, all discoverable via getBidsForOrder", async () => {
		await setPhantomOrderConfig(api, 8453, 10)
		await createBlock(api)

		const commitment = await getActivePhantomCommitment(api)
		expect(commitment).not.toBeNull()

		const bobPromise = bobFiller.submitBid(commitment!, makeUserOp("0x00bb"))
		const charliePromise = charlieFiller.submitBid(commitment!, makeUserOp("0x00cc"))
		const davePromise = daveFiller.submitBid(commitment!, makeUserOp("0x00dd"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)

		const [bobResult, charlieResult, daveResult] = await Promise.all([bobPromise, charliePromise, davePromise])
		console.log("Bob:", bobResult)
		console.log("Charlie:", charlieResult)
		console.log("Dave:", daveResult)

		expect(bobResult.success).toBe(true)
		expect(charlieResult.success).toBe(true)
		expect(daveResult.success).toBe(true)

		const bids = await coprocessor.getBidsForOrder(commitment!)
		console.log("All bids count:", bids.length, bids.map((b) => b.filler))
		expect(bids.length).toBe(3)
	}, 60_000)
})
