/**
 * IntentGatewayV2 Encoding Tests
 *
 * Verifies SDK encoding functions produce identical outputs to the Solidity contract.
 * Test vectors generated from Foundry tests.
 */
import { describe, it, expect, beforeAll } from "vitest"
import { keccak256, concat, type Hex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import {
	orderV2Commitment,
	SELECT_SOLVER_TYPEHASH,
	IntentGatewayV2,
	EvmChain,
	type OrderV2,
	type HexString,
	type PackedUserOperation,
	type FillOptionsV2,
	type SubmitBidOptions,
} from "@/index"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { SessionKeyData } from "@/storage"

import testVectors from "../fixtures/intent-gateway-v2.json"

let intentGatewayV2: IntentGatewayV2
let chainConfigService: ChainConfigService

beforeAll(async () => {
	chainConfigService = new ChainConfigService()

	const sourceEvmChain = new EvmChain({
		chainId: 1,
		host: chainConfigService.getHostAddress("EVM-1"),
		rpcUrl: process.env.ETH_MAINNET || "https://eth.llamarpc.com",
	})

	const destEvmChain = new EvmChain({
		chainId: 56,
		host: chainConfigService.getHostAddress("EVM-56"),
		rpcUrl: process.env.BSC_MAINNET || "https://bsc-dataseed.binance.org",
	})

	intentGatewayV2 = new IntentGatewayV2(sourceEvmChain, destEvmChain)
})



describe("orderV2Commitment", () => {
	it.each(testVectors.orderCommitmentVectors)(
		"$name: matches Solidity",
		(vector) => {
			const order = buildOrderFromFixture(vector.order)
			const commitment = orderV2Commitment(order)
			expect(commitment.toLowerCase()).toBe(vector.commitment.toLowerCase())
		}
	)
})

describe("SELECT_SOLVER_TYPEHASH", () => {
	it("matches Solidity constant", () => {
		expect(SELECT_SOLVER_TYPEHASH.toLowerCase()).toBe(
			testVectors.eip712SignatureVectors[0].SELECT_SOLVER_TYPEHASH.toLowerCase()
		)
	})
})


describe("IntentGatewayV2.computeUserOpHash", () => {
	it.each(testVectors.userOpHashVectors)(
		"$name: matches Solidity",
		(vector) => {
			const userOp: PackedUserOperation = {
				sender: vector.userOp.sender as HexString,
				nonce: BigInt(vector.userOp.nonce),
				initCode: vector.userOp.initCode as HexString,
				callData: vector.userOp.callData as HexString,
				accountGasLimits: vector.userOp.accountGasLimits as HexString,
				preVerificationGas: BigInt(vector.userOp.preVerificationGas),
				gasFees: vector.userOp.gasFees as HexString,
				paymasterAndData: vector.userOp.paymasterAndData as HexString,
				signature: "0x" as HexString,
			}

			const hash = intentGatewayV2.computeUserOpHash(
				userOp,
				vector.entryPoint as HexString,
				BigInt(vector.chainId)
			)

			expect(hash.toLowerCase()).toBe(vector.userOpHash.toLowerCase())
		}
	)
})

describe("IntentGatewayV2.signSolverSelection", () => {
	it.each(testVectors.eip712SignatureVectors)(
		"$name: returns correct signature",
		async (vector) => {
			const sessionKeyData: SessionKeyData = {
				privateKey: vector.sessionKeyPrivateKey as HexString,
				address: vector.sessionKeyAddress as HexString,
				commitment: vector.commitment as HexString,
				createdAt: Date.now(),
			}

			const signature = await intentGatewayV2.signSolverSelection(
				vector.commitment as HexString,
				vector.solver as HexString,
				vector.domainSeparator as HexString,
				sessionKeyData
			)

			expect(signature).not.toBeNull()
			expect(signature!.toLowerCase()).toBe(vector.signature.toLowerCase())
		}
	)
})

describe("IntentGatewayV2.prepareSubmitBid", () => {
	it("returns valid PackedUserOperation with correct signature", async () => {
		const orderVector = testVectors.orderCommitmentVectors[0]
		const gasVector = testVectors.gasPackingVectors[0]
		const bidVector = testVectors.bidSignatureVectors[0]
		const userOpVector = testVectors.userOpHashVectors[0]

		const order = buildOrderFromFixture(orderVector.order)

		const fillOptions: FillOptionsV2 = {
			relayerFee: 0n,
			nativeDispatchFee: 0n,
			outputs: order.output.assets,
		}

		const submitBidOptions: SubmitBidOptions = {
			order,
			fillOptions,
			solverAccount: bidVector.solverAddress as HexString,
			solverPrivateKey: bidVector.solverPrivateKey as HexString,
			nonce: BigInt(userOpVector.userOp.nonce),
			entryPointAddress: userOpVector.entryPoint as HexString,
			callGasLimit: BigInt(gasVector.callGasLimit),
			verificationGasLimit: BigInt(gasVector.verificationGasLimit),
			preVerificationGas: BigInt(userOpVector.userOp.preVerificationGas),
			maxFeePerGas: BigInt(gasVector.maxFeePerGas),
			maxPriorityFeePerGas: BigInt(gasVector.maxPriorityFeePerGas),
		}

		const userOp = await intentGatewayV2.prepareSubmitBid(submitBidOptions)

		// Verify structure
		expect(userOp.sender).toBe(bidVector.solverAddress)
		expect(userOp.nonce).toBe(BigInt(userOpVector.userOp.nonce))
		expect(userOp.initCode).toBe("0x")
		expect(userOp.paymasterAndData).toBe("0x")

		// Verify gas packing
		expect(userOp.accountGasLimits.toLowerCase()).toBe(gasVector.accountGasLimits_erc4337.toLowerCase())
		expect(userOp.gasFees.toLowerCase()).toBe(gasVector.gasFees.toLowerCase())

		// Verify signature format: commitment (32 bytes) + solverSignature (65 bytes) = 97 bytes = 196 hex chars
		expect(userOp.signature.length).toBe(196)

		// Verify commitment in signature
		const signatureCommitment = userOp.signature.slice(0, 66)
		const expectedCommitment = orderV2Commitment(order)
		expect(signatureCommitment.toLowerCase()).toBe(expectedCommitment.toLowerCase())

		// Verify solver signature
		const solverSignature = ("0x" + userOp.signature.slice(66)) as Hex
		const userOpHash = intentGatewayV2.computeUserOpHash(
			{ ...userOp, signature: "0x" as HexString },
			userOpVector.entryPoint as HexString,
			56n
		)
		const messageHash = keccak256(concat([userOpHash, expectedCommitment as Hex, order.session as Hex]))

		const solverAccount = privateKeyToAccount(bidVector.solverPrivateKey as Hex)
		const expectedSolverSig = await solverAccount.signMessage({ message: { raw: messageHash } })
		expect(solverSignature.toLowerCase()).toBe(expectedSolverSig.toLowerCase())
	})
})


function buildOrderFromFixture(orderFixture: any): OrderV2 {
	return {
		user: orderFixture.user as HexString,
		source: orderFixture.source as string,
		destination: orderFixture.destination as string,
		deadline: BigInt(orderFixture.deadline),
		nonce: BigInt(orderFixture.nonce),
		fees: BigInt(orderFixture.fees),
		session: orderFixture.session as HexString,
		predispatch: {
			assets: orderFixture.predispatch.assets.map((a: any) => ({
				token: a.token as HexString,
				amount: BigInt(a.amount),
			})),
			call: orderFixture.predispatch.call as HexString,
		},
		inputs: orderFixture.inputs.map((i: any) => ({
			token: i.token as HexString,
			amount: BigInt(i.amount),
		})),
		output: {
			beneficiary: orderFixture.output.beneficiary as HexString,
			assets: orderFixture.output.assets.map((a: any) => ({
				token: a.token as HexString,
				amount: BigInt(a.amount),
			})),
			call: orderFixture.output.call as HexString,
		},
	}
}
