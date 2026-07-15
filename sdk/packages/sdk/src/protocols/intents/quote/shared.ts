import { zeroAddress } from "viem"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"
import type { ChainConfigService } from "@/configs/ChainConfigService"
import type { IntentQuoteChainContext, QuoteIntentParams } from "./types"

type GatewayParamsObject = { protocolFeeBps?: bigint | number | string }
type GatewayParams = GatewayParamsObject | readonly unknown[]
const BPS_DENOMINATOR = 10_000n

export function validateQuoteParams(params: QuoteIntentParams): void {
	const hasAmountIn = params.amountIn !== undefined
	const hasAmountOut = params.amountOut !== undefined
	if (hasAmountIn === hasAmountOut) throw new Error("Provide exactly one of amountIn or amountOut")
	if (params.amountIn !== undefined && params.amountIn <= 0n) throw new Error("amountIn must be greater than zero")
	if (params.amountOut !== undefined && params.amountOut <= 0n) throw new Error("amountOut must be greater than zero")
	if (params.tokenIn.address.toLowerCase() === params.tokenOut.address.toLowerCase()) {
		throw new Error("tokenIn and tokenOut cannot be the same")
	}
}

export async function readProtocolFeeBps(
	chainConfigService: ChainConfigService,
	source: IntentQuoteChainContext,
): Promise<bigint> {
	const gatewayAddress = chainConfigService.getIntentGatewayAddress(source.stateMachineId)
	if (!gatewayAddress || gatewayAddress === "0x" || gatewayAddress === zeroAddress) {
		throw new Error(`IntentGatewayV2 is not configured for chain ${source.stateMachineId}`)
	}

	const gatewayParams = (await source.client.readContract({
		address: gatewayAddress,
		abi: IntentGatewayV2.ABI,
		functionName: "params",
	})) as GatewayParams

	const protocolFeeBps = Array.isArray(gatewayParams)
		? BigInt(gatewayParams[4] as bigint | number | string)
		: BigInt((gatewayParams as GatewayParamsObject).protocolFeeBps ?? 0)
	if (protocolFeeBps < 0n || protocolFeeBps >= BPS_DENOMINATOR) {
		throw new Error(`Invalid IntentGateway protocol fee: ${protocolFeeBps} bps`)
	}
	return protocolFeeBps
}

/** Mirrors the gateway's floored fee deduction. */
export function deductProtocolFee(amount: bigint, protocolFeeBps: bigint): bigint {
	if (protocolFeeBps <= 0n) return amount
	const fee = (amount * protocolFeeBps) / BPS_DENOMINATOR
	return amount - fee
}

/** Conservatively grosses a net amount up so protocol-fee deduction cannot leave it short. */
export function grossUpForProtocolFee(netAmount: bigint, protocolFeeBps: bigint): bigint {
	if (protocolFeeBps <= 0n) return netAmount
	if (protocolFeeBps >= BPS_DENOMINATOR) throw new Error("protocolFeeBps must be less than 10,000")
	return divCeil(netAmount * BPS_DENOMINATOR, BPS_DENOMINATOR - protocolFeeBps)
}

export function divCeil(numerator: bigint, denominator: bigint): bigint {
	if (denominator <= 0n) throw new Error("denominator must be greater than zero")
	return (numerator + denominator - 1n) / denominator
}
