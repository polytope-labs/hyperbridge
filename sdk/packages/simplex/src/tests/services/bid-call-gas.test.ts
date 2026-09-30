import { describe, expect, it } from "vitest"
import type { ERC7821Call, HexString, Order } from "@hyperbridge/sdk"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"

/**
 * The call gas each bid on an order signs.
 *
 * Every bid on one order shares the order's gas estimate, but each draws on its
 * own limit order and so prepends its own funding calls, which are not simulated.
 * The allowance for them is added when a bid is signed, from that bid's calls:
 * baked into the shared estimate, every bid would carry whichever calls happened
 * to be cached when the estimate was taken.
 */

const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const BASE_CALL_GAS = 1_000_000n
const FUNDING_GAS_PER_CALL = 400_000n

function service(): ContractInteractionService {
	// biome-ignore lint/suspicious/noExplicitAny: nothing on this path reads a chain or config
	return new ContractInteractionService({} as any, { loggers: undefined } as any, privateKeySigner(SOLVER_KEY))
}

const call = (data: HexString): ERC7821Call => ({ target: "0x0000000000000000000000000000000000000001", value: 0n, data })
const order = { id: "0xorder" } as Order

function callGasFor(contract: ContractInteractionService): bigint {
	// biome-ignore lint/suspicious/noExplicitAny: the allowance is what is under test
	return (contract as any).callGasLimitFor(order, BASE_CALL_GAS)
}

describe("the call gas a bid signs", () => {
	it("is the shared estimate when the bid prepends nothing", () => {
		const contract = service()
		contract.cacheService.clearFundingPrepends(order.id!)
		expect(callGasFor(contract)).toBe(BASE_CALL_GAS)
	})

	it("follows the funding calls of the bid being signed", () => {
		const contract = service()

		// One bid on the order draws on a venue twice, the next not at all. Each is
		// set before its bid is signed, and each bid gets its own allowance.
		contract.cacheService.setFundingPrepends(order.id!, [call("0x01"), call("0x02")])
		expect(callGasFor(contract)).toBe(BASE_CALL_GAS + 2n * FUNDING_GAS_PER_CALL)

		contract.cacheService.clearFundingPrepends(order.id!)
		expect(callGasFor(contract)).toBe(BASE_CALL_GAS)
	})
})
