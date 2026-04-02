import { IntentFiller } from "@/core/filler"
import {
	CacheService,
	ChainClientManager,
	ContractInteractionService,
	FillerConfigService,
	type ResolvedChainConfig,
	type FillerConfig as FillerServiceConfig,
} from "@/services"
import { createSimplexSigner, SignerType } from "@/services/wallet"
import { BasicFiller } from "@/strategies/basic"
import {
	type ChainConfig,
	type FillerConfig,
	type HexString,
	type Order,
	type TokenInfo,
	bytes20ToBytes32,
	EvmChain,
	IntentGateway,
	IntentsCoprocessor,
	TronChain,
	PLACE_ORDER_SELECTOR,
	ORDER_V2_PARAM_TYPE,
	DEFAULT_GRAFFITI,
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
import "../setup"
import { pimlicoBundlerUrlForChain as bundlerUrl } from "../pimlicoBundler"
import { ERC20_ABI } from "@/config/abis/ERC20"
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

		const intentFiller = await createIntentFiller(chainConfigs, fillerConfig, chainConfigService)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(bscChapelId)
		const destUsdc = chainConfigService.getUsdcAsset(polygonAmoyId)

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, bscChapelId)
		const destUsdcDecimals = await contractService.getTokenDecimals(destUsdc, polygonAmoyId)
		const amount = parseUnits("0.1", sourceUsdcDecimals)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(destUsdc),
				amount: amount - parseUnits("0.094", destUsdcDecimals),
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

		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, { bidTimeoutMs: 120_000, pollIntervalMs: 5_000 })
		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data, value } = result.value
			const signedTx = (await bscWalletClient.signTransaction(
				(await bscPublicClient.prepareTransactionRequest({
					to,
					data,
					value,
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
				}
				if (status.status === "USEROP_SUBMITTED" && status.transactionHash) {
					console.log("Transaction hash:", status.transactionHash)
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
			chainConfigService.getIntentGatewayV2Address(polygonAmoyId),
		)
		expect(isFilled).toBe(true)

		await intentFiller.stop()
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

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(destUsdc),
				amount: amount - parseUnits("0.094", destUsdcDecimals),
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

		console.log("Preparing to place order...")
		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI)
		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data, value } = result.value

			console.log("Signing place order transaction...")
			const preparedTx = await bscPublicClient.prepareTransactionRequest({
				to,
				data,
				value: 0n,
				account: bscWalletClient.account!,
				chain: bscWalletClient.chain,
			})
			const signedTx = (await bscWalletClient.signTransaction(preparedTx as any)) as HexString
			result = await gen.next(signedTx)
		}

		if (result.value && "status" in result.value && result.value.status === "ORDER_PLACED") {
			order = result.value.order as Order
			console.log(`Order placed successfully with ID: ${order.id}`)
		}

		expect(order.id).toBeDefined()
		expect(order.user).toBe(bytes20ToBytes32(beneficiaryAddress))
		expect(order.source).toBe(toHex(bscChapelId))
		expect(order.destination).toBe(toHex(polygonAmoyId))

		await intentsCoprocessor.disconnect()
	}, 300_000)
})

describe.skip("Filler V2 - Tron Source Chain", () => {
	it("Should place order on Tron Nile, filler submits bid, user selects bid, order filled on Polygon Amoy", async () => {
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

		const intentFiller = await createIntentFiller(chainConfigs, fillerConfig, chainConfigService)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdt = chainConfigService.getUsdtAsset(tronNileId)
		const destUsdt = chainConfigService.getUsdtAsset(polygonAmoyId)

		const sourceUsdtDecimals = await contractService.getTokenDecimals(sourceUsdt, tronNileId)
		const destUsdtDecimals = await contractService.getTokenDecimals(destUsdt, polygonAmoyId)
		const amount = parseUnits("0.1", sourceUsdtDecimals)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdt), amount }]
		const outputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(destUsdt),
				amount: parseUnits("0.094", destUsdtDecimals),
			},
		]

		const privateKey = process.env.PRIVATE_KEY as HexString
		const beneficiaryAddress = privateKeyToAccount(privateKey).address
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)

		let order: Order = {
			user: bytes20ToBytes32(beneficiaryAddress),
			source: toHex(tronNileId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: 0n,
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_GARGANTUA!,
			process.env.SECRET_PHRASE!,
		)

		const tronChain = await TronChain.fromParams({
			chainId: 3448148188,
			host: chainConfigService.getHostAddress(tronNileId),
			rpcUrl: chainConfigService.getRpcUrl(tronNileId),
		})

		const destBundlerUrl = chainConfigService.getBundlerUrl(polygonAmoyId)
		const polygonAmoyEvmChain = EvmChain.fromParams({
			chainId: 80002,
			host: chainConfigService.getHostAddress(polygonAmoyId),
			rpcUrl: chainConfigService.getRpcUrl(polygonAmoyId),
			bundlerUrl: destBundlerUrl,
		})

		await approveTronTokens(tronWeb, sourceUsdt, tronIntentGatewayAddress)

		const userSdkHelper = await IntentGateway.create(tronChain, polygonAmoyEvmChain, intentsCoprocessor)

		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, { bidTimeoutMs: 240_000, pollIntervalMs: 5_000 })
		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { data } = result.value
			const signedTx = (await signTronTransaction(tronWeb, tronIntentGatewayAddress, data)) as HexString
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
				}
				if (status.status === "USEROP_SUBMITTED" && status.transactionHash) {
					console.log("Transaction hash:", status.transactionHash)
				}
				if (status.status === "FAILED") {
					throw new Error(`Order execution failed: ${status.error}`)
				}
			}
			result = await gen.next()
		}
		expect(userOpHash).toBeDefined()
		expect(selectedSolver).toBeDefined()

		const polygonAmoyPublicClient = chainClientManager.getPublicClient(polygonAmoyId)
		const isFilled = await pollForOrderFilled(
			order.id as HexString,
			polygonAmoyPublicClient,
			chainConfigService.getIntentGatewayV2Address(polygonAmoyId),
		)
		expect(isFilled).toBe(true)

		await intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)
})

// ============================================================================
// Shared Helpers
// ============================================================================

async function createIntentFiller(
	chainConfigs: ChainConfig[],
	fillerConfig: FillerConfig,
	chainConfigService: FillerConfigService,
): Promise<IntentFiller> {
	const privateKey = process.env.PRIVATE_KEY as HexString
	const signer = await createSimplexSigner({ type: SignerType.PrivateKey, key: privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)
	const contractService = new ContractInteractionService(
		chainClientManager,
		chainConfigService,
		signer,
		cacheService,
	)

	const bpsPolicy = new FillerBpsPolicy({
		points: [
			{ amount: "1", value: 50 },
			{ amount: "10000", value: 50 },
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

	const strategies = [
		new BasicFiller(
			signer,
			chainConfigService,
			chainClientManager,
			contractService,
			bpsPolicy,
			confirmationPolicy,
		),
	]

	return new IntentFiller(
		chainConfigs,
		strategies,
		fillerConfig,
		chainConfigService,
		chainClientManager,
		contractService,
		signer,
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
		{ chainId: 97, rpcUrl: process.env.BSC_CHAPEL!, bundlerUrl: bundlerUrl(97) },
		{ chainId: 80002, rpcUrl: process.env.POLYGON_AMOY!, bundlerUrl: bundlerUrl(80002) },
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
	const contractService = new ContractInteractionService(
		chainClientManager,
		chainConfigService,
		signer,
		cacheService,
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

	const testChainConfigs: ResolvedChainConfig[] = [
		{ chainId: 3448148188, rpcUrl: process.env.TRON_NILE! },
		{ chainId: 80002, rpcUrl: process.env.POLYGON_AMOY!, bundlerUrl: bundlerUrl(80002) },
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
	const contractService = new ContractInteractionService(
		chainClientManager,
		chainConfigService,
		signer,
		cacheService,
	)

	const tronWeb = new TronWeb({
		fullHost: process.env.TRON_NILE,
		privateKey: privateKey.slice(2),
	})

	const tronIntentGatewayAddress = "TMcm6r9RRVKPJNLgyFxcuJknFruQBuPumF"

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

async function signTronTransaction(
	tronWeb: InstanceType<typeof TronWeb>,
	contractBase58: string,
	calldata: HexString,
): Promise<any> {
	const decoded = decodeFunctionData({ abi: INTENT_GATEWAY_V2_ABI, data: calldata })
	if (!decoded.args || decoded.args.length < 2) {
		throw new Error("Failed to decode placeOrder calldata")
	}

	const [order, graffiti] = decoded.args as [Order, HexString]

	const orderTuple = [
		order.user,
		order.source,
		order.destination,
		order.deadline,
		order.nonce,
		order.fees,
		order.session,
		[order.predispatch.assets.map((a) => [a.token, a.amount]), order.predispatch.call],
		order.inputs.map((i) => [i.token, i.amount]),
		[order.output.beneficiary, order.output.assets.map((a) => [a.token, a.amount]), order.output.call],
	]

	const { transaction } = await (tronWeb.transactionBuilder as any).triggerSmartContract(
		TronWeb.address.toHex(contractBase58),
		PLACE_ORDER_SELECTOR,
		{ feeLimit: 1_000_000_000 },
		[
			{ type: ORDER_V2_PARAM_TYPE, value: orderTuple },
			{ type: "bytes32", value: graffiti },
		],
		tronWeb.defaultAddress.hex,
	)

	if (!transaction) throw new Error("Failed to build Tron placeOrder transaction")

	return tronWeb.trx.sign(transaction)
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
