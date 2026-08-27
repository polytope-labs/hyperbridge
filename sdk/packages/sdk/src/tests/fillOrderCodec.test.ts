import { describe, it, expect, beforeEach, vi } from "vitest"
import { slice } from "viem"
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
const ERC1967_SLOT = "0x360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc"

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

	const IMPL = "0x2222222222222222222222222222222222222222" as HexString
	const IMPL_SLOT_VALUE = `0x000000000000000000000000${IMPL.slice(2)}` as HexString

	/** A client whose proxy points at IMPL, and whose IMPL carries the given selectors. */
	function client(selectors: string[], opts: { slot?: HexString | undefined } = {}) {
		const code = `0x6080${selectors.map((s) => s.slice(2)).join("dead")}` as HexString
		return {
			chain: { id: 8453 },
			getStorageAt: vi.fn().mockResolvedValue("slot" in opts ? opts.slot : IMPL_SLOT_VALUE),
			getCode: vi.fn().mockResolvedValue(code),
		} as any
	}

	it("reports v2 when the implementation carries the v2 fillOrder selector", async () => {
		await expect(getFillOptionsVersion(client([V2_SELECTOR]), GATEWAY)).resolves.toBe(2)
	})

	it("reports v1 when only the older selector is present", async () => {
		await expect(getFillOptionsVersion(client([V1_SELECTOR]), GATEWAY)).resolves.toBe(1)
	})

	it("looks past the proxy at the implementation's code", async () => {
		// getCode on the gateway returns the proxy stub, which carries neither selector.
		const c = client([V2_SELECTOR])

		await getFillOptionsVersion(c, GATEWAY)

		expect(c.getStorageAt).toHaveBeenCalledWith({ address: GATEWAY, slot: ERC1967_SLOT })
		expect(c.getCode).toHaveBeenCalledWith({ address: IMPL })
	})

	it("falls back to the gateway itself when it is not a proxy", async () => {
		const c = client([V2_SELECTOR], { slot: undefined })

		await expect(getFillOptionsVersion(c, GATEWAY)).resolves.toBe(2)
		expect(c.getCode).toHaveBeenCalledWith({ address: GATEWAY })
	})

	it("throws rather than guessing when neither selector is present", async () => {
		// Guessing would break every fill on the chain with a confusing revert instead of
		// a clear message here — the two shapes cannot decode each other.
		await expect(getFillOptionsVersion(client(["0xdeadbe01"]), GATEWAY)).rejects.toThrow(
			/No fillOrder selector found/,
		)
	})

	it("caches per implementation, so an upgrade is picked up rather than pinned", async () => {
		const c = client([V1_SELECTOR])
		await getFillOptionsVersion(c, GATEWAY)
		await getFillOptionsVersion(c, GATEWAY)
		expect(c.getCode).toHaveBeenCalledTimes(1)

		// Same proxy, new implementation behind it: the answer must move with it.
		const upgraded = "0x3333333333333333333333333333333333333333" as HexString
		c.getStorageAt.mockResolvedValue(`0x000000000000000000000000${upgraded.slice(2)}`)
		c.getCode.mockResolvedValue(`0x6080${V2_SELECTOR.slice(2)}`)

		await expect(getFillOptionsVersion(c, GATEWAY)).resolves.toBe(2)
	})
})
