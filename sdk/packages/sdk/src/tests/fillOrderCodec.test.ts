import { describe, it, expect, beforeEach, vi } from "vitest"
import { slice, ContractFunctionRevertedError, ContractFunctionZeroDataError } from "viem"
import {
	encodeFillOrder,
	decodeFillOrder,
	getFillOptionsVersion,
	resetFillOptionsVersionCache,
} from "@/protocols/intents/fillOrderCodec"
import type { FillOptions, HexString, Order } from "@/types"

/**
 * `FillOptions` gained `validUntil`, which changes `fillOrder`'s selector. Both shapes are on
 * the wire at once because gateways upgrade per chain, so the codec has to encode whichever
 * a deployment speaks and decode either.
 */

const GATEWAY = "0x1111111111111111111111111111111111111111" as HexString
const TOKEN = "0x0000000000000000000000000000000000000000000000000000000000000002" as HexString

// Selectors computed from the canonical signatures; pinned so a struct edit that silently
// changes the ABI shows up here rather than as a reverting fill.
const V1_SELECTOR = "0x5cfb1ea5"
const V2_SELECTOR = "0xa5470064"

function order(): Order {
	return {
		user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
		source: "0x",
		destination: "0x",
		deadline: 100n,
		nonce: 1n,
		fees: 0n,
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" },
		inputs: [{ token: TOKEN, amount: 1_000n }],
		output: { beneficiary: ("0x" + "00".repeat(32)) as HexString, assets: [{ token: TOKEN, amount: 0n }], call: "0x" },
	} as unknown as Order
}

function options(validUntil: bigint): FillOptions {
	return { relayerFee: 0n, nativeDispatchFee: 0n, validUntil, outputs: [{ token: TOKEN, amount: 500n }] }
}

function client(readContract: any, chainId = 8453) {
	return { chain: { id: chainId }, readContract } as any
}

/**
 * A stand-in for what viem throws. The probe classifies by `instanceof` on the cause, so the
 * cause has to be a genuine instance of viem's class — constructing via `Object.create`
 * avoids the constructors' required ABI arguments while keeping `instanceof` honest.
 */
function contractError(kind: "revert" | "zero-data") {
	const cause = Object.create(
		(kind === "revert" ? ContractFunctionRevertedError : ContractFunctionZeroDataError).prototype,
	)
	const err: any = new Error(kind === "revert" ? "execution reverted" : "returned no data")
	err.walk = (fn: (e: unknown) => boolean) => (fn(cause) ? cause : undefined)
	return err
}

describe("encodeFillOrder", () => {
	it("emits the v2 selector and round-trips validUntil", () => {
		const data = encodeFillOrder(order(), options(999n), 2)

		expect(slice(data, 0, 4)).toBe(V2_SELECTOR)
		expect(decodeFillOrder(data)!.options.validUntil).toBe(999n)
	})

	it("emits the v1 selector and drops validUntil for an older gateway", () => {
		const data = encodeFillOrder(order(), options(999n), 1)

		expect(slice(data, 0, 4)).toBe(V1_SELECTOR)
		// Nowhere to carry the bound, and nothing on the other side to enforce it.
		expect(decodeFillOrder(data)!.options.validUntil).toBe(0n)
	})

	it("gives the two shapes different selectors, so a v2 payload cannot be mis-decoded by a v1 gateway", () => {
		expect(slice(encodeFillOrder(order(), options(1n), 1), 0, 4)).not.toBe(
			slice(encodeFillOrder(order(), options(1n), 2), 0, 4),
		)
	})
})

describe("decodeFillOrder", () => {
	it("reads back the outputs from either shape", () => {
		for (const version of [1, 2] as const) {
			const decoded = decodeFillOrder(encodeFillOrder(order(), options(7n), version))
			expect(decoded!.options.outputs[0].amount).toBe(500n)
		}
	})

	it("returns null for calldata that is not a fillOrder call", () => {
		expect(decodeFillOrder("0xdeadbeef" as HexString)).toBeNull()
	})
})

describe("getFillOptionsVersion", () => {
	beforeEach(() => resetFillOptionsVersionCache())

	it("reports v2 when the gateway implements fillOptionsVersion", async () => {
		const readContract = vi.fn().mockResolvedValue(2n)

		await expect(getFillOptionsVersion(client(readContract), GATEWAY)).resolves.toBe(2)
	})

	it("treats a revert as v1 — the gateway predates the function", async () => {
		const readContract = vi.fn().mockRejectedValue(contractError("revert"))

		await expect(getFillOptionsVersion(client(readContract), GATEWAY)).resolves.toBe(1)
	})

	it("treats an empty return as v1", async () => {
		const readContract = vi.fn().mockRejectedValue(contractError("zero-data"))

		await expect(getFillOptionsVersion(client(readContract), GATEWAY)).resolves.toBe(1)
	})

	it("rethrows a transport error instead of caching a downgrade", async () => {
		// Caching v1 here would silently strip validUntil from every later fill on this chain
		// for the life of the process, on nothing more than one flaky RPC call.
		const transport: any = new Error("HTTP 429")
		transport.walk = () => undefined
		const readContract = vi.fn().mockRejectedValue(transport)

		await expect(getFillOptionsVersion(client(readContract), GATEWAY)).rejects.toThrow("HTTP 429")
	})

	it("memoises per deployment, and does not confuse two chains", async () => {
		const readContract = vi.fn().mockResolvedValue(2n)
		const c1 = client(readContract, 8453)

		await getFillOptionsVersion(c1, GATEWAY)
		await getFillOptionsVersion(c1, GATEWAY)
		expect(readContract).toHaveBeenCalledTimes(1)

		await getFillOptionsVersion(client(readContract, 1), GATEWAY)
		expect(readContract).toHaveBeenCalledTimes(2)
	})
})
