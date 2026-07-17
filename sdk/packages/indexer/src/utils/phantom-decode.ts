import { Interface, defaultAbiCoder } from "@ethersproject/abi"
import { ethers } from "ethers"
import { FILL_ORDER_ABI, type FillData, type HexString, type RecoverBidSigner } from "@hyperbridge/sdk/intents-helpers"

// VM2-safe decoding and signature recovery for a phantom bid, for the SubQuery substrate sandbox.
//
// The SDK's extractFillData/recoverBidSignerViem use viem, whose @noble/hashes byte handling guards
// with `instanceof Uint8Array`. That throws "Uint8Array expected" inside the VM2 sandbox because the
// global Uint8Array is proxied across realms — it breaks both decodeFunctionData and
// decodeAbiParameters. ethers v5's ABI coder uses js-sha3 keccak and duck-typed byte checks
// (isBytesLike), so it works in the sandbox. These are injected into aggregatePhantomBids so the SDK
// itself stays on the plain viem helpers (used by simplex/tests in Node, where viem is fine).
const executeIface = new Interface(["function execute(bytes32 mode, bytes executionData)"])
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const fillIface = new Interface(FILL_ORDER_ABI as any)
const CALL_TUPLE = ["tuple(address target, uint256 value, bytes data)[]"]

/** Drop-in for the SDK's extractFillData that decodes with ethers (VM2-safe). */
export function extractFillDataVm2(callData: HexString, gatewayAddress: string): FillData | null {
	try {
		const { executionData } = executeIface.decodeFunctionData("execute", callData)
		const [calls] = defaultAbiCoder.decode(CALL_TUPLE, executionData) as unknown as [
			{ target: string; data: string }[],
		]
		const normalized = gatewayAddress.toLowerCase()
		for (const call of calls) {
			if (call.target.toLowerCase() !== normalized) continue
			let decoded
			try {
				decoded = fillIface.decodeFunctionData("fillOrder", call.data)
			} catch {
				continue
			}
			const order = decoded[0] as Record<string, unknown>
			const options = decoded[1] as Record<string, unknown>
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const outputToken = (order as any)?.output?.assets?.[0]?.token as HexString | undefined
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const rawAmount = (options as any)?.outputs?.[0]?.amount
			if (!outputToken || rawAmount === undefined || rawAmount === null) continue
			return { order, options, outputToken, solverAmount: BigInt(rawAmount.toString()) }
		}
	} catch {
		return null
	}
	return null
}

// The EIP-712 payload whose digest is the EntryPoint v0.8 userOpHash. Mirrors the SDK's
// CryptoUtils.packedUserOpTypedData — the two must stay in step or every bid fails verification.
const USER_OP_TYPES = {
	PackedUserOperation: [
		{ name: "sender", type: "address" },
		{ name: "nonce", type: "uint256" },
		{ name: "initCode", type: "bytes" },
		{ name: "callData", type: "bytes" },
		{ name: "accountGasLimits", type: "bytes32" },
		{ name: "preVerificationGas", type: "uint256" },
		{ name: "gasFees", type: "bytes32" },
		{ name: "paymasterAndData", type: "bytes" },
	],
}

/** Drop-in for the SDK's recoverBidSignerViem that hashes and recovers with ethers (VM2-safe). */
export const recoverBidSignerVm2: RecoverBidSigner = async (userOp, entryPoint, chainId, solverSignature) => {
	try {
		const userOpHash = ethers.utils._TypedDataEncoder.hash(
			{ name: "ERC4337", version: "1", chainId: chainId.toString(), verifyingContract: entryPoint },
			USER_OP_TYPES,
			{
				sender: userOp.sender,
				nonce: userOp.nonce.toString(),
				initCode: userOp.initCode,
				callData: userOp.callData,
				accountGasLimits: userOp.accountGasLimits,
				preVerificationGas: userOp.preVerificationGas.toString(),
				gasFees: userOp.gasFees,
				paymasterAndData: userOp.paymasterAndData,
			},
		)
		return ethers.utils.recoverAddress(userOpHash, solverSignature) as HexString
	} catch {
		return null
	}
}
