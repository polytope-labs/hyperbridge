import { IntentFiller } from "@/core/filler"
import {
	BidStorageService,
	CacheService,
	ChainClientManager,
	ContractInteractionService,
	FillerConfigService,
	type ResolvedChainConfig,
	type FillerConfig as FillerServiceConfig,
} from "@/services"
import { createSimplexSigner, SignerType } from "@/services/wallet"
import { FXFiller } from "@/strategies/fx"
import {
	type ChainConfig,
	type FillerConfig,
	type HexString,
	type Order,
	type TokenInfo,
	type SelectBidResult,
	bytes20ToBytes32,
	EvmChain,
	IntentGateway,
	IntentsCoprocessor,
	DEFAULT_GRAFFITI,
} from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { ConfirmationPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import {
	getContract,
	maxUint256,
	parseUnits,
	type PublicClient,
	type WalletClient,
	encodePacked,
	keccak256,
	toHex,
} from "viem"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { privateKeyToAccount } from "viem/accounts"
import "../setup"
import { pimlicoBundlerUrlForChain as bundlerUrl } from "../pimlicoBundler"
import { ERC20_ABI } from "@/config/abis/ERC20"

// ============================================================================
// Test Suites
// ============================================================================
//
// FX (cross-token) fill over the BSC Chapel -> Polygon Amoy path. The user
// pays USDC on BSC and receives an "exotic" token on Polygon; for testnet we
// use Polygon's USDC as the stand-in exotic (configured via FXFiller.token1).

describe("Filler V2 FX - USDC -> Exotic (BSC Chapel -> Polygon Amoy)", () => {
	it("Should place USDC->Exotic order on BSC, filler submits bid, user selects bid, order filled on Polygon", async () => {
		const {
			bscIntentGatewayV2,
			polygonAmoyPublicClient,
			bscPublicClient,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			bscChapelId,
			polygonAmoyId,
			bscWalletClient,
			contractService,
		} = await setUp()

		const intentFiller = await createFxIntentFiller(chainConfigs, fillerConfig, chainConfigService, polygonAmoyId)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(bscChapelId)
		// Treat Polygon's USDC as the exotic output token for this FX test.
		const destExotic = chainConfigService.getUsdcAsset(polygonAmoyId)

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, bscChapelId)
		const destExoticDecimals = await contractService.getTokenDecimals(destExotic, polygonAmoyId)
		const amount = parseUnits("0.1", sourceUsdcDecimals)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(destExotic),
				amount: parseUnits("0.006", destExoticDecimals),
			},
		]

		const privateKey = process.env.PRIVATE_KEY as HexString
		const beneficiaryAddress = privateKeyToAccount(privateKey).address
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)

		let order: Order = {
			user: bytes20ToBytes32(beneficiaryAddress),
			source: toHex(bscChapelId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: parseUnits("0.02", sourceUsdcDecimals),
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_GARGANTUA!,
			process.env.SECRET_PHRASE!,
		)

		const bscEvmChain = EvmChain.fromParams({
			chainId: 97,
			host: chainConfigService.getHostAddress(bscChapelId),
			rpcUrl: chainConfigService.getRpcUrl(bscChapelId),
		})

		const destBundlerUrl = chainConfigService.getBundlerUrl(polygonAmoyId)
		const polygonAmoyEvmChain = EvmChain.fromParams({
			chainId: 80002,
			host: chainConfigService.getHostAddress(polygonAmoyId),
			rpcUrl: chainConfigService.getRpcUrl(polygonAmoyId),
			bundlerUrl: destBundlerUrl,
		})

		const feeToken = await contractService.getFeeTokenWithDecimals(bscChapelId)
		await approveTokens(bscWalletClient, bscPublicClient, feeToken.address, bscIntentGatewayV2.address)
		await approveTokens(bscWalletClient, bscPublicClient, sourceUsdc, bscIntentGatewayV2.address)

		const userSdkHelper = await IntentGateway.create(bscEvmChain, polygonAmoyEvmChain, intentsCoprocessor)

		const gen = userSdkHelper.executeBest(order, DEFAULT_GRAFFITI, { auctionTimeMs: 15_000, pollIntervalMs: 5_000 })
		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data, value } = result.value
			const signedTx = (await bscWalletClient.signTransaction(
				(await bscPublicClient.prepareTransactionRequest({
					to,
					data,
					value: 0n,
					account: bscWalletClient.account!,
					chain: bscWalletClient.chain,
				})) as any,
			)) as HexString
			result = await gen.next(signedTx)
		}
		let userOpHash: HexString | undefined
		let selectedSolver: HexString | undefined
		while (!result.done) {
			if (result.value && "status" in result.value) {
				const status = result.value
				if (status.status === "BID_SELECTED") {
					selectedSolver = status.selectedSolver as HexString
					userOpHash = status.userOpHash as HexString
					if (status.transactionHash) {
						console.log("Transaction hash:", status.transactionHash)
					}
					// Cross-chain settles asynchronously via Hyperbridge — the executor
					// yields no terminal FILLED, so BID_SELECTED is terminal here. Close
					// the generator (stops its bid/deadline polling) and stop driving it.
					void gen.return(undefined).catch(() => {})
					break
				}
				if (status.status === "FAILED") {
					throw new Error(`Order execution failed: ${status.error}`)
				}
			}
			result = await gen.next()
		}
		expect(userOpHash).toBeDefined()
		expect(selectedSolver).toBeDefined()

		const isFilled = await pollForOrderFilled(
			order.id as HexString,
			polygonAmoyPublicClient,
			chainConfigService.getIntentGatewayAddress(polygonAmoyId),
		)
		expect(isFilled).toBe(true)

		await intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)

	it("Should yield Bid objects and let the consumer inspect, select, and execute a bid", async () => {
		const {
			bscIntentGatewayV2,
			polygonAmoyPublicClient,
			bscPublicClient,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			bscChapelId,
			polygonAmoyId,
			bscWalletClient,
			contractService,
		} = await setUp()

		const intentFiller = await createFxIntentFiller(chainConfigs, fillerConfig, chainConfigService, polygonAmoyId)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(bscChapelId)
		const destExotic = chainConfigService.getUsdcAsset(polygonAmoyId)

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, bscChapelId)
		const destExoticDecimals = await contractService.getTokenDecimals(destExotic, polygonAmoyId)
		const amount = parseUnits("0.1", sourceUsdcDecimals)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(destExotic),
				amount: parseUnits("0.006", destExoticDecimals),
			},
		]

		const privateKey = process.env.PRIVATE_KEY as HexString
		const beneficiaryAddress = privateKeyToAccount(privateKey).address
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)

		let order: Order = {
			user: bytes20ToBytes32(beneficiaryAddress),
			source: toHex(bscChapelId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 1n,
			fees: parseUnits("0.02", sourceUsdcDecimals),
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_GARGANTUA!,
			process.env.SECRET_PHRASE!,
		)

		const bscEvmChain = EvmChain.fromParams({
			chainId: 97,
			host: chainConfigService.getHostAddress(bscChapelId),
			rpcUrl: chainConfigService.getRpcUrl(bscChapelId),
		})

		const destBundlerUrl = chainConfigService.getBundlerUrl(polygonAmoyId)
		const polygonAmoyEvmChain = EvmChain.fromParams({
			chainId: 80002,
			host: chainConfigService.getHostAddress(polygonAmoyId),
			rpcUrl: chainConfigService.getRpcUrl(polygonAmoyId),
			bundlerUrl: destBundlerUrl,
		})

		const feeToken = await contractService.getFeeTokenWithDecimals(bscChapelId)
		await approveTokens(bscWalletClient, bscPublicClient, feeToken.address, bscIntentGatewayV2.address)
		await approveTokens(bscWalletClient, bscPublicClient, sourceUsdc, bscIntentGatewayV2.address)

		const userSdkHelper = await IntentGateway.create(bscEvmChain, polygonAmoyEvmChain, intentsCoprocessor)

		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, { auctionTimeMs: 15_000, pollIntervalMs: 5_000 })
		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data } = result.value
			const signedTx = (await bscWalletClient.signTransaction(
				(await bscPublicClient.prepareTransactionRequest({
					to,
					data,
					value: 0n,
					account: bscWalletClient.account!,
					chain: bscWalletClient.chain,
				})) as any,
			)) as HexString
			result = await gen.next(signedTx)
		}

		let userOpHash: HexString | undefined
		let selectedSolver: HexString | undefined
		let inspectedBid = false

		while (!result.done) {
			let feedback: SelectBidResult | undefined
			if (result.value && "status" in result.value) {
				const status = result.value

				if (status.status === "BIDS_RECEIVED") {
					expect(status.bids.length).toBeGreaterThan(0)

					const ranked = await userSdkHelper.sortBids(order, status.bids)
					expect(ranked.length).toBeGreaterThan(0)
					const chosen = ranked[0]

					expect(chosen.solverAddress).toMatch(/^0x[0-9a-fA-F]{40}$/)
					expect(chosen.outputs.length).toBeGreaterThan(0)
					expect(typeof chosen.relayerFee).toBe("bigint")
					expect(typeof chosen.nativeDispatchFee).toBe("bigint")
					const outputUsd = await chosen.outputUsdValue()
					console.log("Consumer-selected bid:", {
						solver: chosen.solverAddress,
						outputs: chosen.outputs.length,
						relayerFee: chosen.relayerFee.toString(),
						outputUsd: outputUsd?.toString() ?? "n/a",
					})
					inspectedBid = true

					await chosen.simulate()
					feedback = await chosen.execute()
				}

				if (status.status === "BID_SELECTED") {
					selectedSolver = status.selectedSolver as HexString
					userOpHash = status.userOpHash as HexString
					if (status.transactionHash) {
						console.log("Transaction hash:", status.transactionHash)
					}

					void gen.return(undefined).catch(() => {})
					break
				}

				if (status.status === "FAILED") {
					throw new Error(`Order execution failed: ${status.error}`)
				}
			}
			result = await gen.next(feedback)
		}

		expect(inspectedBid).toBe(true)
		expect(userOpHash).toBeDefined()
		expect(selectedSolver).toBeDefined()

		const isFilled = await pollForOrderFilled(
			order.id as HexString,
			polygonAmoyPublicClient,
			chainConfigService.getIntentGatewayAddress(polygonAmoyId),
		)
		expect(isFilled).toBe(true)

		await intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)
})

// ============================================================================
// Shared Helpers
// ============================================================================

async function createFxIntentFiller(
	chainConfigs: ChainConfig[],
	fillerConfig: FillerConfig,
	chainConfigService: FillerConfigService,
	exoticChainId: string,
): Promise<IntentFiller> {
	const privateKey = process.env.PRIVATE_KEY as HexString
	const signer = await createSimplexSigner({ type: SignerType.PrivateKey, key: privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)
	const contractService = new ContractInteractionService(chainClientManager, chainConfigService, signer, cacheService)

	// Exotic ≈ $1 (Polygon USDC stand-in), so price is 1 exotic token per USD.
	const bidPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: "1" },
			{ amount: "10000", price: "1" },
		],
	})
	const askPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: "1" },
			{ amount: "10000", price: "1" },
		],
	})

	const confirmationPolicy = new ConfirmationPolicy({
		"97": {
			points: [
				{ amount: "1", value: 1 },
				{ amount: "1000", value: 5 },
			],
		},
		"80002": {
			points: [
				{ amount: "1", value: 1 },
				{ amount: "1000", value: 5 },
			],
		},
	})

	const token1: Record<string, HexString> = {
		[exoticChainId]: chainConfigService.getUsdcAsset(exoticChainId),
	}

	const strategies = [
		new FXFiller(signer, chainConfigService, chainClientManager, contractService, 5000, token1, {
			bidPricePolicy,
			askPricePolicy,
			confirmationPolicy,
		}),
	]

	const bidStorage = new BidStorageService(chainConfigService.getDataDir())

	return new IntentFiller(
		chainConfigs,
		strategies,
		fillerConfig,
		chainConfigService,
		chainClientManager,
		contractService,
		signer,
		undefined,
		bidStorage,
	)
}

async function pollForOrderFilled(
	orderId: HexString,
	publicClient: PublicClient,
	intentGatewayAddress: HexString,
	maxAttempts = 60,
): Promise<boolean> {
	for (let i = 0; i < maxAttempts; i++) {
		const filled = await checkIfOrderFilled(orderId, publicClient, intentGatewayAddress)
		if (filled) {
			console.log("Order filled!")
			return true
		}
		await new Promise((resolve) => setTimeout(resolve, 5_000))
	}
	return false
}

// ============================================================================
// EVM Setup
// ============================================================================

async function setUp() {
	const bscChapelId = "EVM-97"
	const polygonAmoyId = "EVM-80002"
	const chains = [bscChapelId, polygonAmoyId]

	const testChainConfigs: ResolvedChainConfig[] = [
		{ chainId: 97, rpcUrls: [process.env.BSC_CHAPEL!], bundlerUrl: bundlerUrl(97) },
		{ chainId: 80002, rpcUrls: [process.env.POLYGON_AMOY!], bundlerUrl: bundlerUrl(80002) },
	]

	const fillerConfigForService: FillerServiceConfig = {
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_GARGANTUA,
		substratePrivateKey: process.env.SECRET_PHRASE,
	}

	const chainConfigService = new FillerConfigService(testChainConfigs, fillerConfigForService)
	const chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const fillerConfig: FillerConfig = {
		maxConcurrentOrders: 5,
		pendingQueueConfig: {
			maxRechecks: 10,
			recheckDelayMs: 30_000,
		},
	}

	const privateKey = process.env.PRIVATE_KEY as HexString
	const signer = await createSimplexSigner({ type: SignerType.PrivateKey, key: privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)
	const contractService = new ContractInteractionService(chainClientManager, chainConfigService, signer, cacheService)

	const bscWalletClient = chainClientManager.getWalletClient(bscChapelId)
	const bscPublicClient = chainClientManager.getPublicClient(bscChapelId)
	const polygonAmoyPublicClient = chainClientManager.getPublicClient(polygonAmoyId)

	const bscIntentGatewayV2 = getContract({
		address: chainConfigService.getIntentGatewayAddress(bscChapelId),
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	return {
		bscWalletClient,
		bscPublicClient,
		polygonAmoyPublicClient,
		bscIntentGatewayV2,
		contractService,
		bscChapelId,
		polygonAmoyId,
		chainConfigService,
		fillerConfig,
		chainConfigs,
	}
}

// ============================================================================
// Token Approval Helpers
// ============================================================================

async function approveTokens(
	walletClient: WalletClient,
	publicClient: PublicClient,
	tokenAddress: HexString,
	spender: HexString,
) {
	const approval = await publicClient.readContract({
		abi: ERC20_ABI,
		address: tokenAddress,
		functionName: "allowance",
		args: [walletClient.account?.address as HexString, spender],
		account: walletClient.account,
	})

	if (approval === 0n) {
		console.log(`Approving token ${tokenAddress} for ${spender}`)
		const tx = await walletClient.writeContract({
			abi: ERC20_ABI,
			address: tokenAddress,
			functionName: "approve",
			args: [spender, maxUint256],
			chain: walletClient.chain,
			account: walletClient.account!,
		})
		await publicClient.waitForTransactionReceipt({ hash: tx, confirmations: 1 })
	}
}

// ============================================================================
// Order Status Helpers
// ============================================================================

async function checkIfOrderFilled(
	commitment: HexString,
	client: PublicClient,
	intentGatewayV2Address: HexString,
): Promise<boolean> {
	try {
		const mappingSlot = 2n
		const slot = keccak256(encodePacked(["bytes32", "uint256"], [commitment, mappingSlot]))
		const filledStatus = await client.getStorageAt({
			address: intentGatewayV2Address,
			slot,
		})
		return filledStatus !== "0x0000000000000000000000000000000000000000000000000000000000000000"
	} catch (error) {
		console.error("Error checking if order filled:", error)
		return false
	}
}
