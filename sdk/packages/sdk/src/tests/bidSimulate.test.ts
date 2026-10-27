import { ENTRY_POINT_V09 } from "@/configs/chain"
import { BidImpl } from "@/protocols/intents/Bid"
import { BundlerRpcError } from "@/protocols/intents/bundlerRetry"
import { CryptoUtils, SELECT_SOLVER_TYPEHASH } from "@/protocols/intents/CryptoUtils"
import { encodeERC7821ExecuteBatch } from "@/protocols/intents/decode-utils"
import { encodeFillOrder } from "@/protocols/intents/fillOrderCodec"
import { orderCommitment, transformOrderForContract } from "@/protocols/intents/utils"
import type { FillOptions, HexString, Order, PackedUserOperation } from "@/types"
import { concat, pad } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { afterEach, describe, expect, it, vi } from "vitest"

// Retries wait through `sleep`; here it returns at once.
vi.mock("@/utils", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/utils")>()),
	sleep: vi.fn(async () => {}),
}))

const CHAIN_ID = 8453
const TOKEN = pad("0x55", { size: 32 }) as HexString
const NATIVE = pad("0x", { size: 32 }) as HexString
const DISPATCHER = `0x${"d1".repeat(20)}` as HexString
const PARAMS_SLOT = `0x${"0".repeat(63)}5`
// The gateway's params slot 5: `solverSelection` on in byte 20, above the dispatcher.
const PARAMS_WORD = `0x${"00".repeat(11)}01${DISPATCHER.slice(2)}` as HexString
const solver = privateKeyToAccount("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")

const ESTIMATE = {
	result: { callGasLimit: "0x186a0", verificationGasLimit: "0x186a0", preVerificationGas: "0x186a0" },
}

const placed: Order = {
	user: TOKEN,
	source: "EVM-1",
	destination: `EVM-${CHAIN_ID}`,
	deadline: 100n,
	nonce: 1n,
	fees: 0n,
	session: "0x70997970C51812dc3A010C7d01b50e0d17dc79C8",
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: TOKEN, amount: 100n }],
	output: { beneficiary: TOKEN, assets: [{ token: NATIVE, amount: 7n }], call: "0x" },
}
const COMMITMENT = orderCommitment(placed)
const order: Order = { ...placed, id: COMMITMENT }

const fillOptions: FillOptions = {
	outputs: [{ token: NATIVE, amount: 7n }],
	inputs: [{ token: TOKEN, amount: 100n }],
	relayerFee: 0n,
	nativeDispatchFee: 3n,
	validUntil: 0n,
}

/**
 * Stubs a bundler that answers the n-th request with the n-th reply, and the last one
 * thereafter. Returns the requests it receives.
 */
function bundlerAnswering(...replies: Record<string, unknown>[]) {
	const requests: { method: string; params: unknown[] }[] = []
	vi.stubGlobal(
		"fetch",
		vi.fn(async (_url: string, init: { body: string }) => {
			requests.push(JSON.parse(init.body))
			const reply = replies[Math.min(requests.length, replies.length) - 1]
			return { status: 200, json: async () => ({ jsonrpc: "2.0", id: 1, ...reply }) }
		}),
	)
	return requests
}

/** A bid filling `order` on the gateway at `gateway`, which selects by userOpHash, signed for EntryPoint v0.9. */
async function bidOn(gateway: HexString, bundlerUrl: string | null = "https://bundler.invalid") {
	const callData = encodeERC7821ExecuteBatch([
		{ target: gateway, value: 0n, data: encodeFillOrder(transformOrderForContract(placed) as Order, fillOptions) },
	])
	const unsigned: PackedUserOperation = {
		sender: solver.address,
		nonce: CryptoUtils.bidNonceKey(COMMITMENT, order.session, callData) << 64n,
		initCode: "0x",
		callData,
		accountGasLimits: `0x${"00".repeat(32)}`,
		preVerificationGas: 1n,
		gasFees: `0x${"00".repeat(32)}`,
		paymasterAndData: "0x",
		signature: "0x",
	}
	const solverSignature = await solver.signTypedData(
		CryptoUtils.packedUserOpTypedData(unsigned, ENTRY_POINT_V09, BigInt(CHAIN_ID)),
	)
	const userOp = { ...unsigned, signature: concat([COMMITMENT, solverSignature]) }

	const client = {
		chain: { id: CHAIN_ID },
		readContract: vi.fn(async () => SELECT_SOLVER_TYPEHASH),
		getStorageAt: vi.fn(async (_request: Record<string, unknown>) => PARAMS_WORD),
		call: vi.fn(async (_request: Record<string, unknown>) => ({ data: "0x" })),
	}
	// No session key anywhere: simulating a bid does not sign a selection.
	const ctx = {
		bundlerUrl: bundlerUrl ?? undefined,
		dest: {
			config: { stateMachineId: `EVM-${CHAIN_ID}` },
			configService: { getIntentGatewayAddress: () => gateway },
			client,
		},
	} as never
	const bid = new BidImpl({
		ctx,
		crypto: new CryptoUtils(ctx),
		order,
		fillerBid: { filler: "solver", bid: CryptoUtils.bidId(userOp.callData), userOp, deposit: 0n },
		fillOptions,
		priceOutputs: async () => null,
	})
	return { bid, client, userOp }
}

afterEach(() => {
	vi.unstubAllGlobals()
	vi.restoreAllMocks()
})

describe("BidImpl.simulate", () => {
	it("estimates the bid's UserOperation as signed on EntryPoint v0.9, with solver selection turned off", async () => {
		const gateway = "0x0000000000000000000000000000000000000b01" as HexString
		const requests = bundlerAnswering(ESTIMATE)
		const { bid, client, userOp } = await bidOn(gateway)

		await bid.simulate()

		expect(client.getStorageAt).toHaveBeenCalledWith({ address: gateway, slot: PARAMS_SLOT })
		expect(requests).toHaveLength(1)
		expect(requests[0].method).toBe("eth_estimateUserOperationGas")
		expect(requests[0].params).toEqual([
			// The solver's signature alone: no session selection appended.
			CryptoUtils.prepareBundlerCall(userOp),
			ENTRY_POINT_V09,
			// The gateway's own dispatcher kept, the `solverSelection` byte above it cleared.
			{ [gateway]: { stateDiff: { [PARAMS_SLOT]: `0x${"0".repeat(24)}${DISPATCHER.slice(2)}` } } },
		])
		// Nothing is run over the chain RPC, so no native value is forwarded.
		expect(client.call).not.toHaveBeenCalled()
	})

	it.each([
		[-32500, "validation reverted: AA25 invalid account nonce"],
		[-32521, "ERC20: transfer amount exceeds balance"],
	])("fails at once when the bundler rejects the operation with %i", async (code, message) => {
		const requests = bundlerAnswering({ error: { code, message } })
		const { bid } = await bidOn("0x0000000000000000000000000000000000000b02")

		const error = await bid.simulate().catch((e: unknown) => e)

		expect(error).toBeInstanceOf(Error)
		expect((error as Error).message).toBe(`Simulation failed: Bundler error: ${message}`)
		expect((error as Error).cause).toBeInstanceOf(BundlerRpcError)
		expect(((error as Error).cause as BundlerRpcError).code).toBe(code)
		expect(requests).toHaveLength(1)
	})

	it("estimates again after a transient bundler failure", async () => {
		vi.spyOn(console, "warn").mockImplementation(() => {})
		const requests = bundlerAnswering(
			{ error: { code: -32603, message: "internal error: rpc provider error" } },
			ESTIMATE,
		)
		const { bid } = await bidOn("0x0000000000000000000000000000000000000b03")

		await bid.simulate()

		expect(requests).toHaveLength(2)
	})

	it("fails without a bundler", async () => {
		const { bid } = await bidOn("0x0000000000000000000000000000000000000b04", null)

		await expect(bid.simulate()).rejects.toThrow("Simulation failed: Bundler URL not configured")
	})
})
