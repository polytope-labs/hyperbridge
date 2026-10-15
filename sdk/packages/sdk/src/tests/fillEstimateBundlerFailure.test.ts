import { hashTypedData, recoverAddress, slice } from "viem"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { CryptoUtils, LEGACY_SELECT_SOLVER_TYPEHASH, SELECT_SOLVER_TYPEHASH } from "@/protocols/intents/CryptoUtils"
import type { IntentGatewayContext } from "@/protocols/intents/types"
import { orderCommitment } from "@/protocols/intents/utils"
import type { HexString, Order } from "@/types"

// The gateway's release is read from a live node; here it is taken as supported.
vi.mock("@/protocols/intents/fillOrderCodec", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/protocols/intents/fillOrderCodec")>()),
	assertGatewayRelease: vi.fn(async () => {}),
}))

// Retries wait through `sleep`; here it returns at once.
vi.mock("@/utils", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/utils")>()),
	sleep: vi.fn(async () => {}),
}))

const { GasEstimator } = await import("@/protocols/intents/GasEstimator")
const { BundlerRpcError } = await import("@/protocols/intents/bundlerRetry")

// The rundler priority fee is fetched outside the mocked bundler calls; this bundler serves none.
beforeEach(() => {
	vi.stubGlobal(
		"fetch",
		vi.fn(async () => ({
			json: async () => ({ jsonrpc: "2.0", id: 1, error: { code: -32601, message: "Method not found" } }),
		})),
	)
})
afterEach(() => vi.unstubAllGlobals())

// A testnet chain, so gas is not priced through a swap quote.
const CHAIN = "EVM-97"
const GATEWAY = "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716" as HexString
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const USER = "0x9C7054b429f6b1dd35FD03e4fDC4f875Bc19931f" as HexString
const TOKEN = `0x000000000000000000000000${"46c85152bfe9f96829aa94755d9f915f9b10ef5f"}` as HexString

const ORDER: Order = {
	user: USER,
	source: CHAIN,
	destination: CHAIN,
	deadline: 65337297000n,
	nonce: 0n,
	fees: 0n,
	session: USER,
	predispatch: { assets: [], call: "0x" },
	inputs: [{ token: TOKEN, amount: 1_000n }],
	output: { beneficiary: USER, assets: [{ token: TOKEN, amount: 990n }], call: "0x" },
}

/**
 * An estimator whose bundler calls go to `bundler.single`, or, with `null`, through
 * `CryptoUtils.sendBundler` to the stubbed `fetch`.
 */
function estimatorWith(
	bundler: { single: ReturnType<typeof vi.fn> } | null,
	{
		gateway = { address: GATEWAY, typehash: SELECT_SOLVER_TYPEHASH },
		entryPoint = ENTRY_POINT,
	}: { gateway?: { address: HexString; typehash: HexString }; entryPoint?: HexString | null } = {},
) {
	const chain = {
		config: { stateMachineId: CHAIN },
		client: {
			getGasPrice: vi.fn().mockResolvedValue(1_000_000n),
			getBlock: vi.fn().mockResolvedValue({ baseFeePerGas: 900_000n }),
			readContract: vi.fn().mockResolvedValue(gateway.typehash),
		},
		configService: {
			getIntentGatewayAddress: () => gateway.address,
			getEntryPointAddress: () => entryPoint ?? undefined,
		},
		getFeeTokenWithDecimals: vi.fn().mockResolvedValue({ address: `0x${TOKEN.slice(26)}`, decimals: 6 }),
	}
	const ctx = {
		source: chain,
		dest: chain,
		bundlerUrl: "https://bundler.example",
		feeTokenCache: new Map(),
	} as unknown as IntentGatewayContext
	const crypto = bundler
		? ({
				encodeERC7821Execute: () => "0x" as HexString,
				sendBundler: bundler.single,
			} as unknown as CryptoUtils)
		: new CryptoUtils(ctx)

	const estimator = new GasEstimator(ctx, crypto)
	// Storage-slot discovery traces calls on a live node.
	vi.spyOn(estimator, "buildStateOverride").mockResolvedValue({ viem: [], bundler: {} })
	return estimator
}

const rejecting = () => ({
	single: vi.fn().mockRejectedValue(new Error("AA23 reverted")),
})

describe("GasEstimator.estimateFillOrder when the bundler fails to estimate", () => {
	beforeEach(() => vi.clearAllMocks())

	it("throws the bundler's error when a bundler estimate is required", async () => {
		const estimator = estimatorWith(rejecting())

		await expect(estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })).rejects.toThrow(
			"Bundler gas estimation failed: AA23 reverted",
		)
	})

	it("returns fixed gas limits when it is not", async () => {
		const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
		const estimator = estimatorWith(rejecting())

		const estimate = await estimator.estimateFillOrder({ order: ORDER })

		expect(estimate.callGasLimit).toBe(500_000n)
		expect(estimate.verificationGasLimit).toBe(100_000n)
		expect(estimate.preVerificationGas).toBe(100_000n)
		warn.mockRestore()
	})

	it("returns the bundler's figures when the estimate succeeds", async () => {
		const estimator = estimatorWith({
			single: vi
				.fn()
				.mockResolvedValue({
					callGasLimit: "0x186a0",
					verificationGasLimit: "0x186a0",
					preVerificationGas: "0x186a0",
				}),
		})

		const estimate = await estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		expect(estimate.callGasLimit).toBe(160_000n)
		expect(estimate.verificationGasLimit).toBe(105_000n)
		expect(estimate.preVerificationGas).toBe(105_000n)
	})

	it("throws when the chain has no EntryPoint configured and a bundler estimate is required", async () => {
		const bundler = rejecting()
		const estimator = estimatorWith(bundler, { entryPoint: null })

		await expect(estimator.estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })).rejects.toThrow(
			"Bundler gas estimation failed: No EntryPoint configured for EVM-97",
		)
		expect(bundler.single).not.toHaveBeenCalled()
	})
})

describe("GasEstimator.estimateFillOrder when the bundler fails transiently", () => {
	// What rundler answers when its upstream RPC drops a request.
	const RPC_PROVIDER_ERROR = { error: { code: -32603, message: "internal error: rpc provider error" } }
	const ESTIMATE = {
		result: { callGasLimit: "0x186a0", verificationGasLimit: "0x186a0", preVerificationGas: "0x186a0" },
	}

	/**
	 * Stubs a bundler that answers the n-th `eth_estimateUserOperationGas` with the n-th
	 * reply (the last one thereafter) and serves no rundler fee method. Returns the replies sent.
	 */
	function bundlerAnswering(...replies: Record<string, unknown>[]) {
		const sent: Record<string, unknown>[] = []
		vi.stubGlobal(
			"fetch",
			vi.fn(async (_url: string, init: { body: string }) => {
				const { method } = JSON.parse(init.body) as { method: string }
				let reply: Record<string, unknown> = { error: { code: -32601, message: "Method not found" } }
				if (method === "eth_estimateUserOperationGas") {
					reply = replies[Math.min(sent.length, replies.length - 1)]
					sent.push(reply)
				}
				return { status: 200, json: async () => ({ jsonrpc: "2.0", id: 1, ...reply }) }
			}),
		)
		return sent
	}

	it("estimates from the attempt after rundler's rpc provider error", async () => {
		const sent = bundlerAnswering(RPC_PROVIDER_ERROR, ESTIMATE)
		const onBundlerRetry = vi.fn()

		const estimate = await estimatorWith(null).estimateFillOrder({
			order: ORDER,
			requireBundlerEstimate: true,
			onBundlerRetry,
		})

		expect(estimate.callGasLimit).toBe(160_000n)
		expect(sent).toEqual([RPC_PROVIDER_ERROR, ESTIMATE])
		expect(onBundlerRetry).toHaveBeenCalledOnce()
		expect(onBundlerRetry.mock.calls[0][0]).toMatchObject({ attempt: 1, maxAttempts: 3, delayMs: 250 })
		expect(onBundlerRetry.mock.calls[0][0].error.message).toBe("Bundler error: internal error: rpc provider error")
	})

	it("throws the bundler's error after three attempts that all fail transiently", async () => {
		const sent = bundlerAnswering(RPC_PROVIDER_ERROR)
		const onBundlerRetry = vi.fn()

		const failure = estimatorWith(null).estimateFillOrder({
			order: ORDER,
			requireBundlerEstimate: true,
			onBundlerRetry,
		})

		await expect(failure).rejects.toThrow(
			"Bundler gas estimation failed: Bundler error: internal error: rpc provider error",
		)
		await expect(failure.catch((e: Error) => e.cause)).resolves.toBeInstanceOf(BundlerRpcError)
		expect(sent).toHaveLength(3)
		expect(onBundlerRetry).toHaveBeenCalledTimes(2)
	})

	it("does not ask again after a validation revert", async () => {
		const sent = bundlerAnswering({ error: { code: -32500, message: "validation reverted: AA23 reverted" } })
		const onBundlerRetry = vi.fn()

		await expect(
			estimatorWith(null).estimateFillOrder({ order: ORDER, requireBundlerEstimate: true, onBundlerRetry }),
		).rejects.toThrow("Bundler gas estimation failed: Bundler error: validation reverted: AA23 reverted")
		expect(sent).toHaveLength(1)
		expect(onBundlerRetry).not.toHaveBeenCalled()
	})
})

describe("GasEstimator.estimateFillOrder session signature", () => {
	const succeeding = () => ({
		single: vi
			.fn()
			.mockResolvedValue({ callGasLimit: "0x1", verificationGasLimit: "0x1", preVerificationGas: "0x1" }),
	})

	/** The op the estimate sent, packed again, and the session's selection signature at its end. */
	function sentOp(single: ReturnType<typeof vi.fn>) {
		const [, [op]] = single.mock.calls[0] as [string, [Record<string, HexString>]]
		return {
			sender: op.sender,
			selection: slice(op.signature, 97),
			userOpHash: CryptoUtils.computeUserOpHash(
				{
					sender: op.sender,
					nonce: BigInt(op.nonce),
					initCode: "0x",
					callData: op.callData,
					accountGasLimits: CryptoUtils.packGasLimits(
						BigInt(op.verificationGasLimit),
						BigInt(op.callGasLimit),
					),
					preVerificationGas: BigInt(op.preVerificationGas),
					gasFees: CryptoUtils.packGasFees(BigInt(op.maxPriorityFeePerGas), BigInt(op.maxFeePerGas)),
					paymasterAndData: "0x",
					signature: "0x",
				},
				ENTRY_POINT,
				97n,
			),
		}
	}

	const domain = (verifyingContract: HexString) =>
		({ name: "IntentGateway", version: "2", chainId: 97, verifyingContract }) as const

	it("selects by userOpHash on a gateway that selects by userOpHash", async () => {
		const gateway = {
			address: "0x0000000000000000000000000000000000000c01" as HexString,
			typehash: SELECT_SOLVER_TYPEHASH,
		}
		const bundler = succeeding()

		await estimatorWith(bundler, { gateway }).estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		const { sender, selection, userOpHash } = sentOp(bundler.single)
		const digest = hashTypedData({
			domain: domain(gateway.address),
			types: {
				SelectSolver: [
					{ name: "commitment", type: "bytes32" },
					{ name: "userOpHash", type: "bytes32" },
				],
			},
			primaryType: "SelectSolver",
			message: { commitment: orderCommitment(ORDER) as HexString, userOpHash },
		})
		expect(await recoverAddress({ hash: digest, signature: selection })).toBe(sender)
	})

	it("selects by the solver's address on a gateway that selects by address", async () => {
		const gateway = {
			address: "0x0000000000000000000000000000000000000c02" as HexString,
			typehash: LEGACY_SELECT_SOLVER_TYPEHASH,
		}
		const bundler = succeeding()

		await estimatorWith(bundler, { gateway }).estimateFillOrder({ order: ORDER, requireBundlerEstimate: true })

		const { sender, selection } = sentOp(bundler.single)
		const digest = hashTypedData({
			domain: domain(gateway.address),
			types: {
				SelectSolver: [
					{ name: "commitment", type: "bytes32" },
					{ name: "solver", type: "address" },
				],
			},
			primaryType: "SelectSolver",
			message: { commitment: orderCommitment(ORDER) as HexString, solver: sender },
		})
		expect(await recoverAddress({ hash: digest, signature: selection })).toBe(sender)
	})
})
