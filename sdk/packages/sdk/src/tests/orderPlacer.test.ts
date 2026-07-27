import { deriveCanonicalPlacedOrder } from "@/protocols/intents/OrderPlacer"
import { orderCommitment } from "@/protocols/intents/utils"
import type { DecodedOrderPlacedLog, HexString, Order } from "@/types"
import { describe, expect, it } from "vitest"

const ADDRESS = "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString
const SESSION = "0x1111111111111111111111111111111111111111" as HexString

function makeSubmittedOrder(): Order {
	return {
		user: "0x0000000000000000000000000000000000000000000000000000000000000000",
		source: "EVM-8453",
		destination: "EVM-8453",
		deadline: 100n,
		nonce: 0n,
		fees: 0n,
		session: SESSION,
		predispatch: { assets: [{ token: ADDRESS, amount: 10n }], call: "0x1234" },
		inputs: [{ token: ADDRESS, amount: 1_000n }],
		output: { beneficiary: ADDRESS, assets: [{ token: ADDRESS, amount: 990n }], call: "0xabcd" },
	}
}

describe("deriveCanonicalPlacedOrder", () => {
	it("uses the complete OrderPlaced state to calculate the commitment", () => {
		const submitted = makeSubmittedOrder()
		const args: DecodedOrderPlacedLog["args"] = {
			user: ADDRESS,
			source: "EVM-8453",
			destination: "EVM-8453",
			deadline: 120n,
			nonce: 42n,
			fees: 5n,
			session: SESSION,
			beneficiary: ADDRESS,
			predispatch: [{ token: ADDRESS, amount: 8n }],
			inputs: [{ token: ADDRESS, amount: 995n }],
			outputs: [{ token: ADDRESS, amount: 990n }],
	}

		const placed = deriveCanonicalPlacedOrder(submitted, args)
		const expected: Order = {
			...submitted,
			user: ADDRESS,
			deadline: 120n,
			nonce: 42n,
			fees: 5n,
			predispatch: { assets: [{ token: ADDRESS, amount: 8n }], call: "0x1234" },
			inputs: [{ token: ADDRESS, amount: 995n }],
			output: { beneficiary: ADDRESS, assets: [{ token: ADDRESS, amount: 990n }], call: "0xabcd" },
		}

		expect(placed).toMatchObject(expected)
		expect(placed.id).toBe(orderCommitment(expected))
		expect(placed.id).not.toBe(orderCommitment(submitted))
		expect(submitted.user).toBe("0x0000000000000000000000000000000000000000000000000000000000000000")
		expect(submitted.inputs[0].amount).toBe(1_000n)
	})
})
