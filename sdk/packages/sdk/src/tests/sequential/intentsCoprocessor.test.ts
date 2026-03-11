import "log-timestamp"
import { IntentsCoprocessor, encodeUserOpScale } from "@/chains/intentsCoprocessor"
import { SubstrateChain } from "@/chains/substrate"
import type { HexString, PackedUserOperation } from "@/types"
import { keccak256, toHex } from "viem"
import fixtureData from "@/tests/fixtures/intent-gateway-v2.json"

describe.sequential("IntentsCoprocessor", () => {
	let hyperbridge: SubstrateChain
	let coprocessor: IntentsCoprocessor

	const testCommitment = keccak256(toHex(`test-${Date.now()}-${Math.random()}`)) as HexString

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

	beforeAll(async () => {
		hyperbridge = new SubstrateChain({
			wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			hasher: "Keccak",
			stateMachineId: "KUSAMA-4009",
			consensusStateId: "PAS0",
		})
		await hyperbridge.connect()
		console.log("Connected to Hyperbridge")

		const privateKey = process.env.SECRET_PHRASE!
		coprocessor = IntentsCoprocessor.fromSubstrateChain(hyperbridge, privateKey)

		console.log("Test commitment:", testCommitment)
		console.log("Test userOp sender:", testUserOp.sender)
	})

	afterAll(async () => {
		await hyperbridge.disconnect()
		console.log("Disconnected")
	})

	it("should submit a bid with realistic userOp", async () => {
		const keyPair = coprocessor.getKeyPair()
		console.log("KeyPair address:", keyPair.address)
		console.log("Submitting bid with commitment:", testCommitment)
		console.log("UserOp sender:", testUserOp.sender)
		console.log("UserOp nonce:", testUserOp.nonce.toString())
		console.log("Encoded userOp length:", encodedUserOp.length)

		const result = await coprocessor.submitBid(testCommitment, encodedUserOp)

		console.log("Submit bid result:", result)

		if (result.success) {
			console.log("Block hash:", result.blockHash)
			console.log("Extrinsic hash:", result.extrinsicHash)
		} else {
			console.log("Error:", result.error)
		}

		expect(result.success).toBe(true)
	}, 300_000)

	it("should get bid storage entries", async () => {
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

	it("should get full bids for order with decoded userOp", async () => {
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

	it("should retract a bid", async () => {
		console.log("Retracting bid with commitment:", testCommitment)

		const result = await coprocessor.retractBid(testCommitment)

		console.log("Retract bid result:", result)

		if (result.success) {
			console.log("Block hash:", result.blockHash)
			console.log("Extrinsic hash:", result.extrinsicHash)
		} else {
			console.log("Error:", result.error)
		}

		expect(result.success).toBe(true)
	}, 300_000)

	it("should have no bids after retraction", async () => {
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
