import { IntentFiller } from "@/core/filler"
import {
	CacheService,
	ChainClientManager,
	ContractInteractionService,
	FillerConfigService,
	type UserProvidedChainConfig,
	type FillerConfig as FillerServiceConfig,
} from "@/services"
import { BasicFiller } from "@/strategies/basic"
import {
	type ChainConfig,
	type FillerConfig,
	type HexString,
	type OrderV2,
	type TokenInfoV2,
	bytes20ToBytes32,
	orderV2Commitment,
	EvmChain,
	IntentGatewayV2,
	IntentsCoprocessor,
} from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { ConfirmationPolicy } from "@/config/confirmation-policy"
import {
	getContract,
	maxUint256,
	parseUnits,
	type PublicClient,
	type WalletClient,
	encodePacked,
	keccak256,
	toHex,
	parseEventLogs,
} from "viem"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet } from "viem/chains"
import "./setup"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { Decimal } from "decimal.js"

describe.sequential("Filler V2 - Solver Selection ON", () => {
	it("Should place order, filler submits bid, user selects bid, order filled", async () => {
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

		const privateKey = process.env.PRIVATE_KEY as HexString
		const sharedCacheService = new CacheService()
		const chainClientManager = new ChainClientManager(chainConfigService, privateKey)
		const localContractService = new ContractInteractionService(
			chainClientManager,
			privateKey,
			chainConfigService,
			sharedCacheService,
		)

		const strategies = [
			new BasicFiller(privateKey, chainConfigService, chainClientManager, localContractService, 50), // 50 bps = 0.5%
		]

		const intentFiller = new IntentFiller(
			chainConfigs,
			strategies,
			fillerConfig,
			chainConfigService,
			chainClientManager,
			localContractService,
			privateKey,
		)

		// Initialize and start the filler
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(bscChapelId)
		const destUsdc = chainConfigService.getUsdcAsset(polygonAmoyId)

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, bscChapelId)
		const destUsdcDecimals = await contractService.getTokenDecimals(destUsdc, polygonAmoyId)
		const amount = parseUnits("1", sourceUsdcDecimals)

		const inputs: TokenInfoV2[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfoV2[] = [
			{
				token: bytes20ToBytes32(destUsdc),
				amount: amount - parseUnits("0.94", destUsdcDecimals),
			},
		]

		const beneficiaryAddress = privateKeyToAccount(privateKey).address
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)

		let order: OrderV2 = {
			user: bytes20ToBytes32(beneficiaryAddress),
			source: toHex(bscChapelId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: parseUnits("1", 18),
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		// Create SDK helper with IntentsCoprocessor and bundler URL for full solver selection flow
		const hyperbridgeWsUrl = process.env.HYPERBRIDGE_GARGANTUA!
		const substrateKey = process.env.SECRET_PHRASE!

		// Note: The bundler url is like 'https://api.pimlico.io/v2/80002/rpc?apikey=YOUR_KEY'
		// Which includes the chainID, we need to replace the chainID with the actual chainID in our final submission.
		const bundlerUrl = process.env.BUNDLER_URL

		const intentsCoprocessor = await IntentsCoprocessor.connect(hyperbridgeWsUrl, substrateKey)

		const bscEvmChain = new EvmChain({
			chainId: 97,
			host: chainConfigService.getHostAddress(bscChapelId),
			rpcUrl: chainConfigService.getRpcUrl(bscChapelId),
		})

		const polygonAmoyEvmChain = new EvmChain({
			chainId: 80002,
			host: chainConfigService.getHostAddress(polygonAmoyId),
			rpcUrl: chainConfigService.getRpcUrl(polygonAmoyId),
		})

		const feeToken = await contractService.getFeeTokenWithDecimals(bscChapelId)
		await approveTokens(bscWalletClient, bscPublicClient, feeToken.address, bscIntentGatewayV2.address)
		await approveTokens(bscWalletClient, bscPublicClient, sourceUsdc, bscIntentGatewayV2.address)

		const userSdkHelper = new IntentGatewayV2(bscEvmChain, polygonAmoyEvmChain, intentsCoprocessor, bundlerUrl)

		const generator = userSdkHelper.preparePlaceOrder(order)

		const firstResult = await generator.next()
		const { calldata, sessionPrivateKey } = firstResult.value as {
			calldata: HexString
			sessionPrivateKey: HexString
		}

		const txHash = await bscWalletClient.sendTransaction({
			to: bscIntentGatewayV2.address,
			data: calldata,
			account: bscWalletClient.account!,
			chain: bscWalletClient.chain,
		})

		await bscPublicClient.waitForTransactionReceipt({ hash: txHash, confirmations: 1 })

		const secondResult = await generator.next(txHash)
		order = secondResult.value as OrderV2

		console.log("Starting executeIntentOrder flow (waiting for bids from filler)...")

		let userOpHash: HexString | undefined
		let selectedSolver: HexString | undefined

		for await (const status of userSdkHelper.executeIntentOrder({
			order,
			sessionPrivateKey,
			minBids: 1, // Wait for at least 1 bid
			bidTimeoutMs: 120_000, // 2 minute to wait for bids
			pollIntervalMs: 5_000, // Poll every 5 seconds
		})) {
			console.log(`Status: ${status.status}`, status.metadata)

			switch (status.status) {
				case "AWAITING_BIDS":
					console.log("Waiting for filler bids on Hyperbridge...")
					break
				case "BIDS_RECEIVED":
					console.log(`Received ${status.metadata.bidCount} bid(s)`)
					break
				case "BID_SELECTED":
					selectedSolver = status.metadata.selectedSolver as HexString
					userOpHash = status.metadata.userOpHash as HexString
					console.log(`Selected solver: ${selectedSolver}`)
					break
				case "USEROP_SUBMITTED":
					console.log(`UserOp submitted to bundler, transaction hash: ${status.metadata.transactionHash}`)
					break
				case "FAILED":
					throw new Error(`Order execution failed: ${status.metadata.error}`)
			}
		}

		expect(userOpHash).toBeDefined()
		expect(selectedSolver).toBeDefined()

		console.log("Waiting for order to be filled on Polygon Amoy...")

		// Poll for filled status (the bundler executes the UserOp which calls fillOrder)
		let isFilled = false
		const maxAttempts = 60 // 5 minutes with 5s intervals
		for (let i = 0; i < maxAttempts; i++) {
			isFilled = await checkIfOrderFilled(
				order.id as HexString,
				polygonAmoyPublicClient,
				chainConfigService.getIntentGatewayV2Address(polygonAmoyId),
			)
			if (isFilled) {
				console.log("Order filled on Polygon Amoy!")
				break
			}
			await new Promise((resolve) => setTimeout(resolve, 5000))
		}

		expect(isFilled).toBe(true)

		// Cleanup
		intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)
})

// ============================================================================
// Setup and Helper Functions
// ============================================================================

async function setUp() {
	const bscChapelId = "EVM-97"
	const polygonAmoyId = "EVM-80002"

	const chains = [bscChapelId, polygonAmoyId]

	// Create chain configurations
	const testChainConfigs: UserProvidedChainConfig[] = [
		{
			chainId: 97, // BSC Chapel (source chain)
			rpcUrl: process.env.BSC_CHAPEL!,
		},
		{
			chainId: 80002, // Polygon Amoy (destination chain)
			rpcUrl: process.env.POLYGON_AMOY!,
		},
	]

	// Filler service config with Hyperbridge support for solver selection
	const fillerConfigForService: FillerServiceConfig = {
		privateKey: process.env.PRIVATE_KEY as HexString,
		maxConcurrentOrders: 5,
		// Hyperbridge configuration for solver support
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_GARGANTUA,
		substratePrivateKey: process.env.SECRET_PHRASE, // Substrate mnemonic
		solverAccountContractAddress: "0xCDFcFeD7A14154846808FddC8Ba971A2f8a830a3",
	}

	const chainConfigService = new FillerConfigService(testChainConfigs, fillerConfigForService)
	const chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	// Create confirmation policy
	const confirmationPolicyConfig = {
		"97": {
			minAmount: "1",
			maxAmount: "1000",
			minConfirmations: 1,
			maxConfirmations: 5,
		},
		"80002": {
			minAmount: "1",
			maxAmount: "1000",
			minConfirmations: 1,
			maxConfirmations: 5,
		},
	}

	const confirmationPolicy = new ConfirmationPolicy(confirmationPolicyConfig)

	const fillerConfig: FillerConfig = {
		confirmationPolicy: {
			getConfirmationBlocks: (chainId: number, amountUsd: number) =>
				confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
		},
		maxConcurrentOrders: 5,
		pendingQueueConfig: {
			maxRechecks: 10,
			recheckDelayMs: 30000,
		},
	}

	// Create shared services
	const privateKey = process.env.PRIVATE_KEY as HexString
	const sharedCacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, privateKey)
	const contractService = new ContractInteractionService(
		chainClientManager,
		privateKey,
		chainConfigService,
		sharedCacheService,
	)

	// Get clients
	const bscWalletClient = chainClientManager.getWalletClient(bscChapelId)
	const polygonAmoyWalletClient = chainClientManager.getWalletClient(polygonAmoyId)
	const bscPublicClient = chainClientManager.getPublicClient(bscChapelId)
	const polygonAmoyPublicClient = chainClientManager.getPublicClient(polygonAmoyId)

	// Get contract addresses
	const bscIntentGatewayV2Address = chainConfigService.getIntentGatewayV2Address(bscChapelId)
	const polygonAmoyIntentGatewayV2Address = chainConfigService.getIntentGatewayV2Address(polygonAmoyId)
	const bscIsmpHostAddress = chainConfigService.getHostAddress(bscChapelId)
	const polygonAmoyIsmpHostAddress = chainConfigService.getHostAddress(polygonAmoyId)

	// Create contract instances
	const bscIntentGatewayV2 = getContract({
		address: bscIntentGatewayV2Address,
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const polygonAmoyIntentGatewayV2 = getContract({
		address: polygonAmoyIntentGatewayV2Address,
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: polygonAmoyPublicClient, wallet: polygonAmoyWalletClient },
	})

	// Create SDK helper for preparing orders (source -> destination)
	const bscEvmChain = new EvmChain({
		chainId: 97,
		host: bscIsmpHostAddress,
		rpcUrl: chainConfigService.getRpcUrl(bscChapelId),
	})

	const polygonAmoyEvmChain = new EvmChain({
		chainId: 80002,
		host: polygonAmoyIsmpHostAddress,
		rpcUrl: chainConfigService.getRpcUrl(polygonAmoyId),
	})

	// IntentGatewayV2 helper: source (BSC Chapel) -> dest (Polygon Amoy)
	const intentGatewayHelper = new IntentGatewayV2(bscEvmChain, polygonAmoyEvmChain)

	const bscIsmpHost = getContract({
		address: bscIsmpHostAddress,
		abi: EVM_HOST,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const polygonAmoyIsmpHost = getContract({
		address: polygonAmoyIsmpHostAddress,
		abi: EVM_HOST,
		client: { public: polygonAmoyPublicClient, wallet: polygonAmoyWalletClient },
	})

	return {
		chainClientManager,
		bscWalletClient,
		polygonAmoyWalletClient,
		bscPublicClient,
		polygonAmoyPublicClient,
		bscIntentGatewayV2,
		polygonAmoyIntentGatewayV2,
		bscIsmpHostAddress,
		polygonAmoyIsmpHostAddress,
		bscIsmpHost,
		polygonAmoyIsmpHost,
		contractService,
		bscChapelId,
		polygonAmoyId,
		chainConfigService,
		fillerConfig,
		chainConfigs,
		intentGatewayHelper,
	}
}

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

		console.log("Approval tx:", tx)
		await publicClient.waitForTransactionReceipt({ hash: tx, confirmations: 1 })
		console.log("Token approved")
	}
}

async function checkIfOrderFilled(
	commitment: HexString,
	client: PublicClient,
	intentGatewayV2Address: HexString,
): Promise<boolean> {
	try {
		// The filled mapping is at storage slot 2 in IntentGatewayV2 contract
		const mappingSlot = 2n

		const slot = keccak256(encodePacked(["bytes32", "uint256"], [commitment, mappingSlot]))

		const filledStatus = await client.getStorageAt({
			address: intentGatewayV2Address,
			slot: slot,
		})

		console.log("Filled status:", filledStatus)
		return filledStatus !== "0x0000000000000000000000000000000000000000000000000000000000000000"
	} catch (error) {
		console.error("Error checking if order filled:", error)
		return false
	}
}
