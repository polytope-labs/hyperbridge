import { describe, it, expect, vi } from "vitest"
import { slice, keccak256, createPublicClient, custom, type PublicClient } from "viem"
import { baseSepolia } from "viem/chains"
import {
	encodeFillOrder,
	FILL_ORDER_V2_ABI,
	FILL_ORDER_V3_SELECTOR,
	decodeFillOrder,
	getFillOptionsVersion,
	supportsRateFills,
} from "@/protocols/intents/fillOrderCodec"
import type { HexString, Order, TokenInfo } from "@/types"

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
const RATE_SELECTOR = "0x68ddf058"
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
		output: {
			beneficiary: ("0x" + "00".repeat(32)) as HexString,
			assets: [{ token: TOKEN, amount: 0n }],
			call: "0x",
		},
	} as unknown as Order
}

function options(validUntil: bigint) {
	return { relayerFee: 0n, nativeDispatchFee: 0n, validUntil, outputs: [{ token: TOKEN, amount: 500n }] }
}

function client(readContract: any, chainId = 8453) {
	return { chain: { id: chainId }, readContract } as any
}

describe("encodeFillOrder", () => {
	// Hashes captured from the pre-regeneration historical ABI with the fixture below.
	it.each([
		[1, "0xd60307c0d08186ed7832eedc48843249591ad50990a2a8b44321425e13d99eb3"],
		[2, "0x8f3eea94c86fbf74104043fc6176b68c80c4559b22f0a0e4309bb5b16b33eb37"],
	] as const)("preserves frozen v%s calldata bytes", (version, goldenHash) => {
		const data = encodeFillOrder(order(), options(999n), version)
		expect(keccak256(data)).toBe(goldenHash)
		expect(decodeFillOrder(data)).toMatchObject({
			version,
			options: { inputs: [], outputs: options(999n).outputs },
		})
	})

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
			expect(decoded).toMatchObject({ version, options: { inputs: [] } })
		}
	})

	it("distinguishes v3 and round-trips its positional input takes", () => {
		const inputs: TokenInfo[] = [{ token: TOKEN, amount: 400n }]
		const encoded = encodeFillOrder(order(), { ...options(77n), inputs }, 3)
		const decoded = decodeFillOrder(encoded)

		expect(decoded).toMatchObject({ version: 3 })
		expect(decoded!.options.validUntil).toBe(77n)
		expect(decoded!.options.inputs).toEqual(inputs)
		expect(slice(encoded, 0, 4)).toBe(RATE_SELECTOR)
	})

	it("returns null for calldata that is not a fillOrder call", () => {
		expect(decodeFillOrder("0xdeadbeef" as HexString)).toBeNull()
	})
})

describe("getFillOptionsVersion", () => {
	const LEGACY_IMPL = "0x976B268b06f545c4A2BF44866Aa2465bd8B3C67d" as HexString
	const NEW_IMPL = "0x2222222222222222222222222222222222222222" as HexString

	function slotFor(impl: HexString) {
		return `0x000000000000000000000000${impl.slice(2)}` as HexString
	}

	function client(impl: HexString | undefined) {
		return {
			chain: { id: 8453 },
			readContract: vi.fn().mockResolvedValue(1n),
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

	it("rechecks a version-2 proxy and sees a release-3 upgrade", async () => {
		const c = client(NEW_IMPL)
		c.readContract.mockResolvedValue(2n)

		expect(await getFillOptionsVersion(c, GATEWAY)).toBe(2)
		c.readContract.mockResolvedValue(3n)
		expect(await getFillOptionsVersion(c, GATEWAY)).toBe(3)
		expect(c.readContract).toHaveBeenCalledTimes(2)
	})
})

describe("contract release compatibility", () => {
	it.each([
		[2n, 2],
		[3n, 3],
	] as const)("maps gateway release %s before legacy chain overrides", async (release, encoding) => {
		const readContract = vi.fn(async ({ functionName }) => {
			if (functionName !== "version") throw new Error("Unexpected contract method")
			return release
		})
		await expect(getFillOptionsVersion(client(readContract, 84532), GATEWAY)).resolves.toBe(encoding)
	})
	it.each([0n, 4n, 5n, (1n << 64n) - 1n])("rejects unsupported gateway release %s", async (release) => {
		await expect(getFillOptionsVersion(client(vi.fn().mockResolvedValue(release), 84532), GATEWAY)).rejects.toThrow(
			/version/i,
		)
	})
})

describe("supportsRateFills", () => {
	it("requires fresh support from both the gateway and solver-account implementation", async () => {
		const readContract = vi.fn().mockResolvedValueOnce(3n).mockResolvedValueOnce(2n)
		await expect(
			supportsRateFills({ readContract } as any, GATEWAY, "0x2222222222222222222222222222222222222222"),
		).resolves.toBe(false)
		expect(readContract).toHaveBeenCalledTimes(2)
	})

	it.each([0n, 1n, 2n, 4n, 5n, (1n << 64n) - 1n])("rejects unsupported account release %s", async (release) => {
		const readContract = vi.fn().mockResolvedValueOnce(3n).mockResolvedValueOnce(release)
		await expect(supportsRateFills(client(readContract), GATEWAY, GATEWAY)).resolves.toBe(false)
	})

	it("does not cache capability across calls", async () => {
		const readContract = vi.fn().mockResolvedValue(3n)
		const c = { readContract } as any
		const solver = "0x2222222222222222222222222222222222222222"

		expect(await supportsRateFills(c, GATEWAY, solver)).toBe(true)
		readContract.mockResolvedValue(2n)
		expect(await supportsRateFills(c, GATEWAY, solver)).toBe(false)
		expect(readContract).toHaveBeenCalledTimes(4)
	})
})

describe("v3 compatibility boundaries", () => {
	it.each([1, 2] as const)("rejects takes when encoding historical v%s", (version) => {
		expect(() =>
			encodeFillOrder(order(), { ...options(1n), inputs: [{ token: TOKEN, amount: 1n }] }, version),
		).toThrow(/inputs|takes/i)
	})
	it("requires explicit quotes for an ordinary v3 fill", () => {
		expect(() => encodeFillOrder(order(), options(7n) as any, 3)).toThrow(/inputs|quote/i)
		expect(() => encodeFillOrder(order(), { ...options(7n), inputs: [] }, 3)).toThrow(/inputs|quote/i)
		const data = encodeFillOrder(order(), { ...options(7n), inputs: order().inputs }, 3)
		expect(data.slice(0, 10)).toBe(RATE_SELECTOR)
		expect(FILL_ORDER_V3_SELECTOR).toBe(RATE_SELECTOR)
		expect(decodeFillOrder(data)).toMatchObject({ version: 3, options: { inputs: order().inputs } })
		expect(FILL_ORDER_V2_ABI[0].inputs[1].components).toHaveLength(4)
	})
	it("checks capability before legacy chain overrides", async () => {
		expect(await getFillOptionsVersion(client(vi.fn().mockResolvedValue(3n), 84532), GATEWAY)).toBe(3)
	})
	it("does not share detection between identical addresses on separate chains", async () => {
		expect(await getFillOptionsVersion(client(vi.fn().mockResolvedValue(3n), 8453), GATEWAY)).toBe(3)
		expect(await getFillOptionsVersion(client(vi.fn().mockResolvedValue(1n), 84532), GATEWAY)).toBe(1)
	})
	it("propagates capability RPC failures", async () => {
		const c = client(vi.fn().mockRejectedValue(new Error("RPC timeout")))
		await expect(getFillOptionsVersion(c, GATEWAY)).rejects.toThrow("RPC timeout")
		await expect(supportsRateFills(c, GATEWAY, GATEWAY)).rejects.toThrow("RPC timeout")
	})
	it("does not accept the old boolean marker", async () => {
		await expect(supportsRateFills(client(vi.fn().mockResolvedValue(true)), GATEWAY, GATEWAY)).resolves.toBe(false)
	})
	it("rejects malformed versions instead of downgrading", async () => {
		await expect(getFillOptionsVersion(client(vi.fn().mockResolvedValue("0x12345678")), GATEWAY)).rejects.toThrow(
			/version/i,
		)
	})
})

describe("missing version getter classification with real viem errors", () => {
	function rpcClient(rpcError?: { code: number; message: string; data?: string }) {
		const request = vi.fn(async ({ method }: { method: string }) => {
			if (method !== "eth_call") throw new Error(`Unexpected RPC method: ${method}`)
			if (rpcError) throw Object.assign(new Error(rpcError.message), rpcError)
			return "0x"
		})
		return createPublicClient({
			chain: baseSepolia,
			transport: custom({ request }, { retryCount: 0 }),
		}) as unknown as PublicClient
	}

	it.each([
		{ code: -32603, message: "upstream request timeout" },
		{ code: -32603, message: "upstream request timeout", data: "0x" },
		{ code: -32601, message: "method not found" },
		{ code: -32005, message: "rate limit exceeded" },
	])("propagates provider failure $code: $message", async (rpcError) => {
		const c = rpcClient(rpcError)
		await expect(getFillOptionsVersion(c, GATEWAY)).rejects.toThrow(rpcError.message)
		await expect(supportsRateFills(c, GATEWAY, GATEWAY)).rejects.toThrow(rpcError.message)
	})

	it.each([
		{ code: 3, message: "execution reverted", data: "0x" },
		{ code: -32000, message: "execution reverted", data: "0x" },
		{ code: -32603, message: "execution reverted", data: "0x" },
		{ code: -32000, message: "function selector was not recognized and there's no fallback function" },
	])("retains fallback for genuine EVM failure $code: $message", async (rpcError) => {
		const c = rpcClient(rpcError)
		await expect(getFillOptionsVersion(c, GATEWAY)).resolves.toBe(1)
		await expect(supportsRateFills(c, GATEWAY, GATEWAY)).resolves.toBe(false)
	})

	it("retains fallback for a successful call returning no data", async () => {
		const c = rpcClient()
		await expect(getFillOptionsVersion(c, GATEWAY)).resolves.toBe(1)
		await expect(supportsRateFills(c, GATEWAY, GATEWAY)).resolves.toBe(false)
	})
})
