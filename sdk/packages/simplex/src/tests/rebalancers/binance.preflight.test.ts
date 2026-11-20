import { describe, it, expect, vi, beforeEach } from "vitest"
import { getAddress } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { BinanceRebalancer } from "@/services/rebalancers/binance"
import type { UnifiedRebalanceOptions } from "@/services/rebalancers"

/**
 * Offline deposit preflight for the Binance CEX leg. `binance.test.ts` needs live keys and throws
 * at module load without them, so this stubs `@binance/wallet`'s REST client and the on-chain
 * client to exercise the guards that run before any transfer is sent.
 */

const USDC = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d" as HexString
const SOLVER = "0x13E41CdE1D55880cbe031c69f206C2E9BC3c94C2" as HexString
const DEPOSIT_ADDRESS = "0x000000000000000000000000000000000000dead"
const OPTIONS: UnifiedRebalanceOptions = { coin: "USDC", source: "EVM-56", destination: "EVM-42161", amount: "100" }

const { restAPI } = vi.hoisted(() => ({
	restAPI: {
		allCoinsInformation: vi.fn(),
		depositAddress: vi.fn(),
		depositHistory: vi.fn(),
		withdraw: vi.fn(),
		withdrawHistory: vi.fn(),
		withdrawHistoryV2: vi.fn(),
		withdrawTravelRule: vi.fn(),
	},
}))

// The constructor builds a Wallet; every REST call the rebalancer makes goes through this stub.
vi.mock("@binance/wallet", () => ({
	Wallet: class {
		restAPI = restAPI
	},
}))

/** Binance's coin/network config, with `contractAddress` overridable for the drift cases. */
function allCoins(contractAddress: string | undefined) {
	const network = { contractAddress, depositEnable: true, withdrawEnable: true, withdrawFee: "1", withdrawMin: "1" }
	return [
		{
			coin: "USDC",
			networkList: [
				{ ...network, network: "BSC", minConfirm: 15 },
				{ ...network, network: "ARBITRUM", estimatedArrivalTime: 5 },
			],
		},
	]
}

function makeRebalancer(token: HexString = USDC) {
	const writeContract = vi.fn().mockResolvedValue("0xdeposit" as HexString)
	const publicClient = {
		readContract: vi.fn(async ({ functionName }: { functionName: string }) => {
			if (functionName === "decimals") return 6
			if (functionName === "balanceOf") return 10n ** 18n
			throw new Error(`unexpected read: ${functionName}`)
		}),
		waitForTransactionReceipt: vi.fn().mockResolvedValue({ status: "success" }),
	}
	const chainClientManager = {
		getAccount: () => ({ address: SOLVER }),
		getPublicClient: () => publicClient,
		getWalletClient: () => ({ writeContract, account: { address: SOLVER }, chain: {} }),
	}
	const configService = { getUsdcAsset: () => token, getUsdtAsset: () => "0x" as HexString }
	const rebalancer = new BinanceRebalancer(chainClientManager as never, configService as never, {
		apiKey: "key",
		apiSecret: "secret",
		pollIntervalMs: 1,
	})
	return { rebalancer, writeContract }
}

beforeEach(() => {
	vi.clearAllMocks()
	restAPI.allCoinsInformation.mockResolvedValue({ data: () => allCoins(USDC) })
	restAPI.depositAddress.mockResolvedValue({ data: () => ({ address: DEPOSIT_ADDRESS, coin: "USDC" }) })
	// Credit the deposit and complete the withdrawal immediately so the happy path needs no polling.
	restAPI.depositHistory.mockResolvedValue({ data: () => [{ txId: "0xdeposit", status: 1, confirmTimes: 15 }] })
	restAPI.withdraw.mockResolvedValue({ data: () => ({ id: "w1" }) })
	restAPI.withdrawHistory.mockResolvedValue({
		data: () => [{ id: "w1", status: 6, amount: "100", transactionFee: "1", txId: "0xwithdraw" }],
	})
})

describe("BinanceRebalancer deposit preflight", () => {
	it("refuses a deposit whose Binance contract is a different token", async () => {
		restAPI.allCoinsInformation.mockResolvedValue({
			data: () => allCoins("0x1111111111111111111111111111111111111111"),
		})
		const { rebalancer, writeContract } = makeRebalancer()

		await expect(rebalancer.sendViaCex(OPTIONS)).rejects.toThrow(/credits USDC on BSC/)
		expect(writeContract).not.toHaveBeenCalled()
	})

	it("refuses a deposit when Binance reports no contract address", async () => {
		restAPI.allCoinsInformation.mockResolvedValue({ data: () => allCoins(undefined) })
		const { rebalancer, writeContract } = makeRebalancer()

		await expect(rebalancer.sendViaCex(OPTIONS)).rejects.toThrow(/no contract address for USDC on BSC/)
		expect(writeContract).not.toHaveBeenCalled()
	})

	it("refuses a deposit address that carries a tag/memo an ERC-20 transfer cannot send", async () => {
		restAPI.depositAddress.mockResolvedValue({
			data: () => ({ address: DEPOSIT_ADDRESS, coin: "USDC", tag: "12345" }),
		})
		const { rebalancer, writeContract } = makeRebalancer()

		await expect(rebalancer.sendViaCex(OPTIONS)).rejects.toThrow(/requires a tag\/memo/)
		expect(writeContract).not.toHaveBeenCalled()
	})

	it("proceeds to the transfer when the contract matches", async () => {
		const { rebalancer, writeContract } = makeRebalancer()

		const result = await rebalancer.sendViaCex(OPTIONS)

		expect(result.success).toBe(true)
		expect(writeContract).toHaveBeenCalledOnce()
		const tx = writeContract.mock.calls[0][0]
		// The configured token, to Binance's deposit address.
		expect(tx.address.toLowerCase()).toBe(USDC.toLowerCase())
		expect(tx.args[0]).toBe(DEPOSIT_ADDRESS)
	})

	it("tolerates Binance omitting the 0x prefix on the contract address", async () => {
		restAPI.allCoinsInformation.mockResolvedValue({ data: () => allCoins(USDC.slice(2)) })
		const { rebalancer, writeContract } = makeRebalancer()

		await expect(rebalancer.sendViaCex(OPTIONS)).resolves.toMatchObject({ success: true })
		expect(writeContract).toHaveBeenCalledOnce()
	})

	it("matches a contract address across EIP-55 case differences", async () => {
		const checksummed = getAddress(USDC)

		// Binance checksummed, config lowercase.
		restAPI.allCoinsInformation.mockResolvedValue({ data: () => allCoins(checksummed) })
		const first = makeRebalancer()
		await expect(first.rebalancer.sendViaCex(OPTIONS)).resolves.toMatchObject({ success: true })
		expect(first.writeContract).toHaveBeenCalledOnce()

		// And the reverse: config checksummed, Binance lowercase.
		restAPI.allCoinsInformation.mockResolvedValue({ data: () => allCoins(USDC) })
		const second = makeRebalancer(checksummed)
		await expect(second.rebalancer.sendViaCex(OPTIONS)).resolves.toMatchObject({ success: true })
		expect(second.writeContract).toHaveBeenCalledOnce()
	})
})
