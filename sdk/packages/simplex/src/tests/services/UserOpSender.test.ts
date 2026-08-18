import { describe, it, expect, vi, beforeEach, afterEach } from "vitest"
import { toHex } from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { UserOpSender, type Eip7702Authorization } from "@/services/UserOpSender"
import { buildPaymasterAndData } from "@/services/paymaster"
import { packPaymasterAndData, VERIFICATION_GAS_LIMIT_APPROVE, POST_OP_GAS_LIMIT } from "@/services/paymaster/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { Signer } from "@/services/wallet"

/**
 * Regression tests for the approve-mode nonce race (#1070): building Simplex
 * paymaster data can send an on-chain `approve` from the authority EOA. An
 * EIP-7702 authorization signed *before* that tx embeds the pre-approve nonce,
 * so the bundler rejects the op ("EIP-7702 nonce mismatch"). The sender must
 * therefore resolve the authorization factory only after paymaster data is
 * built — and treat any preparation failure as "never submitted" (null).
 */

vi.mock("@/services/paymaster", () => ({
	hasPaymaster: () => true,
	buildPaymasterAndData: vi.fn(),
}))

const CHAIN = "EVM-56"
const CHAIN_ID = 56
const ENTRY_POINT = "0x4337843378433784337843378433784337843378" as HexString
const PAYMASTER = "0x00000000000000000000000000000000000000aa" as HexString
const TOKEN = "0x00000000000000000000000000000000000000bb" as HexString
const DELEGATE = "0x00000000000000000000000000000000000000cc" as HexString
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString

// Fixed limits as passed by the delegation flow — skips bundler gas estimation.
const GAS = { verificationGasLimit: 150_000n, callGasLimit: 50_000n, preVerificationGas: 100_000n }

const configService = {
	getEntryPointAddress: () => ENTRY_POINT,
	getBundlerUrl: () => "http://127.0.0.1:1/bundler",
	getChainId: () => CHAIN_ID,
} as unknown as FillerConfigService

const publicClient = {
	getGasPrice: async () => 1_000_000_000n,
	// Only EntryPoint.getNonce is read on this path (fixed gas limits skip estimation).
	readContract: async () => 0n,
}

const clientManager = {
	getPublicClient: () => publicClient,
	getWalletClient: () => ({}),
} as unknown as ChainClientManager

const signer = {
	account: { address: SOLVER },
	signTypedData: async () => ("0x" + "11".repeat(65)) as HexString,
} as unknown as Signer

const approveModePaymasterAndData = packPaymasterAndData({
	paymaster: PAYMASTER,
	paymasterData: "0x01" as HexString,
	paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_APPROVE,
	paymasterPostOpGasLimit: POST_OP_GAS_LIMIT,
})

let bundlerCalls: Array<{ method: string; params: unknown[] }>

beforeEach(() => {
	bundlerCalls = []
	vi.mocked(buildPaymasterAndData).mockReset()
	vi.stubGlobal("fetch", async (_url: unknown, init?: { body?: string }) => {
		const { method, params } = JSON.parse(init?.body ?? "{}") as { method: string; params: unknown[] }
		bundlerCalls.push({ method, params })
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
		// approve-mode paymaster data mines an approve tx that bumps it to 1.
		let eoaNonce = 0
		vi.mocked(buildPaymasterAndData).mockImplementation(async () => {
			eoaNonce += 1
			return { paymasterAndData: approveModePaymasterAndData, type: "simplex", address: PAYMASTER, token: TOKEN }
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
			forceApproveMode: true,
		})

		expect(result).toEqual({ txHash: "0x" + "cd".repeat(32) })
		expect(signAuthorization).toHaveBeenCalledOnce()

		// The authorization submitted to the bundler must carry the post-approve
		// nonce (1) — a pre-signed tuple would carry the stale 0 and be rejected.
		const send = bundlerCalls.find((c) => c.method === "eth_sendUserOperation")
		expect(send).toBeDefined()
		const [op] = send!.params as [{ eip7702Auth?: { nonce: string } }]
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
				forceApproveMode: true,
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
