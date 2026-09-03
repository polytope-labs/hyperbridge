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
	it("uses the complete OrderPlaced state, preferring the event's call payloads", () => {
		const submitted = makeSubmittedOrder()
		// The event's call payloads deliberately differ from the submitted
		// order's, so this fails if the event values stop taking precedence.
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
			predispatchCall: "0x9999",
			outputCall: "0x8888",
			graffiti: "0x0000000000000000000000000000000000000000000000000000000000000001",
		}

		const placed = deriveCanonicalPlacedOrder(submitted, args)
		const expected: Order = {
			...submitted,
			user: ADDRESS,
			deadline: 120n,
			nonce: 42n,
			fees: 5n,
			predispatch: { assets: [{ token: ADDRESS, amount: 8n }], call: "0x9999" },
			inputs: [{ token: ADDRESS, amount: 995n }],
			output: { beneficiary: ADDRESS, assets: [{ token: ADDRESS, amount: 990n }], call: "0x8888" },
		}

		expect(placed).toMatchObject(expected)
		expect(placed.id).toBe(orderCommitment(expected))
		expect(placed.id).not.toBe(orderCommitment(submitted))
		expect(submitted.user).toBe("0x0000000000000000000000000000000000000000000000000000000000000000")
		expect(submitted.inputs[0].amount).toBe(1_000n)
	})

	it("falls back to the submitted order's call payloads when the event omits them", () => {
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

		expect(placed.predispatch.call).toBe("0x1234")
		expect(placed.output.call).toBe("0xabcd")
	})
})
