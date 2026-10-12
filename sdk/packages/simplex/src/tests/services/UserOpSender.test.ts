import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { toHex } from "viem"
import { rundlerUserOperationFees, type HexString } from "@hyperbridge/sdk"

import { UserOpSender, type Eip7702Authorization } from "@/services/UserOpSender"
import { buildPaymasterAndData } from "@/services/paymaster"
import { packPaymasterAndData, VERIFICATION_GAS_LIMIT_PERMIT2, POST_OP_GAS_LIMIT_SIMPLEX } from "@/services/paymaster/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Signer } from "@/services/wallet"

/**
 * Regression tests for the bootstrap-approve nonce race: building Simplex
 * paymaster data can send an on-chain `approve(Permit2, max)` from the authority
 * EOA. An EIP-7702 authorization signed *before* that tx embeds the pre-approve nonce,
 * so the bundler rejects the op ("EIP-7702 nonce mismatch"). The sender must
 * therefore resolve the authorization factory only after paymaster data is
 * built — and treat any preparation failure as "never submitted" (null).
 */

vi.mock("@/services/paymaster", () => ({
	hasPaymaster: () => true,
	buildPaymasterAndData: vi.fn(),
}))

vi.mock("@hyperbridge/sdk", async (importOriginal) => {
	const actual = await importOriginal<typeof import("@hyperbridge/sdk")>()
	return { ...actual, rundlerUserOperationFees: vi.fn(actual.rundlerUserOperationFees) }
})

const CHAIN = "EVM-56"
const CHAIN_ID = 56
const ENTRY_POINT = "0x4337843378433784337843378433784337843378" as HexString
const PAYMASTER = "0x00000000000000000000000000000000000000aa" as HexString
const TOKEN = "0x00000000000000000000000000000000000000bb" as HexString
const DELEGATE = "0x00000000000000000000000000000000000000cc" as HexString
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString

// Fixed limits as passed by the delegation flow — skips bundler gas estimation.
const GAS = { verificationGasLimit: 150_000n, callGasLimit: 50_000n, preVerificationGas: 100_000n }

const configServiceFor = (bundlerUrl: string) =>
	({
		getEntryPointAddress: () => ENTRY_POINT,
		getBundlerUrl: () => bundlerUrl,
		getChainId: () => CHAIN_ID,
	}) as unknown as FillerConfigService

const configService = configServiceFor("http://127.0.0.1:1/bundler")

const publicClient = {
	getGasPrice: async () => 1_000_000_000n,
	getBlock: async () => ({ baseFeePerGas: 900_000_000n }),
	// Only EntryPoint.getNonce is read on this path (fixed gas limits skip estimation).
	readContract: async () => 0n,
}

const clientManager = {
	getPublicClient: () => publicClient,
	getWalletClient: () => ({}),
} as unknown as ChainClientManager

const signer = {
	address: SOLVER,
	signTypedData: async () => ("0x" + "11".repeat(65)) as HexString,
} as unknown as Signer

const permit2ModePaymasterAndData = packPaymasterAndData({
	paymaster: PAYMASTER,
	paymasterData: "0x02" as HexString,
	paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_PERMIT2,
	paymasterPostOpGasLimit: POST_OP_GAS_LIMIT_SIMPLEX,
})

let bundlerCalls: Array<{ method: string; params: unknown[] }>
let bundlerReplies: Record<string, { result?: unknown; error?: unknown }>

beforeEach(() => {
	bundlerCalls = []
	bundlerReplies = {}
	vi.mocked(buildPaymasterAndData).mockReset()
	vi.mocked(rundlerUserOperationFees).mockClear()
	vi.stubGlobal("fetch", async (_url: unknown, init?: { body?: string }) => {
		const { method, params } = JSON.parse(init?.body ?? "{}") as { method: string; params: unknown[] }
		bundlerCalls.push({ method, params })
		const reply = bundlerReplies[method]
		if (reply) return { json: async () => ({ jsonrpc: "2.0", id: 1, ...reply }) }
		let result: unknown
		switch (method) {
			case "eth_sendUserOperation":
				result = ("0x" + "ab".repeat(32)) as HexString
				break
			case "eth_getUserOperationReceipt":
				result = { success: true, receipt: { transactionHash: ("0x" + "cd".repeat(32)) as HexString } }
				break
			default:
				return { json: async () => ({ jsonrpc: "2.0", id: 1, error: { message: `no ${method}` } }) }
		}
		return { json: async () => ({ jsonrpc: "2.0", id: 1, result }) }
	})
})

afterEach(() => {
	vi.unstubAllGlobals()
})

describe("UserOpSender EIP-7702 authorization ordering", () => {
	it("signs the authorization after paymaster data is built, so the approve tx nonce is reflected", async () => {
		// Simulates the BSC incident: the EOA starts at nonce 0, and building
		// the Permit2 bootstrap approve mines a tx that bumps it to 1.
		let eoaNonce = 0
		vi.mocked(buildPaymasterAndData).mockImplementation(async () => {
			eoaNonce += 1
			return { paymasterAndData: permit2ModePaymasterAndData, type: "simplex", address: PAYMASTER, token: TOKEN }
		})

		// Mirrors DelegationService.buildAuthorization: reads the nonce at signing time.
		const signAuthorization = vi.fn(
			async (): Promise<Eip7702Authorization> => ({
				chainId: CHAIN_ID,
				address: DELEGATE,
				nonce: eoaNonce,
				r: ("0x" + "22".repeat(32)) as HexString,
				s: ("0x" + "33".repeat(32)) as HexString,
				yParity: 0,
			}),
		)

		const sender = new UserOpSender(clientManager, configService, signer)
		const result = await sender.trySendSponsored({
			chain: CHAIN,
			callData: "0x" as HexString,
			eip7702Auth: signAuthorization,
			gas: GAS,
		})

		expect(result).toEqual({ txHash: "0x" + "cd".repeat(32) })
		expect(signAuthorization).toHaveBeenCalledOnce()

		// The authorization submitted to the bundler must carry the post-approve
		// nonce (1) — a pre-signed tuple would carry the stale 0 and be rejected.
		const send = bundlerCalls.find((c) => c.method === "eth_sendUserOperation")
		expect(send).toBeDefined()
		const [op] = send!.params as [{ sender?: string; eip7702Auth?: { nonce: string } }]
		// The op must be attributed to the signer's address — this is what the
		// signer-interface migration changed (`signer.address`, was
		// `signer.account.address`), and a stale stub let it go out undefined.
		expect(op.sender).toBe(SOLVER)
		expect(op.eip7702Auth?.nonce).toBe(toHex(1))
	})

	it("returns null without signing an authorization when paymaster preparation fails", async () => {
		vi.mocked(buildPaymasterAndData).mockRejectedValue(
			new Error("SimplexPaymaster needs a one-time funded approval on this chain"),
		)
		const signAuthorization = vi.fn(async (): Promise<Eip7702Authorization> => {
			throw new Error("should not be called")
		})

		const sender = new UserOpSender(clientManager, configService, signer)
		await expect(
			sender.trySendSponsored({
				chain: CHAIN,
				callData: "0x" as HexString,
				eip7702Auth: signAuthorization,
				gas: GAS,
			}),
		).resolves.toBeNull()

		expect(signAuthorization).not.toHaveBeenCalled()
		expect(bundlerCalls.some((c) => c.method === "eth_sendUserOperation")).toBe(false)
	})

	it("returns null without signing an authorization when no paymaster is available", async () => {
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: "0x" as HexString,
			type: "none",
			reason: "insufficient stablecoin balance for all configured paymasters",
		})
		const signAuthorization = vi.fn(async (): Promise<Eip7702Authorization> => {
			throw new Error("should not be called")
		})

		const sender = new UserOpSender(clientManager, configService, signer)
		await expect(
			sender.trySendSponsored({
				chain: CHAIN,
				callData: "0x" as HexString,
				eip7702Auth: signAuthorization,
				gas: GAS,
			}),
		).resolves.toBeNull()

		expect(signAuthorization).not.toHaveBeenCalled()
		expect(bundlerCalls.some((c) => c.method === "eth_sendUserOperation")).toBe(false)
	})
})

describe("UserOpSender gas price", () => {
	const sponsorWith = async (bundlerUrl: string) => {
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData: permit2ModePaymasterAndData,
			type: "simplex",
			address: PAYMASTER,
			token: TOKEN,
		})
		const sender = new UserOpSender(clientManager, configServiceFor(bundlerUrl), signer)
		return sender.trySendSponsored({ chain: CHAIN, callData: "0x" as HexString, gas: GAS })
	}

	const sentFees = () =>
		bundlerCalls
			.filter((c) => c.method === "eth_sendUserOperation")
			.map((c) => {
				const [op] = c.params as [{ maxFeePerGas: HexString; maxPriorityFeePerGas: HexString }]
				return { maxFeePerGas: BigInt(op.maxFeePerGas), maxPriorityFeePerGas: BigInt(op.maxPriorityFeePerGas) }
			})

	const rundlerCalls = () => bundlerCalls.filter((c) => c.method.startsWith("rundler_")).map((c) => c.method)
	const METHOD_NOT_FOUND = { error: { code: -32601, message: "Method not found" } }

	it("prices a rundler bundler from the fees it suggests, raised by the bumps", async () => {
		bundlerReplies.rundler_getUserOperationGasPrice = {
			result: { suggested: { maxPriorityFeePerGas: toHex(2_000_000_000n), maxFeePerGas: toHex(5_000_000_000n) } },
		}

		await expect(sponsorWith("http://rundler.test/bundler")).resolves.not.toBeNull()

		const fees = { maxPriorityFeePerGas: 2_160_000_000n, maxFeePerGas: 5_500_000_000n }
		expect(sentFees()).toEqual([fees])
		expect(vi.mocked(buildPaymasterAndData).mock.calls[0][0].prefund?.maxFeePerGas).toBe(fees.maxFeePerGas)
		expect(rundlerCalls()).toEqual(["rundler_getUserOperationGasPrice"])
		expect(rundlerUserOperationFees).toHaveBeenCalledWith("http://rundler.test/bundler", expect.anything(), expect.anything())
	})

	it("raises the fees to a rundler bundler's priority fee where it suggests none", async () => {
		bundlerReplies.rundler_getUserOperationGasPrice = METHOD_NOT_FOUND
		// 30 gwei, the Polygon Amoy floor, against a 1 gwei chain gas price.
		bundlerReplies.rundler_maxPriorityFeePerGas = { result: "0x6fc23ac00" }

		await expect(sponsorWith("http://older-rundler.test/bundler")).resolves.not.toBeNull()

		const fees = { maxPriorityFeePerGas: 32_400_000_000n, maxFeePerGas: 990_000_000n + 32_400_000_000n }
		expect(sentFees()).toEqual([fees])
	})

	it("keeps the chain estimate and stops asking when the bundler does not serve rundler fees", async () => {
		bundlerReplies.rundler_getUserOperationGasPrice = METHOD_NOT_FOUND
		bundlerReplies.rundler_maxPriorityFeePerGas = METHOD_NOT_FOUND

		await sponsorWith("http://other-bundler.test/bundler")
		await sponsorWith("http://other-bundler.test/bundler")

		const chainFees = { maxPriorityFeePerGas: 1_080_000_000n, maxFeePerGas: 1_100_000_000n }
		expect(sentFees()).toEqual([chainFees, chainFees])
		expect(rundlerCalls()).toEqual(["rundler_getUserOperationGasPrice", "rundler_maxPriorityFeePerGas"])
	})
})
