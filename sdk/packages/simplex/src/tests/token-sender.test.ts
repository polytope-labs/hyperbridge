import { describe, expect, it, vi } from "vitest"
import { decodeFunctionData } from "viem"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import { TokenSender } from "@/services/TokenSender"
import type { ChainClientManager } from "@/services/ChainClientManager"

const SOLVER = "0x5b2c3e25243634732eE94525Ae98aEc404c82506" as const
const RECIPIENT = "0x1111111111111111111111111111111111111111" as const
const TOKEN = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913" as const
const VAULT = "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6" as const

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
