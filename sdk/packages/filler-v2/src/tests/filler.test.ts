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
	EvmChain,
	IntentGatewayV2,
	IntentsCoprocessor,
} from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { ConfirmationPolicy, FillerBpsPolicy } from "@/config/interpolated-curve"
import {
	getContract,
	maxUint256,
	parseUnits,
	type PublicClient,
	type WalletClient,
	encodePacked,
	keccak256,
	toHex,
	decodeFunctionData,
} from "viem"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { privateKeyToAccount } from "viem/accounts"
import "./setup"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { Decimal } from "decimal.js"
import { TronWeb } from "tronweb"

// ============================================================================
// Test Suites
// ============================================================================

describe("Filler V2 - Solver Selection ON", () => {
	it.skip("Should place order, filler submits bid, user selects bid, order filled", async () => {
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

		const intentFiller = createIntentFiller(chainConfigs, fillerConfig, chainConfigService)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(bscChapelId)
		const destUsdc = chainConfigService.getUsdcAsset(polygonAmoyId)

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, bscChapelId)
		const destUsdcDecimals = await contractService.getTokenDecimals(destUsdc, polygonAmoyId)
		const amount = parseUnits("0.1", sourceUsdcDecimals)

		const inputs: TokenInfoV2[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfoV2[] = [
			{
				token: bytes20ToBytes32(destUsdc),
				amount: amount - parseUnits("0.094", destUsdcDecimals),
			},
		]

		const privateKey = process.env.PRIVATE_KEY as HexString
		const beneficiaryAddress = privateKeyToAccount(privateKey).address
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)

		let order: OrderV2 = {
			user: bytes20ToBytes32(beneficiaryAddress),
			source: toHex(bscChapelId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: parseUnits("0.005", sourceUsdcDecimals),
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_GARGANTUA!,
			process.env.SECRET_PHRASE!,
		)

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

		const bundlerUrl = process.env.BUNDLER_URL
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

		const { userOpHash, selectedSolver } = await executeOrderFlow(userSdkHelper, order, sessionPrivateKey)
		expect(userOpHash).toBeDefined()
		expect(selectedSolver).toBeDefined()

		const isFilled = await pollForOrderFilled(
			order.id as HexString,
			polygonAmoyPublicClient,
			chainConfigService.getIntentGatewayV2Address(polygonAmoyId),
		)
		expect(isFilled).toBe(true)

		intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)

	it("Should place order only", async () => {
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

		const sourceUsdc = chainConfigService.getUsdcAsset(bscChapelId)
		const destUsdc = chainConfigService.getUsdcAsset(polygonAmoyId)

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, bscChapelId)
		const destUsdcDecimals = await contractService.getTokenDecimals(destUsdc, polygonAmoyId)
		const amount = parseUnits("0.1", sourceUsdcDecimals)

		const inputs: TokenInfoV2[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfoV2[] = [
			{
				token: bytes20ToBytes32(destUsdc),
				amount: amount - parseUnits("0.094", destUsdcDecimals),
			},
		]

		const privateKey = process.env.PRIVATE_KEY as HexString
		const beneficiaryAddress = privateKeyToAccount(privateKey).address
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)

		let order: OrderV2 = {
			user: bytes20ToBytes32(beneficiaryAddress),
			source: toHex(bscChapelId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: parseUnits("0.005", sourceUsdcDecimals),
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_GARGANTUA!,
			process.env.SECRET_PHRASE!,
		)

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

		const bundlerUrl = process.env.BUNDLER_URL
		const userSdkHelper = new IntentGatewayV2(bscEvmChain, polygonAmoyEvmChain, intentsCoprocessor, bundlerUrl)

		console.log("Preparing to place order...")
		const generator = userSdkHelper.preparePlaceOrder(order)
		const firstResult = await generator.next()
		const { calldata, sessionPrivateKey } = firstResult.value as {
			calldata: HexString
			sessionPrivateKey: HexString
		}

		console.log("Sending place order transaction...")
		const txHash = await bscWalletClient.sendTransaction({
			to: bscIntentGatewayV2.address,
			data: calldata,
			account: bscWalletClient.account!,
			chain: bscWalletClient.chain,
		})

		console.log(`Transaction sent: ${txHash}`)
		await bscPublicClient.waitForTransactionReceipt({ hash: txHash, confirmations: 1 })

		console.log("Transaction confirmed, getting order details...")
		const secondResult = await generator.next(txHash)
		order = secondResult.value as OrderV2

		console.log(`Order placed successfully with ID: ${order.id}`)

		// Verify the order was placed
		expect(order.id).toBeDefined()
		expect(order.user).toBe(bytes20ToBytes32(beneficiaryAddress))
		expect(order.source).toBe(toHex(bscChapelId))
		expect(order.destination).toBe(toHex(polygonAmoyId))

		await intentsCoprocessor.disconnect()
	}, 300_000)
})

describe("Filler V2 - Tron Source Chain", () => {
	it.skip("Should place order on Tron Nile, filler submits bid, user selects bid, order filled on Polygon Amoy", async () => {
		const {
			tronNileId,
			polygonAmoyId,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			contractService,
			chainClientManager,
			tronWeb,
			tronIntentGatewayAddress,
		} = await setUpTron()

		const intentFiller = createIntentFiller(chainConfigs, fillerConfig, chainConfigService)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdt = chainConfigService.getUsdtAsset(tronNileId)
		const destUsdt = chainConfigService.getUsdtAsset(polygonAmoyId)

		const sourceUsdtDecimals = await contractService.getTokenDecimals(sourceUsdt, tronNileId)
		const destUsdtDecimals = await contractService.getTokenDecimals(destUsdt, polygonAmoyId)
		const amount = parseUnits("0.1", sourceUsdtDecimals)

		const inputs: TokenInfoV2[] = [{ token: bytes20ToBytes32(sourceUsdt), amount }]
		const outputs: TokenInfoV2[] = [
			{
				token: bytes20ToBytes32(destUsdt),
				amount: parseUnits("0.094", destUsdtDecimals),
			},
		]

		const privateKey = process.env.PRIVATE_KEY as HexString
		const beneficiaryAddress = privateKeyToAccount(privateKey).address
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)

		let order: OrderV2 = {
			user: bytes20ToBytes32(beneficiaryAddress),
			source: toHex(tronNileId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: parseUnits("0.005", sourceUsdtDecimals),
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_GARGANTUA!,
			process.env.SECRET_PHRASE!,
		)

		const tronEvmChain = new EvmChain({
			chainId: 3448148188,
			host: chainConfigService.getHostAddress(tronNileId),
			rpcUrl: chainConfigService.getRpcUrl(tronNileId),
		})

		const polygonAmoyEvmChain = new EvmChain({
			chainId: 80002,
			host: chainConfigService.getHostAddress(polygonAmoyId),
			rpcUrl: chainConfigService.getRpcUrl(polygonAmoyId),
		})

		await approveTronTokens(tronWeb, sourceUsdt, tronIntentGatewayAddress)

		const bundlerUrl = process.env.BUNDLER_URL
		const userSdkHelper = new IntentGatewayV2(tronEvmChain, polygonAmoyEvmChain, intentsCoprocessor, bundlerUrl)

		const generator = userSdkHelper.preparePlaceOrder(order)
		const firstResult = await generator.next()
		const { calldata, sessionPrivateKey } = firstResult.value as {
			calldata: HexString
			sessionPrivateKey: HexString
		}

		const txHash = await sendTronTransaction(tronWeb, tronIntentGatewayAddress, calldata)

		const secondResult = await generator.next(txHash)
		order = secondResult.value as OrderV2

		const { userOpHash, selectedSolver } = await executeOrderFlow(userSdkHelper, order, sessionPrivateKey, 240_000)
		expect(userOpHash).toBeDefined()
		expect(selectedSolver).toBeDefined()

		const polygonAmoyPublicClient = chainClientManager.getPublicClient(polygonAmoyId)
		const isFilled = await pollForOrderFilled(
			order.id as HexString,
			polygonAmoyPublicClient,
			chainConfigService.getIntentGatewayV2Address(polygonAmoyId),
		)
		expect(isFilled).toBe(true)

		intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)
})

// ============================================================================
// Shared Helpers
// ============================================================================

function createIntentFiller(
	chainConfigs: ChainConfig[],
	fillerConfig: FillerConfig,
	chainConfigService: FillerConfigService,
): IntentFiller {
	const privateKey = process.env.PRIVATE_KEY as HexString
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, privateKey)
	const contractService = new ContractInteractionService(
		chainClientManager,
		privateKey,
		chainConfigService,
		cacheService,
		process.env.BUNDLER_URL,
	)

	const bpsPolicy = new FillerBpsPolicy({
		points: [
			{ amount: "1", value: 50 },
			{ amount: "10000", value: 50 },
		],
	})

	const strategies = [new BasicFiller(privateKey, chainConfigService, chainClientManager, contractService, bpsPolicy)]

	return new IntentFiller(
		chainConfigs,
		strategies,
		fillerConfig,
		chainConfigService,
		chainClientManager,
		contractService,
		privateKey,
	)
}

async function executeOrderFlow(
	sdkHelper: IntentGatewayV2,
	order: OrderV2,
	sessionPrivateKey: HexString,
	bidTimeoutMs = 120_000,
): Promise<{ userOpHash: HexString; selectedSolver: HexString }> {
	let userOpHash: HexString | undefined
	let selectedSolver: HexString | undefined

	for await (const status of sdkHelper.executeIntentOrder({
		order,
		sessionPrivateKey,
		minBids: 1,
		bidTimeoutMs,
		pollIntervalMs: 5_000,
	})) {
		switch (status.status) {
			case "BIDS_RECEIVED":
				console.log(`Received ${status.metadata.bidCount} bid(s)`)
				break
			case "BID_SELECTED":
				selectedSolver = status.metadata.selectedSolver as HexString
				userOpHash = status.metadata.userOpHash as HexString
				console.log(`Selected solver: ${selectedSolver}`)
				break
			case "USEROP_SUBMITTED":
				console.log(`UserOp submitted, tx: ${status.metadata.transactionHash}`)
				break
			case "FAILED":
				throw new Error(`Order execution failed: ${status.metadata.error}`)
		}
	}

	return { userOpHash: userOpHash!, selectedSolver: selectedSolver! }
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

	const testChainConfigs: UserProvidedChainConfig[] = [
		{ chainId: 97, rpcUrl: process.env.BSC_CHAPEL! },
		{ chainId: 80002, rpcUrl: process.env.POLYGON_AMOY! },
	]

	const fillerConfigForService: FillerServiceConfig = {
		privateKey: process.env.PRIVATE_KEY as HexString,
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_GARGANTUA,
		substratePrivateKey: process.env.SECRET_PHRASE,
		solverAccountContractAddress: "0xCDFcFeD7A14154846808FddC8Ba971A2f8a830a3",
		bundlerUrl: process.env.BUNDLER_URL,
	}

	const chainConfigService = new FillerConfigService(testChainConfigs, fillerConfigForService)
	const chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

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

	const fillerConfig: FillerConfig = {
		confirmationPolicy: {
			getConfirmationBlocks: (chainId: number, amountUsd: number) =>
				confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
		},
		maxConcurrentOrders: 5,
		pendingQueueConfig: {
			maxRechecks: 10,
			recheckDelayMs: 30_000,
		},
	}

	const privateKey = process.env.PRIVATE_KEY as HexString
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, privateKey)
	const contractService = new ContractInteractionService(
		chainClientManager,
		privateKey,
		chainConfigService,
		cacheService,
		chainConfigService.getBundlerUrl(),
	)

	const bscWalletClient = chainClientManager.getWalletClient(bscChapelId)
	const bscPublicClient = chainClientManager.getPublicClient(bscChapelId)
	const polygonAmoyPublicClient = chainClientManager.getPublicClient(polygonAmoyId)

	const bscIntentGatewayV2 = getContract({
		address: chainConfigService.getIntentGatewayV2Address(bscChapelId),
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
// Tron Setup
// ============================================================================

async function setUpTron() {
	const tronNileId = "EVM-3448148188"
	const polygonAmoyId = "EVM-80002"
	const chains = [tronNileId, polygonAmoyId]

	const testChainConfigs: UserProvidedChainConfig[] = [
		{ chainId: 3448148188, rpcUrl: process.env.TRON_NILE! },
		{ chainId: 80002, rpcUrl: process.env.POLYGON_AMOY! },
	]

	const fillerConfigForService: FillerServiceConfig = {
		privateKey: process.env.PRIVATE_KEY as HexString,
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_GARGANTUA,
		substratePrivateKey: process.env.SECRET_PHRASE,
		solverAccountContractAddress: "0xCDFcFeD7A14154846808FddC8Ba971A2f8a830a3",
		bundlerUrl: process.env.BUNDLER_URL,
	}

	const chainConfigService = new FillerConfigService(testChainConfigs, fillerConfigForService)
	const chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const confirmationPolicy = new ConfirmationPolicy({
		"3448148188": {
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

	const fillerConfig: FillerConfig = {
		confirmationPolicy: {
			getConfirmationBlocks: (chainId: number, amountUsd: number) =>
				confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
		},
		maxConcurrentOrders: 5,
		pendingQueueConfig: {
			maxRechecks: 10,
			recheckDelayMs: 30_000,
		},
	}

	const privateKey = process.env.PRIVATE_KEY as HexString
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, privateKey)
	const contractService = new ContractInteractionService(
		chainClientManager,
		privateKey,
		chainConfigService,
		cacheService,
		chainConfigService.getBundlerUrl(),
	)

	const tronWeb = new TronWeb({
		fullHost: process.env.TRON_NILE,
		privateKey: privateKey.slice(2),
	})

	const tronIntentGatewayAddress = "TT4CjjHw7QgLbE9wKtYEopid1YqePkbAfb"

	return {
		tronNileId,
		polygonAmoyId,
		chainConfigs,
		fillerConfig,
		chainConfigService,
		contractService,
		chainClientManager,
		tronWeb,
		tronIntentGatewayAddress,
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

async function approveTronTokens(
	tronWeb: InstanceType<typeof TronWeb>,
	tokenAddress: HexString,
	spenderBase58: string,
) {
	const tokenBase58 = TronWeb.address.fromHex(`41${tokenAddress.slice(2)}`)
	const contract = tronWeb.contract(ERC20_ABI, tokenBase58)
	const allowance = await contract.methods.allowance(tronWeb.defaultAddress.base58 as string, spenderBase58).call()

	if (BigInt(allowance.toString()) === 0n) {
		console.log(`Approving TRC20 token ${tokenBase58} for ${spenderBase58}`)
		const tx = await contract.methods.approve(spenderBase58, maxUint256).send({
			feeLimit: 100_000_000,
		})
		await waitForTronConfirmation(tronWeb, tx)
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

// ============================================================================
// Tron Transaction Helpers
// ============================================================================

async function sendTronTransaction(
	tronWeb: InstanceType<typeof TronWeb>,
	contractBase58: string,
	calldata: HexString,
): Promise<HexString> {
	const decoded = decodeFunctionData({
		abi: INTENT_GATEWAY_V2_ABI,
		data: calldata,
	})

	if (!decoded.args || decoded.args.length < 2) {
		throw new Error("Failed to decode placeOrder calldata")
	}

	const [orderObj, graffiti] = decoded.args as [OrderV2, HexString]

	const orderArray = [
		orderObj.user,
		orderObj.source,
		orderObj.destination,
		orderObj.deadline,
		orderObj.nonce,
		orderObj.fees,
		orderObj.session,
		[orderObj.predispatch.assets.map((a: any) => [a.token, a.amount]), orderObj.predispatch.call],
		orderObj.inputs.map((i: any) => [i.token, i.amount]),
		[
			orderObj.output.beneficiary,
			orderObj.output.assets.map((a: any) => [a.token, a.amount]),
			orderObj.output.call,
		],
	]

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const contract = tronWeb.contract(INTENT_GATEWAY_V2_ABI as unknown as any[], contractBase58)
	const txId = await contract.methods.placeOrder(orderArray, graffiti).send({
		feeLimit: 1_000_000_000,
	})

	console.log("Tron placeOrder tx:", txId)
	await waitForTronConfirmation(tronWeb, txId)

	return `0x${txId}` as HexString
}

async function waitForTronConfirmation(tronWeb: InstanceType<typeof TronWeb>, txId: string, maxAttempts = 30) {
	for (let i = 0; i < maxAttempts; i++) {
		try {
			const txInfo = await tronWeb.trx.getTransactionInfo(txId)
			if (txInfo?.id) {
				if (txInfo.receipt?.result === "SUCCESS") return
				if (txInfo.receipt?.result && txInfo.receipt.result !== "SUCCESS") {
					throw new Error(`Tron tx failed with status: ${txInfo.receipt.result}`)
				}
			}
		} catch {
			// Transaction not yet indexed, keep polling
		}
		await new Promise((resolve) => setTimeout(resolve, 3_000))
	}
	throw new Error(`Tron transaction ${txId} not confirmed after ${maxAttempts} attempts`)
}
