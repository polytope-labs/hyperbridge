import { describe, expect, it, vi } from "vitest"
import { decodeAbiParameters, decodeFunctionData, parseAbi } from "viem"

const ERC7821_EXECUTE_ABI = parseAbi(["function execute(bytes32 mode, bytes executionData)"])
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import { TokenSender } from "@/services/TokenSender"
import type { ChainClientManager } from "@/services/ChainClientManager"

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
		waitForTransactionReceipt: vi.fn().mockResolvedValue({ status: "success" }),
	}
	const clientManager = {
		getPublicClient: () => publicClient,
		getWalletClient: () => ({ sendTransaction, chain: undefined }),
	} as unknown as ChainClientManager
	const vaults = [{ chain: "EVM-8453", vault: VAULT, ...(opts.minBalance ? { minBalance: opts.minBalance } : {}) }]
	const sender = new TokenSender(clientManager, SOLVER, () => vaults.map((v) => ({ ...v })))
	return { sender, sendTransaction }
}

/** The two calls of a batched send, decoded. */
function batchOf(sendTransaction: ReturnType<typeof vi.fn>) {
	const tx = sendTransaction.mock.calls[0][0]
	const outer = decodeFunctionData({ abi: ERC7821_EXECUTE_ABI, data: tx.data })
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
			throw new Error(`unexpected read: ${functionName}`)
		}),
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
