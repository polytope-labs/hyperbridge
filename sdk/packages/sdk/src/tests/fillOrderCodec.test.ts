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

	const LEGACY_IMPL = "0x976B268b06f545c4A2BF44866Aa2465bd8B3C67d" as HexString
	const NEW_IMPL = "0x2222222222222222222222222222222222222222" as HexString

	function slotFor(impl: HexString) {
		return `0x000000000000000000000000${impl.slice(2)}` as HexString
	}

	function client(impl: HexString | undefined) {
		return {
			chain: { id: 8453 },
			getStorageAt: vi.fn().mockResolvedValue(impl === undefined ? undefined : slotFor(impl)),
		} as any
	}

	it("reports v1 on a chain that has not been redeployed, without reading the slot", async () => {
		// Base Sepolia runs a pre-validUntil gateway whose implementation address is not
		// tracked, so the address check would wrongly read it as current.
		const c = client(NEW_IMPL)
		c.chain = { id: 84532 }

		await expect(getFillOptionsVersion(c, GATEWAY)).resolves.toBe(1)
		expect(c.getStorageAt).not.toHaveBeenCalled()
	})

	it("reports v1 for the known pre-validUntil implementation", async () => {
		await expect(getFillOptionsVersion(client(LEGACY_IMPL), GATEWAY)).resolves.toBe(1)
	})

	it("matches the legacy implementation regardless of address casing", async () => {
		// The slot yields lowercase; the list is written checksummed in the source it came from.
		const lower = LEGACY_IMPL.toLowerCase() as HexString
		await expect(getFillOptionsVersion(client(lower), GATEWAY)).resolves.toBe(1)
	})

	it("defaults to v2 for any implementation not on the legacy list", async () => {
		// Listing legacy rather than current is what makes this the safe default: a newly
		// shipped implementation needs no edit here.
		await expect(getFillOptionsVersion(client(NEW_IMPL), GATEWAY)).resolves.toBe(2)
	})

	it("reads the implementation through the ERC-1967 slot", async () => {
		const c = client(NEW_IMPL)

		await getFillOptionsVersion(c, GATEWAY)

		expect(c.getStorageAt).toHaveBeenCalledWith({ address: GATEWAY, slot: ERC1967_SLOT })
	})

	it("treats a gateway that is not a proxy as its own implementation", async () => {
		const c = client(undefined)

		await expect(getFillOptionsVersion(c, GATEWAY)).resolves.toBe(2)
	})

	it("does not cache a v1 answer, so the upgrade that fixes it is picked up", async () => {
		const c = client(LEGACY_IMPL)
		expect(await getFillOptionsVersion(c, GATEWAY)).toBe(1)

		// Same proxy, upgraded behind it.
		c.getStorageAt.mockResolvedValue(slotFor(NEW_IMPL))
		expect(await getFillOptionsVersion(c, GATEWAY)).toBe(2)
	})

	it("caches a v2 answer, which can never regress", async () => {
		const c = client(NEW_IMPL)

		await getFillOptionsVersion(c, GATEWAY)
		await getFillOptionsVersion(c, GATEWAY)

		expect(c.getStorageAt).toHaveBeenCalledTimes(1)
	})
})
