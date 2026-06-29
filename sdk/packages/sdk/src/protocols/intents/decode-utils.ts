// Pure ERC-7821 encode/decode utilities — no chain dependency (safe for SubQuery VM2).
import { encodeFunctionData, encodeAbiParameters, decodeFunctionData, decodeAbiParameters } from "viem"
import ERC7821ABI from "@/abis/erc7281"
import { ERC7821_BATCH_MODE } from "./types"
import type { HexString, ERC7821Call } from "@/types"

export function encodeERC7821ExecuteBatch(calls: ERC7821Call[]): HexString {
	const executionData = encodeAbiParameters(
		[{ type: "tuple[]", components: ERC7821ABI.ABI[1].components }],
		[calls.map((call) => ({ target: call.target, value: call.value, data: call.data }))],
	) as HexString

	return encodeFunctionData({
		abi: ERC7821ABI.ABI,
		functionName: "execute",
		args: [ERC7821_BATCH_MODE, executionData],
	}) as HexString
}

export function decodeERC7821ExecuteBatch(callData: HexString): ERC7821Call[] | null {
	try {
		const decoded = decodeFunctionData({ abi: ERC7821ABI.ABI, data: callData })
		if (decoded.functionName !== "execute" || !decoded.args || decoded.args.length < 2) return null
		const executionData = decoded.args[1] as HexString
		const [calls] = decodeAbiParameters(
			[{ type: "tuple[]", components: ERC7821ABI.ABI[1].components }],
			executionData,
		) as [ERC7821Call[]]
		return calls.map((call) => ({
			target: call.target as HexString,
			value: call.value,
			data: call.data as HexString,
		}))
	} catch {
		return null
	}
}
