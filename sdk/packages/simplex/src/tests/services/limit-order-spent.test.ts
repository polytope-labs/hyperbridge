import { describe, expect, it } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { createPublicClient, custom, encodeAbiParameters, type PublicClient } from "viem"
import { budgetIdFor } from "@/orderbook/amounts"
import { ContractInteractionService } from "@/services/ContractInteractionService"
import { privateKeySigner } from "@/services/wallet"

/**
 * The read of a limit order's tally off the solver's account.
 *
 * The answer is only a number when the account is delegated to an implementation that
 * keeps the tally. Anything else, from an undelegated account to a dead endpoint, has
 * to come back as "unknown" rather than as a failure of whatever asked.
 */

const SOLVER_KEY = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80" as HexString
const SOLVER = privateKeySigner(SOLVER_KEY).address as HexString
const CHAIN = "EVM-97"
const SPENT = "0xae20bed3"
const BUDGET_ID = budgetIdFor("limit-order-1")

interface Call {
	to: string
	data: string
}

/** A viem client over a node that answers every `eth_call` with `answer`. */
function node(answer: () => Promise<HexString>): { client: PublicClient; calls: Call[] } {
	const calls: Call[] = []
	const client = createPublicClient({
		transport: custom(
			{
				request: async ({ method, params }: { method: string; params: [Call] }) => {
					if (method !== "eth_call") throw new Error(`unexpected ${method}`)
					calls.push(params[0])
					return answer()
				},
			},
			{ retryCount: 0 },
		),
	})
	return { client, calls }
}

function service(getPublicClient: (chain: string) => PublicClient): ContractInteractionService {
	return new ContractInteractionService(
		{ getPublicClient } as any,
		{ loggers: undefined } as any,
		privateKeySigner(SOLVER_KEY),
	)
}

const uint = (value: bigint) => encodeAbiParameters([{ type: "uint256" }], [value])

describe("reading a limit order's tally", () => {
	it("asks the solver's own account what it has spent under the budget's id", async () => {
		const { client, calls } = node(async () => uint(250_500_000n))
		const asked: string[] = []
		const contract = service((chain) => {
			asked.push(chain)
			return client
		})

		expect(await contract.limitOrderSpent(CHAIN, BUDGET_ID)).toBe(250_500_000n)

		expect(asked).toEqual([CHAIN])
		expect(calls).toHaveLength(1)
		expect(calls[0].to.toLowerCase()).toBe(SOLVER.toLowerCase())
		expect(calls[0].data).toBe(`${SPENT}${BUDGET_ID.slice(2)}`)
	})

	it("reads a tally of zero as zero, not as unknown", async () => {
		const { client } = node(async () => uint(0n))

		expect(await service(() => client).limitOrderSpent(CHAIN, BUDGET_ID)).toBe(0n)
	})

	it("is unknown when the account answers with no data", async () => {
		// What an account with no code, or one delegated to nothing, returns.
		const { client, calls } = node(async () => "0x")

		expect(await service(() => client).limitOrderSpent(CHAIN, BUDGET_ID)).toBeNull()
		expect(calls).toHaveLength(1)
	})

	it("is unknown when the call reverts", async () => {
		const { client } = node(async () => {
			throw Object.assign(new Error("execution reverted"), { code: 3, data: "0x" })
		})

		expect(await service(() => client).limitOrderSpent(CHAIN, BUDGET_ID)).toBeNull()
	})

	it("is unknown when the endpoint fails", async () => {
		const { client } = node(async () => {
			throw new Error("socket hang up")
		})

		expect(await service(() => client).limitOrderSpent(CHAIN, BUDGET_ID)).toBeNull()
	})

	it("is unknown when the chain has no client", async () => {
		const contract = service(() => {
			throw new Error("Chain EVM-97 is not configured")
		})

		expect(await contract.limitOrderSpent(CHAIN, BUDGET_ID)).toBeNull()
	})
})
