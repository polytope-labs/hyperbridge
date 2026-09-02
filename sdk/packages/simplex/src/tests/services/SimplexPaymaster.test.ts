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
	ContractFunctionExecutionError,
	ContractFunctionRevertedError,
	HttpRequestError,
	type PublicClient,
	type WalletClient,
} from "viem"
import type { HexString } from "@hyperbridge/sdk"

import { buildSimplexPaymasterData, resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"
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
	/** Set false to make PERMIT2() throw a transport (non-contract) error. */
	permit2Transport?: boolean
	/** Set true to make the token's version() probe throw a transport error. */
	permitTransport?: boolean
}): PublicClient {
	return {
		readContract: async ({ functionName, args }: { functionName: string; args?: unknown[] }) => {
			switch (functionName) {
				case "PERMIT2":
					// Both failure shapes follow viem's readContract wrapping: every failure —
					// transport included — arrives as ContractFunctionExecutionError, and only
					// the cause chain separates a 429 from a genuine revert. (Real viem nests
					// the HttpRequestError one level deeper, behind a CallExecutionError; the
					// classifier walks the chain, so depth is irrelevant.) A bare Error here
					// would dodge the classifier and test nothing.
					if (opts.permit2Transport === false) {
						throw new ContractFunctionExecutionError(
							new HttpRequestError({
								url: "https://rpc.example",
								status: 429,
								details: "Too Many Requests",
							}),
							{ abi: [], functionName: "PERMIT2" } as never,
						)
					}
					// An old deployment reverts PERMIT2(); the client only treats a genuine
					// contract revert (ContractFunctionRevertedError in the cause chain) as
					// "unsupported".
					if (opts.permit2Capable === false) {
						throw new ContractFunctionExecutionError(
							new ContractFunctionRevertedError({
								abi: [],
								functionName: "PERMIT2",
								message: "execution reverted",
							}),
							{ abi: [], functionName: "PERMIT2" } as never,
						)
					}
					return PERMIT2
				case "balanceOf":
					return 5n * 10n ** BigInt(DECIMALS)
				case "allowance": {
					const spender = (args?.[1] as string).toLowerCase()
					if (spender === PERMIT2.toLowerCase()) return opts.permit2Allowance ?? 0n
					return opts.paymasterAllowance ?? 0n
				}
				case "version":
					if (opts.permitTransport) {
						throw new ContractFunctionExecutionError(
							new HttpRequestError({
								url: "https://rpc.example",
								status: 429,
								details: "Too Many Requests",
							}),
							{ abi: [], functionName: "version" } as never,
						)
					}
					if (opts.permit) return "2"
					// A token without version() reverts the read; viem wraps it the same
					// way as the PERMIT2 probe above, and only the revert in the cause
					// chain lets tokenSupportsPermit treat it as "no permit".
					throw new ContractFunctionExecutionError(
						new ContractFunctionRevertedError({
							abi: [],
							functionName: "version",
							message: "execution reverted",
						}),
						{ abi: [], functionName: "version" } as never,
					)
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
		// Signature laid out as v (1) ‖ r (32) ‖ s (32); SIG = r(0x11*32) ‖ s(0x11*32) ‖ v(0x1b).
		expect(slice(data, 117, 118)).toBe("0x1b")
		expect(slice(data, 118, 150)).toBe(`0x${"11".repeat(32)}`)
		expect(slice(data, 150, 182)).toBe(`0x${"11".repeat(32)}`)
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

	it("zeroes a stale Permit2 allowance before approving max (Ethereum USDT rule)", async () => {
		const { walletClient, writeContract } = mockWalletClient()
		const stalePaymaster = "0x3333333333333333333333333333333333333333" as HexString

		const pm = await buildSimplexPaymasterData(
			// A leftover sub-$5 Permit2 allowance from another integration.
			mockClient({ permit2Allowance: 1n, native: 10n ** 18n }),
			walletClient,
			mockSigner().signer,
			SOLVER,
			stalePaymaster,
			CHAIN,
			configService,
			{ skipPermit: true },
		)

		// approve(Permit2, 0) then approve(Permit2, max) — never a non-zero → non-zero change.
		expect(writeContract).toHaveBeenCalledTimes(2)
		const calls = writeContract.mock.calls as unknown as [{ args: unknown[] }][]
		expect(calls[0][0].args).toEqual([PERMIT2, 0n])
		expect(calls[1][0].args).toEqual([PERMIT2, maxUint256])
		expect(slice(pm!.paymasterData, 0, 1)).toBe("0x02")
	})

	it("propagates a transport error rather than caching the paymaster as non-PERMIT2", async () => {
		const { walletClient } = mockWalletClient()
		const flakyPaymaster = "0x4444444444444444444444444444444444444444" as HexString

		// A 429/timeout on PERMIT2() must not silently drop the solver to a native approve;
		// it must surface so the caller retries, leaving the negative uncached.
		await expect(
			buildSimplexPaymasterData(
				mockClient({ permit2Transport: false, native: 10n ** 18n }),
				walletClient,
				mockSigner().signer,
				SOLVER,
				flakyPaymaster,
				CHAIN,
				configService,
				{ skipPermit: true },
			),
		).rejects.toThrow(/HTTP request failed/)

		// And nothing was cached: the very next attempt with a healthy RPC probes again
		// and lands in PERMIT2 mode instead of being pinned to "unsupported" for the TTL.
		const { walletClient: retryWallet, writeContract: retryWrite } = mockWalletClient()
		const pm = await buildSimplexPaymasterData(
			mockClient({ permit2Allowance: maxUint256, native: 0n }),
			retryWallet,
			mockSigner().signer,
			SOLVER,
			flakyPaymaster,
			CHAIN,
			configService,
			{ skipPermit: true },
		)
		expect(retryWrite).not.toHaveBeenCalled()
		expect(slice(pm!.paymasterData, 0, 1)).toBe("0x02")
	})

	it("propagates a transport error from the token permit probe rather than treating it as no-permit", async () => {
		const { walletClient, writeContract } = mockWalletClient()

		// A 429/timeout on version() must not masquerade as "token has no permit" —
		// that would route a fresh solver into a native-funded approve(Permit2, max)
		// instead of the txless PERMIT mode a healthy probe would have picked.
		await expect(
			buildSimplexPaymasterData(
				mockClient({ permitTransport: true, native: 10n ** 18n }),
				walletClient,
				mockSigner().signer,
				SOLVER,
				PAYMASTER,
				CHAIN,
				configService,
				{ skipPermit: false },
			),
		).rejects.toThrow(/HTTP request failed/)
		expect(writeContract).not.toHaveBeenCalled()
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

describe("resolvePendingPermit2Approval (native-delegation batching)", () => {
	it("returns the fee token to approve when a no-permit token has no Permit2 allowance", async () => {
		const pending = await resolvePendingPermit2Approval(
			mockClient({ native: 0n }),
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
		)
		expect(pending).toEqual({ token: USDC, spender: PERMIT2 })
	})

	it("returns null for a stale non-zero allowance — the batched tx cannot zero-first", async () => {
		// USDT's non-zero → non-zero rule would revert the batched approve(max), and the
		// delegation tx skips simulation, so this must defer to sendFundedApprove instead.
		const pending = await resolvePendingPermit2Approval(
			mockClient({ permit2Allowance: 1n }),
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
		)
		expect(pending).toBeNull()
	})

	it("returns null at the recommended allowance — PERMIT2 mode already works, nothing to batch", async () => {
		const pending = await resolvePendingPermit2Approval(
			mockClient({ permit2Allowance: RECOMMENDED }),
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
		)
		expect(pending).toBeNull()
	})

	it("returns null for a permit-capable token (PERMIT mode handles it)", async () => {
		const pending = await resolvePendingPermit2Approval(
			mockClient({ permit: true }),
			SOLVER,
			PAYMASTER,
			CHAIN,
			configService,
		)
		expect(pending).toBeNull()
	})

	it("returns null on chains without Permit2 configured", async () => {
		const pending = await resolvePendingPermit2Approval(
			mockClient({ native: 0n }),
			SOLVER,
			PAYMASTER,
			CHAIN,
			makeConfigService("0x"),
		)
		expect(pending).toBeNull()
	})

	it("returns null when the paymaster deployment predates PERMIT2 mode", async () => {
		const legacyPaymaster = "0x2222222222222222222222222222222222222222" as HexString
		const pending = await resolvePendingPermit2Approval(
			mockClient({ native: 0n, permit2Capable: false }),
			SOLVER,
			legacyPaymaster,
			CHAIN,
			configService,
		)
		expect(pending).toBeNull()
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
