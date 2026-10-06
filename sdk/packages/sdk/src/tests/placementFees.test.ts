import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { IEvmChain } from "@/chain"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { DEFAULT_GRAFFITI } from "@/protocols/intents/types"
import type { FillOrderEstimate, HexString, IntentOrderStatusUpdate, Order } from "@/types"
import { bytes20ToBytes32 } from "@/utils"
import { decodeFunctionData } from "viem"
import { describe, expect, it, vi } from "vitest"

vi.mock("@/storage/load-driver", () => ({ loadDriver: () => undefined }))

const BASE = "EVM-8453"
const ARBITRUM = "EVM-42161"
const BASE_FEE_TOKEN = "0x00000000000000000000000000000000000000Ba" as HexString
const ARBITRUM_FEE_TOKEN = "0x00000000000000000000000000000000000000A4" as HexString
const BENEFICIARY = "0xEa4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString
const INPUT_TOKEN = "0x1111111111111111111111111111111111111111" as HexString
const OUTPUT_TOKEN = "0x2222222222222222222222222222222222222222" as HexString

const configService = new ChainConfigService({})

function makeChain(stateMachineId: string, feeToken: HexString): IEvmChain {
	return {
		config: { stateMachineId },
		configService,
		client: { getCode: vi.fn().mockResolvedValue("0x") },
		getFeeTokenWithDecimals: vi.fn().mockResolvedValue({ address: feeToken, decimals: 6 }),
	} as unknown as IEvmChain
}

function makeOrder(source: string, destination: string, fees: bigint): Order {
	return {
		user: BENEFICIARY,
		source,
		destination,
		deadline: 65337297000n,
		nonce: 0n,
		fees,
		session: "0x0000000000000000000000000000000000000000",
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: bytes20ToBytes32(INPUT_TOKEN), amount: 1_000_000n }],
		output: {
			beneficiary: BENEFICIARY,
			assets: [{ token: bytes20ToBytes32(OUTPUT_TOKEN), amount: 1_000_000n }],
			call: "0x",
		},
	}
}

const ESTIMATE: FillOrderEstimate = {
	fillOptions: { relayerFee: 0n, nativeDispatchFee: 0n, validUntil: 0n, outputs: [], inputs: [] },
	inputs: [],
	callGasLimit: 500_000n,
	verificationGasLimit: 100_000n,
	preVerificationGas: 100_000n,
	paymasterVerificationGasLimit: 0n,
	paymasterPostOpGasLimit: 0n,
	maxFeePerGas: 1_000_000_000n,
	maxPriorityFeePerGas: 1_000_000n,
	totalGasCostWei: 700_000_000_000_000n,
	totalGasInFeeToken: 2_500n,
	relayerFeeInSourceFeeToken: 0n,
}

type Placement = Extract<IntentOrderStatusUpdate, { status: "AWAITING_PLACE_ORDER" }>

async function firstPlacement(
	gateway: IntentGateway,
	method: "execute" | "executeBest",
	order: Order,
): Promise<Placement> {
	const generator = gateway[method](order, DEFAULT_GRAFFITI, { auctionTimeMs: 1 })
	try {
		const result = await generator.next()
		if (result.done || result.value.status !== "AWAITING_PLACE_ORDER") {
			throw new Error(`Expected ${method} to yield AWAITING_PLACE_ORDER first`)
		}
		return result.value
	} finally {
		await generator.return(undefined)
	}
}

function encodedFee(data: HexString): bigint {
	const decoded = decodeFunctionData({ abi: IntentGatewayV2ABI, data })
	expect(decoded.functionName).toBe("placeOrder")
	return (decoded.args?.[0] as { fees: bigint }).fees
}

describe.each(["execute", "executeBest"] as const)("IntentGateway.%s placement fee metadata", (method) => {
	it("exposes the source fee token and the exact encoded fee", async () => {
		const gateway = await IntentGateway.create(
			makeChain(BASE, BASE_FEE_TOKEN),
			makeChain(ARBITRUM, ARBITRUM_FEE_TOKEN),
		)
		const order = makeOrder(BASE, ARBITRUM, 1n)

		const placement = await firstPlacement(gateway, method, order)

		expect(placement.to).toBe(configService.getIntentGatewayAddress(BASE))
		expect(placement.to).not.toBe("0x")
		expect(placement.feeTokenAddress).toBe(BASE_FEE_TOKEN)
		expect(placement.feeTokenAmount).toBe(1n)
		expect(placement.nativeFee).toBe(0n)
		expect(placement.value).toBe(0n)
		expect(placement.sessionPrivateKey).toMatch(/^0x[\da-f]{64}$/i)
		expect(encodedFee(placement.data)).toBe(1n)
	})

	it("prices a zero-fee same-chain order at twice the fill gas with a 2% native buffer", async () => {
		const chain = makeChain(BASE, BASE_FEE_TOKEN)
		const gateway = await IntentGateway.create(chain, chain)
		// biome-ignore lint/suspicious/noExplicitAny: the estimator is private; this pins its answer
		vi.spyOn((gateway as any).gasEstimator, "estimateFillOrder").mockResolvedValue(ESTIMATE)

		const placement = await firstPlacement(gateway, method, makeOrder(BASE, BASE, 0n))

		expect(placement.feeTokenAddress).toBe(BASE_FEE_TOKEN)
		expect(placement.feeTokenAmount).toBe(5_000n)
		expect(placement.nativeFee).toBe(714_000_000_000_000n)
		expect(placement.value).toBe(0n)
		expect(encodedFee(placement.data)).toBe(5_000n)
	})
})
