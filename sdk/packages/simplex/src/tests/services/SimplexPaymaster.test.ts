import { describe, it, expect, vi } from "vitest"
import {
	encodePacked,
	hashTypedData,
	keccak256,
	encodeAbiParameters,
	concat,
	maxUint256,
	size,
	slice,
	toHex,
	type PublicClient,
	type WalletClient,
} from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { buildSimplexPaymasterData } from "@/services/paymaster/provider/simplex"
import { permit2TransferTypedData, normalizeSignature65 } from "@/services/paymaster/permit2"
import {
	VERIFICATION_GAS_LIMIT_APPROVE,
	VERIFICATION_GAS_LIMIT_PERMIT,
	VERIFICATION_GAS_LIMIT_PERMIT2,
	PERMIT2_DEADLINE_SECONDS,
	RECOMMENDED_AMOUNT_USD,
	THRESHOLD_USD,
} from "@/services/paymaster/types"
import type { FillerConfigService } from "@/services/FillerConfigService"

/**
 * Simplex paymaster mode selection on chains whose fee token has no EIP-2612
 * permit (BSC pegged stables). Bootstrap is one funded approve(Permit2, max);
 * afterwards every op carries a per-op Permit2 signature (#1071). An unfunded
 * solver must fail fast with one actionable line before any estimateGas
 * round-trip (#1070), and a solver with an existing allowance must never need
 * native at all.
 */

const USDC = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d" as HexString // BSC pegged, 18 decimals
const PAYMASTER = "0x0578cFB241215b77442a541325d6A4E6dFE700Ec" as HexString
const PERMIT2 = "0x000000000022D473030F116dDEE9F6B43aC78BA3" as HexString
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString
const CHAIN = "EVM-56"
const DECIMALS = 18
const RECOMMENDED = RECOMMENDED_AMOUNT_USD * 10n ** BigInt(DECIMALS)
const THRESHOLD = THRESHOLD_USD * 10n ** BigInt(DECIMALS)

function makeConfigService(permit2: HexString | "0x" = PERMIT2): FillerConfigService {
	return {
		getChainId: () => 56,
		getUsdcAsset: () => USDC,
		getUsdcDecimals: () => DECIMALS,
		getUsdtAsset: () => "0x" as HexString,
		getUsdtDecimals: () => DECIMALS,
		getPermit2Address: () => permit2,
	} as unknown as FillerConfigService
}
const configService = makeConfigService()

const SIG = ("0x" + "11".repeat(64) + "1b") as HexString

function mockSigner() {
	const signTypedData = vi.fn(async () => SIG)
	return { signer: { signTypedData }, signTypedData }
}

function mockClient(opts: {
	paymasterAllowance?: bigint
	permit2Allowance?: bigint
	native?: bigint
	permit?: boolean
	/** Whether the paymaster deployment exposes PERMIT2() (defaults to true). */
	permit2Capable?: boolean
}): PublicClient {
	return {
		readContract: async ({ functionName, args }: { functionName: string; args?: unknown[] }) => {
			switch (functionName) {
				case "PERMIT2":
					if (opts.permit2Capable === false) throw new Error("execution reverted")
					return PERMIT2
				case "balanceOf":
					return 5n * 10n ** BigInt(DECIMALS)
				case "allowance": {
					const spender = (args?.[1] as string).toLowerCase()
					if (spender === PERMIT2.toLowerCase()) return opts.permit2Allowance ?? 0n
					return opts.paymasterAllowance ?? 0n
				}
				case "version":
					if (opts.permit) return "2"
					throw new Error("no version()")
				case "name":
					return "USD Coin"
				case "nonces":
					return 0n
				default:
					throw new Error(`unexpected readContract: ${functionName}`)
			}
		},
		getBalance: async () => opts.native ?? 0n,
		getGasPrice: async () => 1_000_000_000n,
		waitForTransactionReceipt: async () => ({ status: "success" }),
	} as unknown as PublicClient
}

function mockWalletClient() {
	const writeContract = vi.fn(async () => ("0x" + "ee".repeat(32)) as HexString)
	const walletClient = {
		chain: undefined,
		// A viem WalletClient really does carry `account` — this is not a Signer stub.
		account: { address: SOLVER },
		writeContract,
	} as unknown as WalletClient
	return { walletClient, writeContract }
}

function approveCall(writeContract: ReturnType<typeof vi.fn>) {
	const [call] = writeContract.mock.calls[0] as unknown as [{ functionName: string; args: unknown[] }]
	return call
}

const build = (client: PublicClient, walletClient: WalletClient, signer = mockSigner().signer, skipPermit = true) =>
	buildSimplexPaymasterData(client, walletClient, signer, SOLVER, PAYMASTER, CHAIN, configService, {
		skipPermit,
	})

describe("buildSimplexPaymasterData mode selection (no-permit token)", () => {
	it("uses PERMIT2 mode with no tx once the token is approved to Permit2", async () => {
		const { walletClient, writeContract } = mockWalletClient()
		const { signer, signTypedData } = mockSigner()
		const before = BigInt(Math.floor(Date.now() / 1000))

		const pm = await build(mockClient({ permit2Allowance: maxUint256, native: 0n }), walletClient, signer)

		expect(writeContract).not.toHaveBeenCalled()
		expect(pm?.paymasterVerificationGasLimit).toBe(VERIFICATION_GAS_LIMIT_PERMIT2)
		expect(pm?.token).toBe(USDC)

		const data = pm!.paymasterData
		expect(size(data)).toBe(182)
		expect(slice(data, 0, 21)).toBe(encodePacked(["uint8", "address"], [2, USDC]))
		expect(BigInt(slice(data, 21, 53))).toBe(RECOMMENDED)
		const nonce = BigInt(slice(data, 53, 85))
		const deadline = BigInt(slice(data, 85, 117))
		expect(slice(data, 117, 182)).toBe(SIG)
		expect(deadline).toBeGreaterThanOrEqual(before + PERMIT2_DEADLINE_SECONDS)
		expect(deadline).toBeLessThanOrEqual(before + PERMIT2_DEADLINE_SECONDS + 5n)

		// The signer saw a Permit2 PermitTransferFrom naming the paymaster as spender.
		// The chain is carried by domain.chainId — signing backends read it from there.
		expect(signTypedData).toHaveBeenCalledOnce()
		const [typedData] = signTypedData.mock.calls[0] as unknown as [ReturnType<typeof permit2TransferTypedData>]
		expect(typedData.primaryType).toBe("PermitTransferFrom")
		expect(typedData.domain).toEqual({ name: "Permit2", chainId: 56, verifyingContract: PERMIT2 })
		expect(typedData.message).toEqual({
			permitted: { token: USDC, amount: RECOMMENDED },
			spender: PAYMASTER,
			nonce,
			deadline,
		})
		expect(typedData.types.EIP712Domain.map((f) => f.name)).toEqual(["name", "chainId", "verifyingContract"])
	})

	it("bootstraps with exactly one approve(Permit2, max) and then uses PERMIT2 mode", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		const pm = await build(mockClient({ native: 10n ** 18n }), walletClient)

		expect(writeContract).toHaveBeenCalledOnce()
		expect(approveCall(writeContract).args).toEqual([PERMIT2, maxUint256])
		expect(slice(pm!.paymasterData, 0, 1)).toBe("0x02")
		expect(pm?.paymasterVerificationGasLimit).toBe(VERIFICATION_GAS_LIMIT_PERMIT2)
	})

	it("fails fast with an actionable message when the EOA cannot fund the bootstrap approve", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		await expect(build(mockClient({ native: 0n }), walletClient)).rejects.toThrow(
			/one-time funded approval .* send native dust/,
		)
		// The pre-check must reject before any tx (and its estimateGas) is attempted.
		expect(writeContract).not.toHaveBeenCalled()
	})

	it("keeps APPROVE mode with no tx while a legacy paymaster allowance is in place", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		const pm = await build(mockClient({ paymasterAllowance: THRESHOLD, native: 0n }), walletClient)

		expect(writeContract).not.toHaveBeenCalled()
		expect(pm?.paymasterData).toBe(encodePacked(["uint8", "address"], [1, USDC]))
		expect(pm?.paymasterVerificationGasLimit).toBe(VERIFICATION_GAS_LIMIT_APPROVE)
	})

	it("prefers PERMIT2 over a legacy paymaster allowance when both exist", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		const pm = await build(
			mockClient({ paymasterAllowance: THRESHOLD, permit2Allowance: maxUint256, native: 0n }),
			walletClient,
		)

		expect(writeContract).not.toHaveBeenCalled()
		expect(slice(pm!.paymasterData, 0, 1)).toBe("0x02")
	})

	it("still prefers EIP-2612 PERMIT mode when the token supports it and permits are not skipped", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		const pm = await build(
			mockClient({ permit: true, permit2Allowance: maxUint256, native: 0n }),
			walletClient,
			mockSigner().signer,
			false,
		)

		expect(writeContract).not.toHaveBeenCalled()
		expect(slice(pm!.paymasterData, 0, 1)).toBe("0x00")
		expect(pm?.paymasterVerificationGasLimit).toBe(VERIFICATION_GAS_LIMIT_PERMIT)
	})

	it("keeps the legacy bootstrap while the paymaster deployment predates PERMIT2 mode", async () => {
		const { walletClient, writeContract } = mockWalletClient()
		const legacyPaymaster = "0x1111111111111111111111111111111111111111" as HexString

		const pm = await buildSimplexPaymasterData(
			mockClient({ native: 10n ** 18n, permit2Capable: false, permit2Allowance: maxUint256 }),
			walletClient,
			mockSigner().signer,
			SOLVER,
			legacyPaymaster,
			CHAIN,
			configService,
			{ skipPermit: true },
		)

		expect(writeContract).toHaveBeenCalledOnce()
		expect(approveCall(writeContract).args).toEqual([legacyPaymaster, RECOMMENDED])
		expect(pm?.paymasterData).toBe(encodePacked(["uint8", "address"], [1, USDC]))
	})

	it("falls back to the capped approve(paymaster) bootstrap on chains without Permit2", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		const pm = await buildSimplexPaymasterData(
			mockClient({ native: 10n ** 18n }),
			walletClient,
			mockSigner().signer,
			SOLVER,
			PAYMASTER,
			CHAIN,
			makeConfigService("0x"),
			{ skipPermit: true },
		)

		expect(writeContract).toHaveBeenCalledOnce()
		expect(approveCall(writeContract).args).toEqual([PAYMASTER, RECOMMENDED])
		expect(pm?.paymasterData).toBe(encodePacked(["uint8", "address"], [1, USDC]))
	})
})

describe("Permit2 typed data", () => {
	it("hashes to the canonical Permit2 digest", () => {
		const params = {
			permit2: PERMIT2,
			chainId: 56,
			token: USDC,
			amount: RECOMMENDED,
			spender: PAYMASTER,
			nonce: 123456789n,
			deadline: 1_800_000_000n,
		}
		const typedData = permit2TransferTypedData(params)

		// Hand-derived from the Permit2 type strings so a typo in the typed data is caught.
		const domainTypehash = keccak256(toHex("EIP712Domain(string name,uint256 chainId,address verifyingContract)"))
		const domainSeparator = keccak256(
			encodeAbiParameters(
				[{ type: "bytes32" }, { type: "bytes32" }, { type: "uint256" }, { type: "address" }],
				[domainTypehash, keccak256(toHex("Permit2")), 56n, PERMIT2],
			),
		)
		const tokenPermissionsTypehash = keccak256(toHex("TokenPermissions(address token,uint256 amount)"))
		const permitTransferFromTypehash = keccak256(
			toHex(
				"PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)",
			),
		)
		const structHash = keccak256(
			encodeAbiParameters(
				[
					{ type: "bytes32" },
					{ type: "bytes32" },
					{ type: "address" },
					{ type: "uint256" },
					{ type: "uint256" },
				],
				[
					permitTransferFromTypehash,
					keccak256(
						encodeAbiParameters(
							[{ type: "bytes32" }, { type: "address" }, { type: "uint256" }],
							[tokenPermissionsTypehash, USDC, RECOMMENDED],
						),
					),
					PAYMASTER,
					params.nonce,
					params.deadline,
				],
			),
		)
		const expected = keccak256(concat(["0x1901", domainSeparator, structHash]))

		expect(hashTypedData(typedData as Parameters<typeof hashTypedData>[0])).toBe(expected)
	})

	it("normalizes signatures to 65 bytes with v in {27, 28}", () => {
		const rs = "0x" + "aa".repeat(32) + "bb".repeat(32)
		expect(normalizeSignature65(`${rs}00` as HexString)).toBe(`${rs}1b`)
		expect(normalizeSignature65(`${rs}1c` as HexString)).toBe(`${rs}1c`)
		expect(() => normalizeSignature65(`${rs}1c1c` as HexString)).toThrow(/64 or 65 bytes/)
	})
})
