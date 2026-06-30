import { Interface, defaultAbiCoder } from "@ethersproject/abi"
import { FILL_ORDER_ABI, type FillData, type HexString } from "@hyperbridge/sdk/intents-helpers"

// VM2-safe decoding of a phantom bid's ERC-7821 calldata for the SubQuery substrate sandbox.
//
// The SDK's extractFillData uses viem, whose @noble/hashes byte handling guards with
// `instanceof Uint8Array`. That throws "Uint8Array expected" inside the VM2 sandbox because the
// global Uint8Array is proxied across realms — it breaks both decodeFunctionData and
// decodeAbiParameters. ethers v5's ABI coder uses js-sha3 keccak and duck-typed byte checks
// (isBytesLike), so it works in the sandbox. This is injected into aggregatePhantomBids so the SDK
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
