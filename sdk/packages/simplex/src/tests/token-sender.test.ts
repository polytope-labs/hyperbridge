import { describe, expect, it, vi } from "vitest"
import {
	decodeAbiParameters,
	decodeFunctionData,
	encodeFunctionData,
	maxUint256,
	parseAbi,
	WaitForTransactionReceiptTimeoutError,
} from "viem"

const ERC7821_EXECUTE_ABI = parseAbi(["function execute(bytes32 mode, bytes executionData)"])
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import { TokenSender } from "@/services/TokenSender"
import { UserOpSender } from "@/services/UserOpSender"
import { SubmittedUnconfirmedError, type SubmittedUserOp } from "@/services/submission"
import { buildPaymasterAndData } from "@/services/paymaster"
import {
	packPaymasterAndData,
	VERIFICATION_GAS_LIMIT_PERMIT2,
	POST_OP_GAS_LIMIT_SIMPLEX,
} from "@/services/paymaster/types"
import type { ChainClientManager } from "@/services/ChainClientManager"
import { resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"

// Whether a send must install the Permit2 approval is the paymaster provider's call; each test
// that cares says what it found. Left unset, nothing is pending and sends go out as they always did.
vi.mock("@/services/paymaster/provider/simplex", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/services/paymaster/provider/simplex")>()),
	resolvePendingPermit2Approval: vi.fn(),
}))

// The sponsored path under test runs the real UserOpSender, so its paymaster selection is stubbed;
// the deposit/balance reads behind it belong to the paymaster tests. The rest of the module
// (`paymasterReserveForToken`, used by the vault tests above) stays real.
vi.mock("@/services/paymaster", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/services/paymaster")>()),
	hasPaymaster: () => true,
	buildPaymasterAndData: vi.fn(),
}))

const SOLVER = "0x5b2c3e25243634732eE94525Ae98aEc404c82506" as const
const RECIPIENT = "0x1111111111111111111111111111111111111111" as const
const TOKEN = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" as const
const VAULT = "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6" as const

/** A sender whose wallet is short and whose vault holds the rest, as a real filler's does. */
function makeVaultSender(opts: {
	walletBalance: bigint
	maxWithdraw: bigint
	minBalance?: string
	vaultAsset?: `0x${string}`
	/** Reserve the paymaster would pull, in token units; implies the sponsored path. */
	reserve?: bigint
}) {
	const sendTransaction = vi.fn().mockResolvedValue("0xhash")
	const publicClient = {
		getBalance: vi.fn().mockResolvedValue(10n ** 18n),
		readContract: vi.fn(async ({ functionName }: { functionName: string }): Promise<unknown> => {
			if (functionName === "decimals") return 6
			if (functionName === "balanceOf") return opts.walletBalance
			if (functionName === "asset") return opts.vaultAsset ?? TOKEN
			if (functionName === "maxWithdraw") return opts.maxWithdraw
			throw new Error(`unexpected read: ${functionName}`)
		}),
		getTransactionCount: vi.fn().mockResolvedValue(7),
		waitForTransactionReceipt: vi.fn().mockResolvedValue({ status: "success" }),
	}
	const clientManager = {
		getPublicClient: () => publicClient,
		getWalletClient: () => ({ sendTransaction, chain: undefined }),
	} as unknown as ChainClientManager
	const vaults = [{ chain: "EVM-8453", vault: VAULT, ...(opts.minBalance ? { minBalance: opts.minBalance } : {}) }]
	// canSponsor decides whether a reserve applies at all; the amount comes from
	// the config service, which is where paymasterReserveForToken reads it.
	const userOpSender =
		opts.reserve === undefined
			? undefined
			: ({ canSponsor: () => true, trySendSponsored: async () => null } as never)
	const configService =
		opts.reserve === undefined
			? undefined
			: ({
					getPaymasterAddress: () => "0x15b3B03C870c7ef252029c35A12d3b339F5c8d7f",
					getSimplexPaymasterAddress: () => "0x15b3B03C870c7ef252029c35A12d3b339F5c8d7f",
					getCirclePaymasterAddress: () => undefined,
					getUsdcAsset: () => TOKEN,
					getUsdcDecimals: () => 6,
					getUsdtAsset: () => "0x",
					getUsdtDecimals: () => 6,
				} as never)
	const sender = new TokenSender(clientManager, SOLVER, () => vaults.map((v) => ({ ...v })), userOpSender, configService)
	return { sender, sendTransaction }
}

/** The calls of a batched send, decoded from its first native tx or from the given ERC-7821 callData. */
function batchOf(sendTransaction: ReturnType<typeof vi.fn>, callData?: `0x${string}`) {
	const data = callData ?? sendTransaction.mock.calls[0][0].data
	const outer = decodeFunctionData({ abi: ERC7821_EXECUTE_ABI, data })
	const [calls] = decodeAbiParameters(
		[
			{
				type: "tuple[]",
				components: [
					{ name: "target", type: "address" },
					{ name: "value", type: "uint256" },
					{ name: "data", type: "bytes" },
				],
			},
		],
		outer.args[1] as `0x${string}`,
	)
	return calls as Array<{ target: string; value: bigint; data: `0x${string}` }>
}

function makeSender(vaults: Array<{ chain: string; vault: `0x${string}` }> = []) {
	const sendTransaction = vi.fn().mockResolvedValue("0xhash")
	const publicClient = {
		getBalance: vi.fn().mockResolvedValue(10n ** 18n),
		readContract: vi.fn(async ({ functionName }: { functionName: string }): Promise<number | bigint> => {
			if (functionName === "decimals") return 6
			if (functionName === "balanceOf") return 1_000_000_000n
			// Uncapped unless a test overrides it, so the share sends below keep working.
			if (functionName === "maxRedeem") return 1_000_000_000n
			throw new Error(`unexpected read: ${functionName}`)
		}),
		getTransactionCount: vi.fn().mockResolvedValue(7),
		waitForTransactionReceipt: vi.fn().mockResolvedValue({ status: "success" }),
	}
	const clientManager = {
		getPublicClient: () => publicClient,
		getWalletClient: () => ({ sendTransaction, chain: undefined }),
	} as unknown as ChainClientManager
	const sender = new TokenSender(clientManager, SOLVER, () => vaults.map((v) => ({ ...v })))
	return { sender, sendTransaction, publicClient }
}

describe("TokenSender", () => {
	it("sends an ERC-20 transfer with decimals-parsed units", async () => {
		const { sender, sendTransaction } = makeSender()
		const result = await sender.send({ chain: "EVM-8453", token: TOKEN, amount: "25", to: RECIPIENT })
		expect(result).toEqual({ txHash: "0xhash", sponsored: false, redeemed: false })

		const tx = sendTransaction.mock.calls[0][0]
		expect(tx.to).toBe(TOKEN)
		const decoded = decodeFunctionData({ abi: ERC20_ABI, data: tx.data })
		expect(decoded.functionName).toBe("transfer")
		expect(decoded.args).toEqual([RECIPIENT, 25_000_000n])
	})

	it("redeems configured vault shares straight to the recipient", async () => {
		const { sender, sendTransaction } = makeSender([{ chain: "EVM-8453", vault: VAULT }])
		const result = await sender.send({ chain: "EVM-8453", token: VAULT, amount: "10", to: RECIPIENT })
		expect(result.redeemed).toBe(true)

		const tx = sendTransaction.mock.calls[0][0]
		expect(tx.to).toBe(VAULT)
		const decoded = decodeFunctionData({ abi: ERC4626_ABI, data: tx.data })
		expect(decoded.functionName).toBe("redeem")
		expect(decoded.args).toEqual([10_000_000n, RECIPIENT, SOLVER])
	})

	it("treats the same address as a plain transfer on a chain where it is not a configured vault", async () => {
		const { sender, sendTransaction } = makeSender([{ chain: "EVM-42161", vault: VAULT }])
		const result = await sender.send({ chain: "EVM-8453", token: VAULT, amount: "10", to: RECIPIENT })
		expect(result.redeemed).toBe(false)
		const decoded = decodeFunctionData({ abi: ERC20_ABI, data: sendTransaction.mock.calls[0][0].data })
		expect(decoded.functionName).toBe("transfer")
	})

	it("refuses a vault-share send above the vault's maxRedeem before submitting anything", async () => {
		// A liquidity-capped vault would revert `redeem(units)`; the guard refuses up front with
		// the amount it can actually redeem instead of building a batch that cannot succeed.
		const { sender, sendTransaction, publicClient } = makeSender([{ chain: "EVM-8453", vault: VAULT }])
		publicClient.readContract.mockImplementation(async ({ functionName }: { functionName: string }) => {
			if (functionName === "decimals") return 6
			if (functionName === "balanceOf") return 1_000_000_000n
			if (functionName === "maxRedeem") return 4_000_000n
			throw new Error(`unexpected read: ${functionName}`)
		})

		await expect(sender.send({ chain: "EVM-8453", token: VAULT, amount: "10", to: RECIPIENT })).rejects.toThrow(
			/only redeem 4000000 shares/,
		)
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("sends native value directly", async () => {
		const { sender, sendTransaction } = makeSender()
		await sender.send({ chain: "EVM-8453", token: "native", amount: "0.5", to: RECIPIENT })
		const tx = sendTransaction.mock.calls[0][0]
		expect(tx.to).toBe(RECIPIENT)
		expect(tx.value).toBe(5n * 10n ** 17n)
		expect(tx.data).toBe("0x")
	})

	it("leaves the wallet at the vault's floor instead of at zero", async () => {
		// The numbers from the send that reverted on Base at block 51039630: the
		// wallet held 9.989773 USDC, the operator sent 4000, and the withdrawal was
		// sized to make up exactly the difference. The paymaster then debited its
		// gas from the same USDC during validation, so the transfer was short by
		// that much and the whole batch reverted.
		const { sender, sendTransaction } = makeVaultSender({
			walletBalance: 9_989_773n,
			maxWithdraw: 481_970_486_507n,
			minBalance: "3000",
		})
		const result = await sender.send({ chain: "EVM-8453", token: TOKEN, amount: "4000", to: RECIPIENT })
		expect(result.redeemed).toBe(true)

		const calls = batchOf(sendTransaction)
		expect(calls).toHaveLength(2)
		const withdrawal = decodeFunctionData({ abi: ERC4626_ABI, data: calls[0].data })
		expect(withdrawal.functionName).toBe("withdraw")
		// The shortfall plus the floor, not the shortfall alone.
		expect(withdrawal.args?.[0]).toBe(4_000_000_000n - 9_989_773n + 3_000_000_000n)
		const transfer = decodeFunctionData({ abi: ERC20_ABI, data: calls[1].data })
		expect(transfer.args).toEqual([RECIPIENT, 4_000_000_000n])
	})

	it("withdraws only the shortfall when the vault declares no floor", async () => {
		// Withdraw-only vaults omit minBalance; nothing changes for them.
		const { sender, sendTransaction } = makeVaultSender({
			walletBalance: 9_989_773n,
			maxWithdraw: 481_970_486_507n,
		})
		await sender.send({ chain: "EVM-8453", token: TOKEN, amount: "4000", to: RECIPIENT })
		const withdrawal = decodeFunctionData({ abi: ERC4626_ABI, data: batchOf(sendTransaction)[0].data })
		expect(withdrawal.args?.[0]).toBe(4_000_000_000n - 9_989_773n)
	})

	it("falls back to the bare shortfall when the vault cannot cover the floor as well", async () => {
		// A floor is a preference. Refusing a transfer the operator asked for
		// because the vault is nearly empty would be worse than sending it.
		const { sender, sendTransaction } = makeVaultSender({
			walletBalance: 9_989_773n,
			maxWithdraw: 4_000_000_000n,
			minBalance: "3000",
		})
		const result = await sender.send({ chain: "EVM-8453", token: TOKEN, amount: "4000", to: RECIPIENT })
		expect(result.redeemed).toBe(true)
		const withdrawal = decodeFunctionData({ abi: ERC4626_ABI, data: batchOf(sendTransaction)[0].data })
		expect(withdrawal.args?.[0]).toBe(4_000_000_000n - 9_989_773n)
	})

	it("reserves the paymaster's gas when it is larger than the vault floor", async () => {
		// The floor only helps if it is set and big enough. The reserve is what the
		// paymaster will actually pull out of this wallet during validation, and it
		// applies even to a withdraw-only vault with no floor of its own.
		const { sender, sendTransaction } = makeVaultSender({
			walletBalance: 9_989_773n,
			maxWithdraw: 481_970_486_507n,
			reserve: 2_000_000n,
		})
		await sender.send({ chain: "EVM-8453", token: TOKEN, amount: "4000", to: RECIPIENT })
		const withdrawal = decodeFunctionData({ abi: ERC4626_ABI, data: batchOf(sendTransaction)[0].data })
		expect(withdrawal.args?.[0]).toBe(4_000_000_000n - 9_989_773n + 2_000_000n)
	})

	it("tops the wallet up when it covers the transfer but not the paymaster's pull", async () => {
		// The case a floor alone never reaches: the wallet covers the amount, so the
		// old code built a lone transfer and left nothing for validation to take.
		const { sender, sendTransaction } = makeVaultSender({
			walletBalance: 4_000_000_000n,
			maxWithdraw: 481_970_486_507n,
			reserve: 2_000_000n,
		})
		const result = await sender.send({ chain: "EVM-8453", token: TOKEN, amount: "4000", to: RECIPIENT })
		expect(result.redeemed).toBe(true)
		const calls = batchOf(sendTransaction)
		expect(calls).toHaveLength(2)
		const withdrawal = decodeFunctionData({ abi: ERC4626_ABI, data: calls[0].data })
		expect(withdrawal.args?.[0]).toBe(2_000_000n)
	})

	it("still refuses when no vault holds the asset at all", async () => {
		const { sender } = makeVaultSender({
			walletBalance: 9_989_773n,
			maxWithdraw: 481_970_486_507n,
			minBalance: "3000",
			vaultAsset: "0x2222222222222222222222222222222222222222",
		})
		await expect(sender.send({ chain: "EVM-8453", token: TOKEN, amount: "4000", to: RECIPIENT })).rejects.toThrow(
			/Insufficient token balance/,
		)
	})

	it("rejects bad inputs before submitting anything", async () => {
		const { sender, sendTransaction, publicClient } = makeSender()
		await expect(sender.send({ chain: "EVM-8453", token: TOKEN, amount: "25", to: "0x123" as never })).rejects.toThrow(
			"Invalid recipient",
		)
		await expect(sender.send({ chain: "EVM-8453", token: "not-an-address", amount: "1", to: RECIPIENT })).rejects.toThrow(
			"Invalid token",
		)
		await expect(sender.send({ chain: "EVM-8453", token: TOKEN, amount: "0", to: RECIPIENT })).rejects.toThrow(
			"greater than 0",
		)
		publicClient.readContract.mockImplementation(async ({ functionName }: { functionName: string }) => {
			if (functionName === "decimals") return 6
			return 1_000n // balance below requested
		})
		await expect(sender.send({ chain: "EVM-8453", token: TOKEN, amount: "25", to: RECIPIENT })).rejects.toThrow(
			"Insufficient token balance",
		)
		expect(sendTransaction).not.toHaveBeenCalled()
	})
})

describe("TokenSender sponsored batches", () => {
	/** The sponsored op a successful send reports. */
	const USER_OP: SubmittedUserOp = {
		kind: "userOp",
		userOpHash: `0x${"cd".repeat(32)}`,
		nonce: 5n,
		nonceKey: 0n,
		expires: true,
		submittedAt: 1_000,
	}

	const FEE_TOKEN = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359" as const
	const PERMIT2 = "0x000000000022D473030F116dDEE9F6B43aC78BA3" as const
	const PAYMASTER = "0xe99acFe0f5fC4C8ea54A187D8D3b05f136150095" as const
	const transfer = {
		target: TOKEN,
		value: 0n,
		data: encodeFunctionData({ abi: ERC20_ABI, functionName: "transfer", args: [RECIPIENT, 25_000_000n] }),
	}

	function makeSponsoredSender(sponsored: { txHash: `0x${string}`; userOp: SubmittedUserOp } | null = { txHash: "0xsponsored", userOp: USER_OP }) {
		const sendTransaction = vi.fn().mockResolvedValue("0xnative")
		const trySendSponsored = vi.fn().mockResolvedValue(sponsored)
		const clientManager = {
			getPublicClient: () => ({
				getTransactionCount: vi.fn().mockResolvedValue(7),
				waitForTransactionReceipt: vi.fn().mockResolvedValue({ status: "success" }),
			}),
			getWalletClient: () => ({ sendTransaction, chain: undefined }),
		} as unknown as ChainClientManager
		const sender = new TokenSender(
			clientManager,
			SOLVER,
			() => [],
			{ canSponsor: () => true, trySendSponsored } as never,
			{ getSimplexPaymasterAddress: () => PAYMASTER } as never,
		)
		return { sender, trySendSponsored, sendTransaction }
	}

	/** The calls inside a sponsored op's ERC-7821 `execute` callData. */
	function sponsoredBatch(trySendSponsored: ReturnType<typeof vi.fn>) {
		const { callData } = trySendSponsored.mock.calls[0][0]
		return batchOf(trySendSponsored, callData)
	}

	it("pays with a permit and installs the Permit2 approval when the wallet has none", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValueOnce({
			token: FEE_TOKEN,
			spender: PERMIT2,
			permitCapable: true,
		})
		const { sender, trySendSponsored, sendTransaction } = makeSponsoredSender()

		expect(await sender.submitCalls("EVM-137", [transfer])).toEqual({ txHash: "0xsponsored", sponsored: true, submission: USER_OP })

		const request = trySendSponsored.mock.calls[0][0]
		expect(request.permitBootstrap).toBe(true)
		expect(request.gas.callGasLimit).toBe(900_000n)
		const [approve, sent] = sponsoredBatch(trySendSponsored)
		expect(approve.target).toBe(FEE_TOKEN)
		expect(decodeFunctionData({ abi: ERC20_ABI, data: approve.data }).args).toEqual([PERMIT2, maxUint256])
		expect(sent.target).toBe(TOKEN)
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it.each([
		["the fee token has no permit", { token: FEE_TOKEN, spender: PERMIT2, permitCapable: false }],
		["the approval is already in place", null],
	])("sends the batch unchanged when %s", async (_case, pending) => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValueOnce(pending as never)
		const { sender, trySendSponsored } = makeSponsoredSender()

		await sender.submitCalls("EVM-137", [transfer])

		const request = trySendSponsored.mock.calls[0][0]
		expect(request.permitBootstrap).toBe(false)
		expect(sponsoredBatch(trySendSponsored).map((call) => call.target)).toEqual([TOKEN])
	})

	it("sends the sponsored op on the nonce key the caller names", async () => {
		const { sender, trySendSponsored } = makeSponsoredSender()

		await sender.submitCalls("EVM-137", [transfer], { nonceKey: 7n })

		expect(trySendSponsored.mock.calls[0][0].nonceKey).toBe(7n)
	})

	it("leaves the nonce key unset for a caller that names none, so the op uses key 0", async () => {
		const { sender, trySendSponsored } = makeSponsoredSender()

		await sender.submitCalls("EVM-137", [transfer])

		expect(trySendSponsored.mock.calls[0][0].nonceKey).toBeUndefined()
	})

	it("sends the batch unchanged when the approval check fails", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockRejectedValueOnce(new Error("429 Too Many Requests"))
		const { sender, trySendSponsored } = makeSponsoredSender()

		await sender.submitCalls("EVM-137", [transfer])

		expect(trySendSponsored.mock.calls[0][0].permitBootstrap).toBe(false)
	})

	it("leaves the bootstrap approve out of the native fallback", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValueOnce({
			token: FEE_TOKEN,
			spender: PERMIT2,
			permitCapable: true,
		})
		const { sender, sendTransaction } = makeSponsoredSender(null)

		expect(await sender.submitCalls("EVM-137", [transfer])).toMatchObject({
			txHash: "0xnative",
			sponsored: false,
			submission: { kind: "tx", txHash: "0xnative", nonce: 7 },
		})

		const tx = sendTransaction.mock.calls[0][0]
		expect(tx.to).toBe(TOKEN)
		expect(decodeFunctionData({ abi: ERC20_ABI, data: tx.data }).functionName).toBe("transfer")
	})
})

describe("TokenSender native batch safety", () => {
	const TX = `0x${"ab".repeat(32)}` as `0x${string}`
	const CALL = { target: TOKEN, value: 0n, data: "0x1234" as `0x${string}` }
	const BATCH = [CALL, { target: TOKEN, value: 0n, data: "0x5678" as `0x${string}` }]

	function makeNativeSender(isDelegated?: (chain: string) => Promise<boolean>) {
		const sendTransaction = vi.fn().mockResolvedValue(TX)
		const waitForTransactionReceipt = vi.fn().mockResolvedValue({ status: "success" })
		const getTransactionCount = vi.fn().mockResolvedValue(7)
		const clientManager = {
			getPublicClient: () => ({ waitForTransactionReceipt, getTransactionCount }),
			getWalletClient: () => ({ sendTransaction, chain: undefined }),
		} as unknown as ChainClientManager
		const sender = new TokenSender(clientManager, SOLVER, () => [], undefined, undefined, isDelegated)
		return { sender, sendTransaction, waitForTransactionReceipt, getTransactionCount }
	}

	it("reports a native tx whose receipt timed out as submitted but unconfirmed", async () => {
		const { sender, waitForTransactionReceipt } = makeNativeSender()
		waitForTransactionReceipt.mockRejectedValue(new WaitForTransactionReceiptTimeoutError({ hash: TX }))

		const error = await sender.submitCalls("EVM-8453", [CALL]).catch((err) => err)

		expect(error).toBeInstanceOf(SubmittedUnconfirmedError)
		expect(error.message).toContain("Timed out while waiting for transaction")
		expect(error.chain).toBe("EVM-8453")
		// The nonce travels with the hash: a node forgetting the tx proves nothing, the nonce moving does.
		expect(error.submission).toMatchObject({ kind: "tx", txHash: TX, nonce: 7 })
	})

	it("reports any receipt-read failure as submitted but unconfirmed", async () => {
		const { sender, waitForTransactionReceipt } = makeNativeSender()
		// Not a timeout: the node accepted the tx, so its outcome is still unknown.
		waitForTransactionReceipt.mockRejectedValue(new Error("connection reset"))

		const error = await sender.submitCalls("EVM-8453", [CALL]).catch((err) => err)

		expect(error).toBeInstanceOf(SubmittedUnconfirmedError)
		expect(error.message).toContain("connection reset")
		expect(error.submission).toMatchObject({ kind: "tx", txHash: TX })
	})

	it("sends with the wallet's pending nonce, chosen explicitly", async () => {
		const { sender, sendTransaction, getTransactionCount } = makeNativeSender()

		await sender.submitCalls("EVM-8453", [CALL])

		expect(getTransactionCount).toHaveBeenCalledWith({ address: SOLVER, blockTag: "pending" })
		expect(sendTransaction.mock.calls[0][0].nonce).toBe(7)
	})

	it("reports a mined revert as a plain failure, not an unconfirmed submission", async () => {
		const { sender, waitForTransactionReceipt } = makeNativeSender()
		waitForTransactionReceipt.mockResolvedValue({ status: "reverted" })

		const error = await sender.submitCalls("EVM-8453", [CALL]).catch((err) => err)

		expect(error).not.toBeInstanceOf(SubmittedUnconfirmedError)
		expect(error.message).toContain(`Send tx reverted: ${TX}`)
	})

	it("refuses a native batch on a wallet that is not 7702-delegated", async () => {
		const { sender, sendTransaction } = makeNativeSender(async () => false)

		await expect(sender.submitCalls("EVM-8453", BATCH)).rejects.toThrow(
			"The wallet is not 7702-delegated on EVM-8453 (or its delegation could not be read); a batch sent to it would do nothing",
		)
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("sends a native batch through executeBatch when the wallet is delegated", async () => {
		const { sender, sendTransaction } = makeNativeSender(async () => true)

		expect(await sender.submitCalls("EVM-8453", BATCH)).toMatchObject({ txHash: TX, sponsored: false })
		expect(sendTransaction.mock.calls[0][0].to).toBe(SOLVER)
	})

	it("sends a single native call straight to the target regardless of delegation", async () => {
		const { sender, sendTransaction } = makeNativeSender(async () => false)

		expect(await sender.submitCalls("EVM-8453", [CALL])).toMatchObject({ txHash: TX, sponsored: false })
		expect(sendTransaction.mock.calls[0][0].to).toBe(TOKEN)
	})
})

/**
 * The c07 regression, end to end through the real UserOpSender: a lost `eth_sendUserOperation`
 * response must not let `submitCalls` re-send the same calls as a native tx. Only a definite
 * "the bundler never got it" may fall back.
 */
describe("TokenSender sponsored send with a lost bundler response", () => {
	const FEE_TOKEN = "0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359" as const
	const PAYMASTER = "0xe99acFe0f5fC4C8ea54A187D8D3b05f136150095" as const
	const ENTRY_POINT = "0x4337843378433784337843378433784337843378" as const
	const HTML_PAGE = Symbol("html-page")
	const transfer = {
		target: TOKEN,
		value: 0n,
		data: encodeFunctionData({ abi: ERC20_ABI, functionName: "transfer", args: [RECIPIENT, 25_000_000n] }),
	}
	const paymasterAndData = packPaymasterAndData({
		paymaster: PAYMASTER,
		paymasterData: "0x02" as `0x${string}`,
		paymasterVerificationGasLimit: VERIFICATION_GAS_LIMIT_PERMIT2,
		paymasterPostOpGasLimit: POST_OP_GAS_LIMIT_SIMPLEX,
	})

	/** A TokenSender wired to a real UserOpSender, with only the chain/bundler transport faked. */
	function makeRealSender(bundlerUrl: string) {
		const sendTransaction = vi.fn().mockResolvedValue("0xnative")
		const publicClient = {
			getGasPrice: async () => 1_000_000_000n,
			getBlock: async () => ({ baseFeePerGas: 900_000_000n }),
			readContract: async () => 0n,
			getTransactionCount: async () => 7,
			waitForTransactionReceipt: vi.fn().mockResolvedValue({ status: "success" }),
		}
		const clientManager = {
			getPublicClient: () => publicClient,
			getWalletClient: () => ({ sendTransaction, chain: undefined }),
		} as unknown as ChainClientManager
		const signer = { address: SOLVER, signTypedData: async () => ("0x" + "11".repeat(65)) as `0x${string}` }
		const configService = {
			getEntryPointAddress: () => ENTRY_POINT,
			getBundlerUrl: () => bundlerUrl,
			getChainId: () => 8453,
			getSimplexPaymasterAddress: () => PAYMASTER,
		} as never
		const userOpSender = new UserOpSender(clientManager, configService, signer as never)
		const sender = new TokenSender(clientManager, SOLVER, () => [], userOpSender, configService)
		return { sender, sendTransaction }
	}

	const fetchMethods: string[] = []

	const stubFetch = (byMethod: Record<string, unknown>) => {
		vi.stubGlobal("fetch", async (_url: unknown, init?: { body?: string }) => {
			const { method } = JSON.parse(init?.body ?? "{}") as { method: string }
			fetchMethods.push(method)
			const reply = byMethod[method]
			if (reply === undefined) {
				return { status: 404, json: async () => ({ jsonrpc: "2.0", id: 1, error: { code: -32601 } }) }
			}
			if (reply === HTML_PAGE) {
				return {
					status: 504,
					json: async () => {
						throw new SyntaxError("Unexpected token '<'")
					},
				}
			}
			if (reply instanceof Error) throw reply
			const { status, ...body } = reply as { status?: number } & object
			return { status: status ?? 200, json: async () => ({ jsonrpc: "2.0", id: 1, ...body }) }
		})
	}

	const drainLookups = async () => {
		for (const delay of [2_000, 5_000, 10_000]) await vi.advanceTimersByTimeAsync(delay)
	}

	beforeEach(() => {
		fetchMethods.length = 0
		vi.mocked(buildPaymasterAndData).mockResolvedValue({
			paymasterAndData,
			type: "simplex",
			address: PAYMASTER,
			token: FEE_TOKEN,
		})
	})

	afterEach(() => {
		vi.useRealTimers()
		vi.unstubAllGlobals()
	})

	it("rejects with SubmittedUnconfirmedError and never sends natively when the lookup fails too", async () => {
		const timeout = Object.assign(new Error("This operation was aborted"), { name: "TimeoutError" })
		const reset = Object.assign(new Error("socket closed"), { cause: { code: "ECONNRESET" } })
		stubFetch({ eth_sendUserOperation: timeout, eth_getUserOperationByHash: reset })
		const { sender, sendTransaction } = makeRealSender("http://tok-unknown.test/bundler")
		vi.useFakeTimers()

		const promise = sender.submitCalls("EVM-8453", [transfer]).catch((err) => err)
		await drainLookups()
		const error = await promise

		expect(error).toBeInstanceOf(SubmittedUnconfirmedError)
		expect(error.submission).toMatchObject({ kind: "userOp" })
		expect(sendTransaction).not.toHaveBeenCalled()
	})

	it("falls back to a native tx when the bundler confirms it never received the op", async () => {
		stubFetch({ eth_sendUserOperation: HTML_PAGE, eth_getUserOperationByHash: { result: null } })
		const { sender, sendTransaction } = makeRealSender("http://tok-absent.test/bundler")
		vi.useFakeTimers()

		const promise = sender.submitCalls("EVM-8453", [transfer])
		await drainLookups()

		expect(await promise).toMatchObject({ txHash: "0xnative", sponsored: false })
		expect(sendTransaction).toHaveBeenCalledOnce()
	})

	it("treats a proxy's JSON 502 as ambiguous and recovers the op, not resending natively", async () => {
		const txHash = `0x${"ef".repeat(32)}`
		stubFetch({
			eth_sendUserOperation: { status: 502, error: { code: 504, message: "upstream timed out" } },
			eth_getUserOperationByHash: { result: { userOpHash: `0x${"ab".repeat(32)}` } },
			eth_getUserOperationReceipt: { result: { success: true, receipt: { transactionHash: txHash } } },
		})
		const { sender, sendTransaction } = makeRealSender("http://tok-502.test/bundler")
		vi.useFakeTimers()

		const promise = sender.submitCalls("EVM-8453", [transfer])
		await drainLookups()

		expect(await promise).toMatchObject({ txHash, sponsored: true })
		expect(fetchMethods).toContain("eth_getUserOperationByHash")
		expect(sendTransaction).not.toHaveBeenCalled()
	})
})
