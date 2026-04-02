import { describe, it, expect, afterEach, vi } from "vitest"


import { IntentFiller } from "@/core/filler"
import { FXFiller } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { LimitOrderStorageService } from "@/services/LimitOrderStorageService"
import { CacheService } from "@/services/CacheService"
import { orderCommitment, bytes20ToBytes32, type Order, type HexString } from "@hyperbridge/sdk"
import { Decimal } from "decimal.js"
import { mkdtempSync, rmSync } from "fs"
import { join } from "path"
import { tmpdir } from "os"

// ── Addresses ────────────────────────────────────────────────────────────────

const USDC_ADDRESS = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48" as HexString
const USDT_ADDRESS = "0xdac17f958d2ee523a2206206994597c13d831ec7" as HexString
const EXOTIC_ADDRESS = "0x0000000000000000000000000000000000001234" as HexString
const FILLER_ADDRESS = "0x000000000000000000000000000000000000abcd" as HexString
const CHAIN = "EVM-1"
const CHAIN_ID = 1

// ── Test Order ───────────────────────────────────────────────────────────────

function createTestOrder(): Order {
	const order: Order = {
		user: bytes20ToBytes32("0x0000000000000000000000000000000000000001"),
		source: CHAIN as HexString,
		destination: CHAIN as HexString,
		deadline: 99_999_999n, // Far-future block
		nonce: 1n,
		fees: 5_000n, // 0.005 USDC (6 decimals)
		session: "0x0000000000000000000000000000000000000000" as HexString,
		predispatch: { assets: [], call: "0x" as HexString },
		inputs: [
			{
				token: bytes20ToBytes32(USDC_ADDRESS),
				amount: 1_000_000n, // 1 USDC (6 decimals)
			},
		],
		output: {
			beneficiary: bytes20ToBytes32("0x0000000000000000000000000000000000000002"),
			assets: [
				{
					token: bytes20ToBytes32(EXOTIC_ADDRESS),
					amount: 2_000_000_000_000_000_000n, // 2 exotic (18 decimals)
				},
			],
			call: "0x" as HexString,
		},
	}

	order.id = orderCommitment(order)
	return order
}

// ── Mock Factories ───────────────────────────────────────────────────────────

function createMockPublicClient() {
	return {
		getBlockNumber: vi.fn().mockResolvedValue(1_000n),
		getBlock: vi.fn().mockResolvedValue({ number: 1_000n, timestamp: 1_700_000_000n }),
		getBalance: vi.fn().mockResolvedValue(0n),
		getTransactionConfirmations: vi.fn().mockResolvedValue(100n),
		readContract: vi.fn().mockImplementation(async ({ functionName }: { functionName: string }) => {
			if (functionName === "balanceOf") {
				// Filler has plenty of exotic tokens
				return 100_000_000_000_000_000_000n // 100 exotic (18 decimals)
			}
			if (functionName === "decimals") {
				return 18
			}
			return 0n
		}),
		chain: { blockTime: 12_000 },
	}
}

function createMockChainClientManager(publicClient: ReturnType<typeof createMockPublicClient>) {
	return {
		getPublicClient: vi.fn().mockReturnValue(publicClient),
		getWalletClient: vi.fn().mockReturnValue({}),
		getSigner: vi.fn().mockReturnValue({ account: { address: FILLER_ADDRESS } }),
	}
}

function createMockConfigService() {
	return {
		getUsdcAsset: vi.fn().mockReturnValue(USDC_ADDRESS),
		getUsdtAsset: vi.fn().mockReturnValue(USDT_ADDRESS),
		getIntentGatewayV2Address: vi.fn().mockReturnValue("0x0000000000000000000000000000000000000000"),
		getRpcUrl: vi.fn().mockReturnValue("http://localhost:8545"),
		getHostAddress: vi.fn().mockReturnValue("0x0000000000000000000000000000000000000000"),
		getConsensusStateId: vi.fn().mockReturnValue("ETH0"),
		getHyperbridgeWsUrl: vi.fn().mockReturnValue(""),
		getSubstratePrivateKey: vi.fn().mockReturnValue(""),
		getConfiguredChainIds: vi.fn().mockReturnValue([CHAIN_ID]),
		getChainConfig: vi.fn().mockReturnValue({ chainId: CHAIN_ID }),
		getDataDir: vi.fn().mockReturnValue(undefined),
		getEntryPointAddress: vi.fn().mockReturnValue("0x0000000000000000000000000000000000000000"),
		getTargetGasUnits: vi.fn().mockReturnValue(3_000_000),
		getRebalancingConfig: vi.fn().mockReturnValue(undefined),
	}
}

function createMockContractService(cacheService: CacheService) {
	return {
		cacheService,
		getCache: vi.fn().mockReturnValue(cacheService),
		getFeeTokenWithDecimals: vi.fn().mockResolvedValue({
			token: USDC_ADDRESS,
			decimals: 6,
		}),
		getTokenDecimals: vi.fn().mockImplementation(async (token: string) => {
			const addr = token.toLowerCase()
			if (addr === USDC_ADDRESS.toLowerCase() || addr === USDT_ADDRESS.toLowerCase()) return 6
			return 18
		}),
		estimateGasFillPost: vi.fn().mockResolvedValue({
			totalCostInSourceFeeToken: 1_000n, // Low gas cost
		}),
		getInputUsdValue: vi.fn().mockResolvedValue(new Decimal(1)), // 1 USD
		isSolverSelectionActive: vi.fn().mockResolvedValue(false),
		prepareBidUserOp: vi.fn().mockResolvedValue({
			commitment: "0x1234",
			userOp: {},
		}),
	}
}

function createMockSigner() {
	return {
		account: { address: FILLER_ADDRESS },
		signTypedData: vi.fn(),
	}
}

function createFXFiller(
	mockConfigService: ReturnType<typeof createMockConfigService>,
	mockChainClientManager: ReturnType<typeof createMockChainClientManager>,
	mockContractService: ReturnType<typeof createMockContractService>,
	mockSigner: ReturnType<typeof createMockSigner>,
	askPrice: string,
	bidPrice: string,
) {
	return new FXFiller(
		mockSigner as any,
		mockConfigService as any,
		mockChainClientManager as any,
		mockContractService as any,
		5000, // maxOrderUsd
		{ [CHAIN]: EXOTIC_ADDRESS },
		{
			bidPricePolicy: new FillerPricePolicy({
				points: [
					{ amount: "1", price: bidPrice },
					{ amount: "10000", price: bidPrice },
				],
			}),
			askPricePolicy: new FillerPricePolicy({
				points: [
					{ amount: "1", price: askPrice },
					{ amount: "10000", price: askPrice },
				],
			}),
		},
	)
}

// ── Tests ────────────────────────────────────────────────────────────────────

describe("Limit Orders", () => {
	let tmpDir: string
	let limitOrderStorage: LimitOrderStorageService

	afterEach(() => {
		try {
			limitOrderStorage?.close()
		} catch {}
		try {
			rmSync(tmpDir, { recursive: true, force: true })
		} catch {}
	})

	it("stores unprofitable orders and executes them when rates improve", async () => {
		tmpDir = mkdtempSync(join(tmpdir(), "simplex-limit-test-"))
		limitOrderStorage = new LimitOrderStorageService(tmpDir)

		const cacheService = new CacheService()
		cacheService.setSolverSelection(CHAIN, false)

		const publicClient = createMockPublicClient()
		const mockChainClientManager = createMockChainClientManager(publicClient)
		const mockConfigService = createMockConfigService()
		const mockContractService = createMockContractService(cacheService)
		const mockSigner = createMockSigner()

		const order = createTestOrder()

		// ── Phase 1: Unfavorable prices ──────────────────────────────────
		// Ask price = 1 exotic/USD → filler would output 1 exotic for 1 USDC
		// But user wants 2 exotic → unprofitable → stored as limit order
		const unprofitableFx = createFXFiller(
			mockConfigService,
			mockChainClientManager,
			mockContractService,
			mockSigner,
			"1",  // askPrice: 1 exotic per USD (too low)
			"1",  // bidPrice
		)

		const filler1 = new IntentFiller(
			[{ chainId: CHAIN_ID } as any],
			[unprofitableFx],
			{ maxConcurrentOrders: 5 } as any,
			mockConfigService as any,
			mockChainClientManager as any,
			mockContractService as any,
			mockSigner as any,
			undefined, // rebalancingService
			undefined, // bidStorage
			limitOrderStorage,
		)

		// Emit order directly on the monitor (don't call start() to avoid block scanning)
		filler1.monitor.emit("newOrder", { order, transactionHash: "0xdeadbeef" })

		// Wait for the global queue to process
		await vi.waitFor(
			() => {
				const pending = limitOrderStorage.getPendingLimitOrders()
				expect(pending).toHaveLength(1)
				expect(pending[0].orderId).toBe(order.id)
			},
			{ timeout: 5_000, interval: 100 },
		)

		// Verify the stored order can be deserialized correctly
		const stored = limitOrderStorage.getPendingLimitOrders()[0]
		const deserialized = limitOrderStorage.deserializeOrder(stored.orderJson)
		expect(deserialized.deadline).toBe(order.deadline)
		expect(deserialized.inputs[0].amount).toBe(order.inputs[0].amount)
		expect(deserialized.output.assets[0].amount).toBe(order.output.assets[0].amount)

		await filler1.stop()

		// ── Phase 2: Favorable prices ────────────────────────────────────
		// Ask price = 3 exotic/USD → filler would output 3 exotic for 1 USDC
		// User wants 2 exotic → profitable → should be picked up by sweep
		const profitableFx = createFXFiller(
			mockConfigService,
			mockChainClientManager,
			mockContractService,
			mockSigner,
			"3",  // askPrice: 3 exotic per USD (profitable)
			"3",  // bidPrice
		)

		const filler2 = new IntentFiller(
			[{ chainId: CHAIN_ID } as any],
			[profitableFx],
			{ maxConcurrentOrders: 5 } as any,
			mockConfigService as any,
			mockChainClientManager as any,
			mockContractService as any,
			mockSigner as any,
			undefined,
			undefined,
			limitOrderStorage,
		)

		// Trigger the sweep directly (instead of waiting 60s for the interval)
		await (filler2 as any).sweepLimitOrders()

		// The limit order should have been deleted (executed)
		const remaining = limitOrderStorage.getPendingLimitOrders()
		expect(remaining).toHaveLength(0)

		await filler2.stop()
	})

	it("expires limit orders when the deadline block has passed", async () => {
		tmpDir = mkdtempSync(join(tmpdir(), "simplex-limit-test-"))
		limitOrderStorage = new LimitOrderStorageService(tmpDir)

		const cacheService = new CacheService()
		cacheService.setSolverSelection(CHAIN, false)

		const publicClient = createMockPublicClient()
		const mockChainClientManager = createMockChainClientManager(publicClient)
		const mockConfigService = createMockConfigService()
		const mockContractService = createMockContractService(cacheService)
		const mockSigner = createMockSigner()

		// Create order with a low deadline
		const order = createTestOrder()
		order.deadline = 500n // Already passed (current block is 1000)
		order.id = orderCommitment(order)

		const unprofitableFx = createFXFiller(
			mockConfigService,
			mockChainClientManager,
			mockContractService,
			mockSigner,
			"1",
			"1",
		)

		// Store the order directly as a limit order
		limitOrderStorage.storeLimitOrder(order, "FXFiller")
		expect(limitOrderStorage.getPendingLimitOrders()).toHaveLength(1)

		const filler = new IntentFiller(
			[{ chainId: CHAIN_ID } as any],
			[unprofitableFx],
			{ maxConcurrentOrders: 5 } as any,
			mockConfigService as any,
			mockChainClientManager as any,
			mockContractService as any,
			mockSigner as any,
			undefined,
			undefined,
			limitOrderStorage,
		)

		// Trigger sweep — should expire the order since current block (1000) >= deadline (500)
		await (filler as any).sweepLimitOrders()

		expect(limitOrderStorage.getPendingLimitOrders()).toHaveLength(0)

		await filler.stop()
	})
})
