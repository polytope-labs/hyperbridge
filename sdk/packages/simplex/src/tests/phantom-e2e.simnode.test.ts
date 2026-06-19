/**
 * End-to-end integration test for the phantom order bid lifecycle.
 *
 * Tests the full SDK ↔ pallet-intents-coprocessor integration: a coprocessor
 * registers phantom orders, fillers place bids via the SDK, and the bids are
 * discoverable via intents_getBidsForOrder RPC. Governance (bid window) is
 * exercised via sudo.
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
import { randomBytes } from "crypto"
import { IntentsCoprocessor, encodeUserOpScale } from "@hyperbridge/sdk"
import type { HexString, PackedUserOperation } from "@hyperbridge/sdk"

const SIMNODE_URL = process.env.SIMNODE_URL || "ws://127.0.0.1:9990"

function randomCommitment(): HexString {
	return `0x${randomBytes(32).toString("hex")}` as HexString
}

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
	// Find this extrinsic's index in the block so we only check its events
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

		// Use URI derivation paths — works with any Substrate dev chain regardless of genesis funding
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

		// Reset bid window to a generous default so tests start from a clean state
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(100))
	}, 60_000)

	afterAll(async () => {
		await api.disconnect()
	})

	it("registerPhantomOrder() stores the commitment on Hyperbridge", async () => {
		const commitment = randomCommitment()
		const chain = "EVM-8453"

		const promise = coprocessor.registerPhantomOrder(commitment, chain)
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const result = await promise

		console.log("registerPhantomOrder result:", result)
		expect(result.success).toBe(true)

		// Verify CurrentPhantomOrder storage via raw RPC — avoids type registration for custom types.
		// OptionQuery stores the raw value T directly (no 0x01 Some prefix); the first 32 bytes are H256.
		const storageKey = await api.query.intentsCoprocessor.currentPhantomOrder.key()
		const raw: any = await api.rpc.state.getStorage(storageKey)
		expect(raw).not.toBeNull()

		const hex: string = raw.toHex()
		// hex = "0x" + <H256 commitment 64 chars> + <PhantomOrderInfo SCALE bytes>
		expect(hex.slice(2, 66).toLowerCase()).toBe(commitment.slice(2).toLowerCase())
	}, 30_000)

	it("submitBid() places a filler bid visible via getBidsForOrder()", async () => {
		const commitment = randomCommitment()
		const userOp = makeUserOp("0x0001")

		// Register phantom order
		const regPromise = coprocessor.registerPhantomOrder(commitment, "EVM-8453")
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		await regPromise

		// Bob places a bid
		const bidPromise = bobFiller.submitBid(commitment, userOp)
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const result = await bidPromise

		console.log("submitBid result:", result)
		expect(result.success).toBe(true)

		// Verify via getBidsForOrder
		const bids = await coprocessor.getBidsForOrder(commitment)
		console.log("bids:", bids.length, bids.map((b) => b.filler))
		expect(bids.length).toBe(1)
	}, 60_000)

	it("multiple fillers can bid on the same phantom order", async () => {
		const commitment = randomCommitment()

		// Register phantom order
		const regPromise = coprocessor.registerPhantomOrder(commitment, "EVM-8453")
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		await regPromise

		// Queue both bids simultaneously, then seal one block for both
		const bobPromise = bobFiller.submitBid(commitment, makeUserOp("0x00bb"))
		const charliePromise = charlieFiller.submitBid(commitment, makeUserOp("0x00cc"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)

		const [bobResult, charlieResult] = await Promise.all([bobPromise, charliePromise])
		console.log("Bob bid result:", bobResult)
		console.log("Charlie bid result:", charlieResult)

		expect(bobResult.success).toBe(true)
		expect(charlieResult.success).toBe(true)

		const bids = await coprocessor.getBidsForOrder(commitment)
		console.log("bids count:", bids.length)
		expect(bids.length).toBe(2)
	}, 60_000)

	it("duplicate bid from same filler is rejected with DuplicatePhantomBid", async () => {
		const commitment = randomCommitment()

		// Register
		const regPromise = coprocessor.registerPhantomOrder(commitment, "EVM-8453")
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		await regPromise

		// First bid from Bob — should succeed
		const firstPromise = bobFiller.submitBid(commitment, makeUserOp("0x0001"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const firstResult = await firstPromise
		expect(firstResult.success).toBe(true)

		// Second bid from same account (Bob) — should fail with DuplicatePhantomBid
		const dupPromise = bobFiller.submitBid(commitment, makeUserOp("0x0002"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const dupResult = await dupPromise

		console.log("Duplicate bid result:", dupResult)
		expect(dupResult.success).toBe(false)
		expect(dupResult.error).toMatch(/DuplicatePhantomBid/i)
	}, 60_000)

	it("bid is rejected after the bid window closes", async () => {
		const commitment = randomCommitment()

		// Set bid window to 1 block
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(1))

		// Register (block N)
		const regPromise = coprocessor.registerPhantomOrder(commitment, "EVM-8453")
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		await regPromise

		// Advance two more empty blocks — window closes after block N+1
		await createBlock(api)
		await createBlock(api)

		// Bid at block N+3 — window already closed
		const bidPromise = bobFiller.submitBid(commitment, makeUserOp("0x00ff"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const result = await bidPromise

		console.log("Post-window bid result:", result)
		expect(result.success).toBe(false)
		expect(result.error).toMatch(/PhantomOrderBidWindowClosed/i)

		// Reset window so subsequent tests are not affected
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(100))
	}, 60_000)

	it("registration replaces the previous phantom order", async () => {
		const first = randomCommitment()
		const second = randomCommitment()

		// Register first
		const firstPromise = coprocessor.registerPhantomOrder(first, "EVM-8453")
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		await firstPromise

		// Register second — should overwrite
		const secondPromise = coprocessor.registerPhantomOrder(second, "EVM-1")
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		const result = await secondPromise
		expect(result.success).toBe(true)

		// CurrentPhantomOrder should now hold the second commitment
		const storageKey = await api.query.intentsCoprocessor.currentPhantomOrder.key()
		const raw: any = await api.rpc.state.getStorage(storageKey)
		const hex: string = raw.toHex()
		expect(hex.slice(2, 66).toLowerCase()).toBe(second.slice(2).toLowerCase())
	}, 60_000)

	it("full flow: three fillers bid, all discoverable via getBidsForOrder", async () => {
		const commitment = randomCommitment()

		// Register phantom order
		const regPromise = coprocessor.registerPhantomOrder(commitment, "EVM-8453")
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)
		await regPromise

		// Three fillers queue bids simultaneously; one block includes all three
		const bobPromise = bobFiller.submitBid(commitment, makeUserOp("0x00bb"))
		const charliePromise = charlieFiller.submitBid(commitment, makeUserOp("0x00cc"))
		const davePromise = daveFiller.submitBid(commitment, makeUserOp("0x00dd"))
		await new Promise((r) => setTimeout(r, 300))
		await createBlock(api)

		const [bobResult, charlieResult, daveResult] = await Promise.all([bobPromise, charliePromise, davePromise])
		console.log("Bob:", bobResult)
		console.log("Charlie:", charlieResult)
		console.log("Dave:", daveResult)

		expect(bobResult.success).toBe(true)
		expect(charlieResult.success).toBe(true)
		expect(daveResult.success).toBe(true)

		const bids = await coprocessor.getBidsForOrder(commitment)
		console.log("All bids count:", bids.length, bids.map((b) => b.filler))
		expect(bids.length).toBe(3)
	}, 60_000)
})
