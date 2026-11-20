import "log-timestamp"
import { IntentsCoprocessor, encodeUserOpScale } from "@/chains/intentsCoprocessor"
import { SubstrateChain } from "@/chains/substrate"
import type { HexString, PackedUserOperation } from "@/types"
import { keccak256, parseUnits, toHex } from "viem"
import fixtureData from "@/tests/fixtures/intent-gateway-v2.json"
import { requireFunded } from "./funding"

// placeBid reserves Gargantua's storage deposit (0.1 BRIDGE by default), and placing and retracting pay fees; whole tokens.
const MIN_BIDDER_BALANCE = "1"
const UNFUNDED_NOTE = "No bid was placed: the bidder is unfunded or its funding check failed"

describe.sequential("IntentsCoprocessor", () => {
	let hyperbridge: SubstrateChain
	let coprocessor: IntentsCoprocessor

	const testCommitment = keccak256(toHex(`test-${Date.now()}-${Math.random()}`)) as HexString
	const BID = keccak256(toHex("test-bid")) as HexString

	const fixtureUserOp = fixtureData.userOpHashVectors[0].userOp

	const testUserOp: PackedUserOperation = {
		sender: fixtureUserOp.sender as HexString,
		nonce: BigInt(fixtureUserOp.nonce),
		initCode: fixtureUserOp.initCode as HexString,
		callData: fixtureUserOp.callData as HexString,
		accountGasLimits: fixtureUserOp.accountGasLimits as HexString,
		preVerificationGas: BigInt(fixtureUserOp.preVerificationGas),
		gasFees: fixtureUserOp.gasFees as HexString,
		paymasterAndData: fixtureUserOp.paymasterAndData as HexString,
		signature: "0x" as HexString,
	}
	const encodedUserOp = encodeUserOp(testUserOp)

	// Set while the bidder's funding is unchecked or short, so the tests that read or retract the bid skip with it.
	let unfunded = true

	// The default 10s hook timeout races websocket jitter on the public
	// Hyperbridge endpoint; give the connection a real window. 60s still lost
	// the race in CI — the hook timed out and took all five tests down as
	// skipped — so this matches the per-test budget below.
	beforeAll(async () => {
		hyperbridge = await SubstrateChain.connect({
			wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			hasher: "Keccak",
			stateMachineId: "KUSAMA-4009",
			consensusStateId: "PAS0",
		})
		console.log("Connected to Hyperbridge")

		const privateKey = process.env.SECRET_PHRASE!
		coprocessor = IntentsCoprocessor.fromSubstrateChain(hyperbridge, privateKey)

		console.log("Test commitment:", testCommitment)
		console.log("Test userOp sender:", testUserOp.sender)
	}, 300_000)

	afterAll(async () => {
		await hyperbridge?.disconnect()
		console.log("Disconnected")
	}, 60_000)

	it("should submit a bid with realistic userOp", async (ctx) => {
		const keyPair = coprocessor.getKeyPair()
		const api = hyperbridge.api
		if (!api) throw new Error("Not connected to Hyperbridge")
		const decimals = api.registry.chainDecimals[0]
		await requireFunded(ctx, keyPair.address, [
			{
				chain: "Hyperbridge Gargantua",
				symbol: api.registry.chainTokens[0],
				decimals,
				min: parseUnits(MIN_BIDDER_BALANCE, decimals),
				read: async () => {
					const account = (await api.query.system.account(keyPair.address)).toJSON() as {
						data: { free: number | string }
					}
					return BigInt(account.data.free)
				},
			},
		])
		unfunded = false
		console.log("KeyPair address:", keyPair.address)
		console.log("Submitting bid with commitment:", testCommitment)
		console.log("UserOp sender:", testUserOp.sender)
		console.log("UserOp nonce:", testUserOp.nonce.toString())
		console.log("Encoded userOp length:", encodedUserOp.length)

		const result = await coprocessor.submitBid(testCommitment, encodedUserOp, BID)

		console.log("Submit bid result:", result)

		if (result.success) {
			console.log("Block hash:", result.blockHash)
			console.log("Extrinsic hash:", result.extrinsicHash)
		} else {
			console.log("Error:", result.error)
		}

		expect(result.success).toBe(true)
	}, 300_000)

	it("should get bid storage entries", async (ctx) => {
		ctx.skip(unfunded, UNFUNDED_NOTE)
		console.log("Getting bid storage entries for commitment:", testCommitment)

		const entries = await coprocessor.getBidStorageEntries(testCommitment)

		console.log("Bid storage entries:", entries)
		console.log("Number of entries:", entries.length)

		for (const entry of entries) {
			console.log("  Filler:", entry.filler)
			console.log("  Deposit:", entry.deposit.toString())
		}

		expect(entries.length).toBeGreaterThan(0)
	}, 300_000)

	it("should get full bids for order with decoded userOp", async (ctx) => {
		ctx.skip(unfunded, UNFUNDED_NOTE)
		console.log("Getting full bids for commitment:", testCommitment)

		const bids = await coprocessor.getBidsForOrder(testCommitment)

		console.log("Number of bids:", bids.length)

		for (const bid of bids) {
			console.log("  Filler:", bid.filler)
			console.log("  Deposit:", bid.deposit.toString())
			console.log("  UserOp sender:", bid.userOp.sender)
			console.log("  UserOp nonce:", bid.userOp.nonce.toString())
			console.log("  UserOp callData:", bid.userOp.callData)
		}

		expect(bids.length).toBeGreaterThan(0)
		// Verify the decoded userOp matches what we submitted
		const ourBid = bids.find((b) => b.userOp.sender.toLowerCase() === testUserOp.sender.toLowerCase())
		expect(ourBid).toBeDefined()
		expect(ourBid?.userOp.nonce).toBe(testUserOp.nonce)
	}, 300_000)

	it("should retract a bid", async (ctx) => {
		ctx.skip(unfunded, UNFUNDED_NOTE)
		console.log("Retracting bid with commitment:", testCommitment)

		const result = await coprocessor.retractBid(testCommitment, BID)

		console.log("Retract bid result:", result)

		if (result.success) {
			console.log("Block hash:", result.blockHash)
			console.log("Extrinsic hash:", result.extrinsicHash)
		} else {
			console.log("Error:", result.error)
		}

		expect(result.success).toBe(true)
	}, 300_000)

	it("should have no bids after retraction", async (ctx) => {
		ctx.skip(unfunded, UNFUNDED_NOTE)
		console.log("Verifying bid was retracted for commitment:", testCommitment)

		const entries = await coprocessor.getBidStorageEntries(testCommitment)

		console.log("Bid storage entries after retraction:", entries.length)

		expect(entries.length).toBe(0)
	}, 300_000)
})

/** Encode UserOp using SCALE codec for Hyperbridge submission */
function encodeUserOp(userOp: PackedUserOperation): HexString {
	return encodeUserOpScale(userOp)
}
