import { IntentFiller } from "@/core/filler"
import {
	ChainClientManager,
	ContractInteractionService,
	FillerConfigService,
	UserProvidedChainConfig,
} from "@/services"
import { BasicFiller } from "@/strategies/basic"
import {
	ChainConfig,
	FillerConfig,
	HexString,
	Order,
	PaymentInfo,
	TokenInfo,
	IndexerClient,
	createQueryClient,
	getRequestCommitment,
	RequestStatus,
	orderCommitment,
	bytes20ToBytes32,
	ADDRESS_ZERO,
	postRequestCommitment,
	EvmChain,
	IntentGateway,
	DEFAULT_GRAFFITI,
} from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { ConfirmationPolicy } from "@/config/confirmation-policy"
import {
	decodeFunctionData,
	encodePacked,
	getContract,
	hexToString,
	keccak256,
	maxUint256,
	parseAbi,
	parseAbiItem,
	parseEventLogs,
	parseUnits,
	PublicClient,
	WalletClient,
} from "viem"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { bscTestnet, gnosisChiado } from "viem/chains"
import "./setup"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { HandlerV1_ABI } from "@/config/abis/HandlerV1"
import { UNISWAP_ROUTER_V2_ABI } from "@/config/abis/UniswapRouterV2"

import { compareDecimalValues } from "@/utils"
import { readFileSync } from "fs"
import { resolve, dirname } from "path"
import { fileURLToPath } from "url"
import { parse } from "toml"
import { Decimal } from "decimal.js"

// Helper function to load test configuration from TOML
function loadTestConfig() {
	const __filename = fileURLToPath(import.meta.url)
	const __dirname = dirname(__filename)
	const configPath = resolve(__dirname, "test-config.toml")
	let tomlContent = readFileSync(configPath, "utf-8")

	// Replace environment variable placeholders with actual values
	tomlContent = tomlContent.replace(
		/\$\{BSC_CHAPEL\}/g,
		process.env.BSC_CHAPEL || "https://bnb-testnet.api.onfinality.io/public",
	)
	tomlContent = tomlContent.replace(
		/\$\{GNOSIS_CHIADO\}/g,
		process.env.GNOSIS_CHIADO || "https://gnosis-chiado-rpc.publicnode.com",
	)
	tomlContent = tomlContent.replace(
		/\$\{ETH_MAINNET\}/g,
		process.env.ETH_MAINNET || "https://eth-mainnet.g.alchemy.com/v2/demo",
	)
	tomlContent = tomlContent.replace(/\$\{BSC_MAINNET\}/g, process.env.BSC_MAINNET || "https://binance.llamarpc.com")
	tomlContent = tomlContent.replace(
		/\$\{HYPERBRIDGE_GARGANTUA\}/g,
		process.env.HYPERBRIDGE_GARGANTUA || "wss://gargantua.hyperbridge.xyz",
	)

	return parse(tomlContent)
}

describe.sequential("Basic", () => {
	let indexer: IndexerClient

	beforeAll(async () => {
		const { bscIsmpHost, gnosisChiadoIsmpHost } = await setUp()
		const queryClient = createQueryClient({
			url: process.env.INDEXER_URL!,
		})

		indexer = new IndexerClient({
			source: {
				consensusStateId: "BSC0",
				rpcUrl: process.env.BSC_CHAPEL!,
				stateMachineId: "EVM-97",
				host: bscIsmpHost.address,
			},
			dest: {
				consensusStateId: "GNO0",
				rpcUrl: process.env.GNOSIS_CHIADO!,
				stateMachineId: "EVM-10200",
				host: gnosisChiadoIsmpHost.address,
			},
			hyperbridge: {
				consensusStateId: "PAS0",
				stateMachineId: "KUSAMA-4009",
				wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			},
			queryClient: queryClient,
			pollInterval: 1_000,
		})
	}, 1_000_000)

	it.skip("Should listen, place order, fill order, and check if filled at the source chain", async () => {
		const {
			bscIntentGateway,
			gnosisChiadoIntentGateway,
			bscWalletClient,
			bscPublicClient,
			bscIsmpHost,
			gnosisChiadoIsmpHost,
			feeTokenBscAddress,
			chainConfigs,
			fillerConfig,
			gnosisChiadoPublicClient,
			bscEvmHelper,
			gnosisChiadoEvmHelper,
			chainConfigService,
			bscChapelId,
		} = await setUp()

		const intentGatewayHelper = new IntentGateway(bscEvmHelper, gnosisChiadoEvmHelper)
		const strategies = [new BasicFiller(process.env.PRIVATE_KEY as HexString, chainConfigService)]
		const intentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig, chainConfigService)
		intentFiller.start()

		const daiAsset = chainConfigService.getDaiAsset(bscChapelId)
		const inputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(daiAsset),
				amount: 100n,
			},
		]
		const outputs: PaymentInfo[] = [
			{
				token: "0x0000000000000000000000000000000000000000000000000000000000000000",
				amount: 100n,
				beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
			},
		]

		let order = {
			user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
			sourceChain: await bscIsmpHost.read.host(),
			destChain: await gnosisChiadoIsmpHost.read.host(),
			deadline: 65337297n,
			nonce: 0n,
			fees: 0n,
			outputs,
			inputs,
			callData: "0x" as HexString,
		}

		const estimatedFees = await intentGatewayHelper.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})

		order.fees = estimatedFees.feeTokenAmount

		await approveTokens(bscWalletClient, bscPublicClient, feeTokenBscAddress, bscIntentGateway.address)

		const orderDetectedPromise = new Promise<Order>((resolve) => {
			const eventMonitor = intentFiller.monitor
			if (!eventMonitor) {
				console.error("Event monitor not found on intentFiller")
				resolve({} as Order)
				return
			}

			eventMonitor.on("newOrder", (data: { order: Order }) => {
				resolve(data.order)
			})
		})

		const hash = await bscIntentGateway.write.placeOrder([order, DEFAULT_GRAFFITI], {
			account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
			chain: bscTestnet,
		})

		const receipt = await bscPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		console.log("Order placed on BSC:", receipt.transactionHash)

		console.log("Waiting for event monitor to detect the order...")
		const detectedOrder = await orderDetectedPromise
		console.log("Order successfully detected by event monitor:", detectedOrder)

		const orderFilledPromise = new Promise<{ orderId: string; hash: string }>((resolve) => {
			const eventMonitor = intentFiller.monitor
			if (!eventMonitor) {
				console.error("Event monitor not found on intentFiller")
				resolve({ orderId: "", hash: "" })
				return
			}

			eventMonitor.on("orderFilled", (data: { orderId: string; hash: string }) => {
				console.log("Order filled by event monitor:", data.orderId, data.hash)
				resolve(data)
			})
		})

		const { orderId, hash: filledHash } = await orderFilledPromise
		console.log("Order filled:", orderId, filledHash)

		const filledReceipt = await gnosisChiadoPublicClient.waitForTransactionReceipt({
			hash: filledHash as `0x${string}`,
			confirmations: 1,
		})

		let isFilled = await checkIfOrderFilled(
			orderId as HexString,
			gnosisChiadoPublicClient,
			gnosisChiadoIntentGateway.address,
		)

		expect(isFilled).toBe(true)

		// parse EvmHost PostRequestEvent emitted in the transcation logs
		const event = parseEventLogs({ abi: EVM_HOST, logs: filledReceipt.logs })[0]

		if (event.eventName !== "PostRequestEvent") {
			throw new Error("Unexpected Event type")
		}

		const request = event.args
		console.log("PostRequestEvent", { request })
		const commitment = postRequestCommitment(request).commitment

		for await (const status of indexer.postRequestStatusStream(commitment)) {
			console.log(JSON.stringify(status, null, 4))
			switch (status.status) {
				case RequestStatus.SOURCE_FINALIZED: {
					console.log(
						`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
					)
					break
				}
				case RequestStatus.HYPERBRIDGE_DELIVERED: {
					console.log(
						`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
					)
					break
				}
				case RequestStatus.HYPERBRIDGE_FINALIZED: {
					console.log(
						`Status ${status.status}, Transaction: https://gnosis-chiado.blockscout.com/tx/${status.metadata.transactionHash}`,
					)

					break
				}
				case RequestStatus.DESTINATION: {
					console.log(
						`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
					)

					// Check if the order is filled at the source chain
					const isFilled = await checkIfOrderFilled(
						orderId as HexString,
						bscPublicClient,
						bscIntentGateway.address,
					)
					expect(isFilled).toBe(true)
					break
				}
			}
		}

		intentFiller.stop()
	}, 1_000_000)

	it("Should listen, place order, fill order at BSC Chapel, and check if filled at Gnosis Chiado", async () => {
		const {
			bscIntentGateway,
			gnosisChiadoIntentGateway,
			bscWalletClient,
			bscPublicClient,
			bscIsmpHost,
			gnosisChiadoIsmpHost,
			feeTokenGnosisChiadoAddress,
			chainConfigs,
			fillerConfig,
			gnosisChiadoPublicClient,
			bscEvmHelper,
			gnosisChiadoEvmHelper,
			chainConfigService,
			bscChapelId,
			gnosisChiadoId,
			gnosisChiadoWalletClient,
		} = await setUp()

		const intentGatewayHelper = new IntentGateway(gnosisChiadoEvmHelper, bscEvmHelper)
		const strategies = [new BasicFiller(process.env.PRIVATE_KEY as HexString, chainConfigService)]
		const intentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig, chainConfigService)
		intentFiller.start()

		const inputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(chainConfigService.getDaiAsset(gnosisChiadoId)),
				amount: 100n,
			},
		]
		const outputs: PaymentInfo[] = [
			{
				token: bytes20ToBytes32(chainConfigService.getDaiAsset(bscChapelId)),
				amount: 100n,
				beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
			},
		]

		let order = {
			user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
			sourceChain: await gnosisChiadoIsmpHost.read.host(),
			destChain: await bscIsmpHost.read.host(),
			deadline: 65337297000n,
			nonce: 0n,
			fees: 0n,
			outputs,
			inputs,
			callData: "0x" as HexString,
		}

		const estimatedFees = await intentGatewayHelper.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})

		order.fees = estimatedFees.feeTokenAmount

		await approveTokens(
			gnosisChiadoWalletClient,
			gnosisChiadoPublicClient,
			feeTokenGnosisChiadoAddress,
			gnosisChiadoIntentGateway.address,
		)

		await approveTokens(
			gnosisChiadoWalletClient,
			gnosisChiadoPublicClient,
			chainConfigService.getDaiAsset(gnosisChiadoId),
			gnosisChiadoIntentGateway.address,
		)

		const orderDetectedPromise = new Promise<Order>((resolve) => {
			const eventMonitor = intentFiller.monitor
			if (!eventMonitor) {
				console.error("Event monitor not found on intentFiller")
				resolve({} as Order)
				return
			}

			eventMonitor.on("newOrder", (data: { order: Order }) => {
				resolve(data.order)
			})
		})

		const hash = await gnosisChiadoIntentGateway.write.placeOrder(
			[order, bytes20ToBytes32("0x7f5f2cf1aec83bf0c74df566a41aa7ed65ea84ea")],
			{
				account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
				chain: gnosisChiado,
			},
		)

		const receipt = await gnosisChiadoPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		console.log("Order placed on Gnosis Chiado:", receipt.transactionHash)

		console.log("Waiting for event monitor to detect the order...")
		const detectedOrder = await orderDetectedPromise
		console.log("Order successfully detected by event monitor:", detectedOrder)

		const orderFilledPromise = new Promise<{ orderId: string; hash: string }>((resolve) => {
			const eventMonitor = intentFiller.monitor
			if (!eventMonitor) {
				console.error("Event monitor not found on intentFiller")
				resolve({ orderId: "", hash: "" })
				return
			}

			eventMonitor.on("orderFilled", (data: { orderId: string; hash: string }) => {
				console.log("Order filled by event monitor:", data.orderId, data.hash)
				resolve(data)
			})
		})

		const { orderId, hash: filledHash } = await orderFilledPromise
		console.log("Order filled:", orderId, filledHash)

		const filledReceipt = await bscPublicClient.waitForTransactionReceipt({
			hash: filledHash as `0x${string}`,
			confirmations: 1,
		})

		let isFilled = await checkIfOrderFilled(orderId as HexString, bscPublicClient, bscIntentGateway.address)

		expect(isFilled).toBe(true)

		// parse EvmHost PostRequestEvent emitted in the transcation logs
		const event = parseEventLogs({ abi: EVM_HOST, logs: filledReceipt.logs })[0]

		if (event.eventName !== "PostRequestEvent") {
			throw new Error("Unexpected Event type")
		}

		const request = event.args
		console.log("PostRequestEvent", { request })
		const commitment = postRequestCommitment(request).commitment

		console.log("Post request commitment: ", commitment)

		for await (const status of indexer.postRequestStatusStream(commitment)) {
			console.log(JSON.stringify(status, null, 4))
			switch (status.status) {
				case RequestStatus.SOURCE_FINALIZED: {
					console.log(
						`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
					)
					break
				}
				case RequestStatus.HYPERBRIDGE_DELIVERED: {
					console.log(
						`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
					)
					break
				}
				case RequestStatus.HYPERBRIDGE_FINALIZED: {
					console.log(
						`Status ${status.status}, Transaction: https://gnosis-chiado.blockscout.com/tx/${status.metadata.transactionHash}`,
					)

					break
				}
				case RequestStatus.DESTINATION: {
					console.log(
						`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
					)

					// Check if the order is filled at the source chain
					const isFilled = await checkIfOrderFilled(
						orderId as HexString,
						gnosisChiadoPublicClient,
						gnosisChiadoIntentGateway.address,
					)
					expect(isFilled).toBe(true)
					break
				}
			}
		}

		intentFiller.stop()
	}, 1_000_000)

	it.skip("Should timeout if order deadline is reached", async () => {
		const {
			bscIntentGateway,
			bscWalletClient,
			bscPublicClient,
			bscIsmpHost,
			gnosisChiadoIsmpHost,
			feeTokenBscAddress,
			contractInteractionService,
			gnosisChiadoIntentGateway,
			bscHandler,
			bscChapelId,
			chainConfigService,
		} = await setUp()

		const daiAsset = chainConfigService.getDaiAsset(bscChapelId)

		const inputs: TokenInfo[] = [
			{
				token: bytes20ToBytes32(daiAsset),
				amount: 100n,
			},
		]
		const outputs: PaymentInfo[] = [
			{
				token: "0x0000000000000000000000000000000000000000000000000000000000000000",
				amount: 100n,
				beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
			},
		]

		const order = {
			user: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
			sourceChain: await bscIsmpHost.read.host(),
			destChain: await gnosisChiadoIsmpHost.read.host(),
			deadline: 0n, // Expired deadline
			nonce: 0n,
			fees: 1000000n,
			outputs,
			inputs,
			callData: "0x" as HexString,
		}

		await approveTokens(bscWalletClient, bscPublicClient, feeTokenBscAddress, bscIntentGateway.address)

		let hash = await bscIntentGateway.write.placeOrder([order, DEFAULT_GRAFFITI], {
			account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
			chain: bscTestnet,
		})

		let receipt = await bscPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		const orderPlaceEvent = parseEventLogs({ abi: INTENT_GATEWAY_ABI, logs: receipt.logs })[0]

		if (orderPlaceEvent.eventName !== "OrderPlaced") {
			throw new Error("Unexpected Event type")
		}

		const orderPlaced = orderPlaceEvent.args

		console.log("Order placed on BSC:", orderPlaced)

		// Now cancel the order

		const latestHeightDestChain = await contractInteractionService.getHostLatestStateMachineHeight(
			hexToString(order.destChain),
		)

		const cancelOptions = {
			relayerFee: 10000000000n,
			height: latestHeightDestChain,
		}

		hash = await bscIntentGateway.write.cancelOrder([orderPlaced, cancelOptions], {
			account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
			chain: bscTestnet,
		})

		receipt = await bscPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		console.log("Order cancelled on BSC:", receipt.transactionHash)

		// parse EvmHost GetRequestEvent emitted in the transcation logs
		const event = parseEventLogs({ abi: EVM_HOST, logs: receipt.logs })[0]

		if (event.eventName !== "GetRequestEvent") {
			throw new Error("Unexpected Event type")
		}

		const request = event.args
		console.log("GetRequestEvent", { request })
		const commitment = getRequestCommitment({ ...request, keys: [...request.keys] })
		console.log("Get Request Commitment: ", commitment)

		for await (const status of indexer.getRequestStatusStream(commitment)) {
			switch (status.status) {
				case RequestStatus.SOURCE_FINALIZED: {
					console.log(
						`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
					)
					break
				}
				case RequestStatus.HYPERBRIDGE_DELIVERED: {
					console.log(
						`Status ${status.status}, Transaction: https://gargantua.statescan.io/#/extrinsics/${status.metadata.transactionHash}`,
					)
					break
				}
				case RequestStatus.HYPERBRIDGE_FINALIZED: {
					console.log(
						`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
					)
					const { args, functionName } = decodeFunctionData({
						abi: HandlerV1_ABI,
						data: status.metadata.calldata,
					})

					expect(functionName).toBe("handleGetResponses")

					try {
						const hash = await bscHandler.write.handleGetResponses(args as any, {
							account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
							chain: bscTestnet,
						})
						receipt = await bscPublicClient.waitForTransactionReceipt({
							hash,
							confirmations: 1,
						})

						console.log(`Transaction submitted: https://testnet.bscscan.com/tx/${hash}`)

						// Check for the EscrowRefunded event
						const escrowRefundedEvent = parseEventLogs({ abi: INTENT_GATEWAY_ABI, logs: receipt.logs })[0]
						if (escrowRefundedEvent.eventName !== "EscrowRefunded") {
							throw new Error("Unexpected Event type")
						}

						expect(escrowRefundedEvent.args.commitment).toBe(
							orderCommitment({
								...orderPlaced,
								sourceChain: hexToString(orderPlaced.sourceChain),
								destChain: hexToString(orderPlaced.destChain),
								outputs: orderPlaced.outputs as PaymentInfo[],
								inputs: orderPlaced.inputs as TokenInfo[],
							}),
						)
					} catch (e) {
						console.error("Error self-relaying: ", e)
					}
					break
				}
				case RequestStatus.DESTINATION: {
					console.log(
						`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
					)
					break
				}
			}
		}
	}, 1_000_0000)

	it("Should validate order inputs and outputs correctly", async () => {
		const { chainConfigService, bscChapelId, mainnetId } = await setUp()

		const basicFiller = new BasicFiller(process.env.PRIVATE_KEY as HexString, chainConfigService)

		// Get token assets for both chains
		const sourceDaiAsset = chainConfigService.getDaiAsset(bscChapelId)
		const sourceUsdtAsset = chainConfigService.getUsdtAsset(bscChapelId)
		const sourceUsdcAsset = chainConfigService.getUsdcAsset(bscChapelId)

		const destDaiAsset = chainConfigService.getDaiAsset(mainnetId)
		const destUsdtAsset = chainConfigService.getUsdtAsset(mainnetId)
		const destUsdcAsset = chainConfigService.getUsdcAsset(mainnetId)

		// Test case 1: Valid order with matching USDC tokens
		const validOrder: Order = {
			user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
			sourceChain: bscChapelId,
			destChain: mainnetId,
			deadline: 65337297000n,
			nonce: 0n,
			fees: 1000000n,
			inputs: [
				{
					token: bytes20ToBytes32(sourceUsdcAsset),
					amount: 1n * 10n ** 18n, // 1 USDC (6 decimals)
				},
			],
			outputs: [
				{
					token: bytes20ToBytes32(destUsdcAsset),
					amount: 1n * 10n ** 6n, // 1 USDC (6 decimals)
					beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
				},
			],
			callData: "0x" as HexString,
		}

		// Test case 2: Invalid order with different array lengths
		const invalidLengthOrder: Order = {
			...validOrder,
			inputs: [
				{
					token: bytes20ToBytes32(sourceUsdcAsset),
					amount: 1000000n,
				},
				{
					token: bytes20ToBytes32(sourceUsdtAsset),
					amount: 1000000000n, // 1 USDT (6 decimals)
				},
			],
			outputs: [
				{
					token: bytes20ToBytes32(destUsdcAsset),
					amount: 1000000n,
					beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
				},
			],
		}

		// Test case 3: Invalid order with token type mismatch
		const invalidTokenTypeOrder: Order = {
			...validOrder,
			inputs: [
				{
					token: bytes20ToBytes32(sourceUsdcAsset),
					amount: 1000000n,
				},
			],
			outputs: [
				{
					token: bytes20ToBytes32(destUsdtAsset), // USDC input but USDT output
					amount: 1000000n,
					beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
				},
			],
		}

		// Test case 4: Invalid order with unsupported token
		const unsupportedTokenOrder: Order = {
			...validOrder,
			inputs: [
				{
					token: bytes20ToBytes32("0x1234567890123456789012345678901234567890" as HexString), // Unsupported token
					amount: 1000000n,
				},
			],
			outputs: [
				{
					token: bytes20ToBytes32(destUsdcAsset),
					amount: 1000000n,
					beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
				},
			],
		}

		const canFillValid = await basicFiller.validateOrderInputsOutputs(validOrder)
		expect(canFillValid).toBe(true)

		const canFillInvalidLength = await basicFiller.validateOrderInputsOutputs(invalidLengthOrder)
		expect(canFillInvalidLength).toBe(false)

		// Invalid token type order should fail validation
		const canFillInvalidTokenType = await basicFiller.validateOrderInputsOutputs(invalidTokenTypeOrder)
		expect(canFillInvalidTokenType).toBe(false)

		// Unsupported token order should fail validation
		const canFillUnsupportedToken = await basicFiller.validateOrderInputsOutputs(unsupportedTokenOrder)
		expect(canFillUnsupportedToken).toBe(false)

		// Direct stress test for compare decimal values

		// Core precision tests that matter
		let val1 = parseUnits("11245.123456789012345678", 18) // Full 18 decimal precision
		let val2 = parseUnits("11245.123456", 6) // 6 decimal precision
		expect(compareDecimalValues(val1, 18, val2, 6)).toBe(false) // Different due to precision loss

		let val3 = parseUnits("11245.123456000000000000", 18) // 18 decimals but only 6 significant
		let val4 = parseUnits("11245.123456", 6) // Same logical value
		expect(compareDecimalValues(val3, 18, val4, 6)).toBe(true)

		// Edge case: 1 wei difference (matters for exact comparisons)
		let wei18 = BigInt("1000000000000000001") // 1 ETH + 1 wei
		let eth6 = parseUnits("1", 6) // 1 token (6 decimals)
		expect(compareDecimalValues(wei18, 18, eth6, 6)).toBe(false) // 1 wei makes them different

		// Zero values (should always match regardless of decimals)
		let zero18 = parseUnits("0", 18)
		let zero6 = parseUnits("0", 6)
		expect(compareDecimalValues(zero18, 18, zero6, 6)).toBe(true)

		// Real stablecoin amounts (common use case)
		let usdc = parseUnits("1234.567890", 6) // USDC amount
		let dai = parseUnits("1234.567890", 18) // Same USD value in DAI
		expect(compareDecimalValues(usdc, 6, dai, 18)).toBe(true)

		// Same decimals should work (sanity check)
		let sameVal1 = parseUnits("100.5", 18)
		let sameVal2 = parseUnits("100.5", 18)
		expect(compareDecimalValues(sameVal1, 18, sameVal2, 18)).toBe(true)

		// Large numbers (stress test)
		let large18 = parseUnits("999999999999.123456000000000000", 18)
		let large6 = parseUnits("999999999999.123456", 6)
		expect(compareDecimalValues(large18, 18, large6, 6)).toBe(true)

		// Fractional precision that gets truncated (common bug source)
		let fraction18 = parseUnits("0.000000000000000123", 18) // 123 wei
		zero6 = parseUnits("0", 6) // 0 in 6 decimals
		expect(compareDecimalValues(fraction18, 18, zero6, 6)).toBe(false)
	}, 300_000)
})

describe.sequential("ConfirmationPolicy", () => {
	it("Should return correct confirmations for min, max, and interpolated amounts", () => {
		const policyConfig = {
			"1": {
				minAmount: "100",
				maxAmount: "1000",
				minConfirmations: 2,
				maxConfirmations: 12,
			},
		}

		const policy = new ConfirmationPolicy(policyConfig)

		// Test boundaries
		expect(policy.getConfirmationBlocks(1, new Decimal(50))).toBe(2) // Below min
		expect(policy.getConfirmationBlocks(1, new Decimal(100))).toBe(2) // At min
		expect(policy.getConfirmationBlocks(1, new Decimal(1000))).toBe(12) // At max
		expect(policy.getConfirmationBlocks(1, new Decimal(1500))).toBe(12) // Above max

		// Test interpolation at midpoint
		expect(policy.getConfirmationBlocks(1, new Decimal(550))).toBe(7) // Midpoint
	})

	it("Should handle multiple chains with different policies", () => {
		const policyConfig = {
			"1": {
				minAmount: "1000",
				maxAmount: "10000",
				minConfirmations: 3,
				maxConfirmations: 30,
			},
			"97": {
				minAmount: "10",
				maxAmount: "100",
				minConfirmations: 1,
				maxConfirmations: 5,
			},
		}

		const policy = new ConfirmationPolicy(policyConfig)

		expect(policy.getConfirmationBlocks(1, new Decimal(5500))).toBe(17) // Mainnet midpoint
		expect(policy.getConfirmationBlocks(97, new Decimal(55))).toBe(3) // BSC Chapel midpoint
	})

	it("Should handle decimal/floating point amounts correctly", () => {
		const policyConfig = {
			"1": {
				minAmount: "10.5",
				maxAmount: "100.75",
				minConfirmations: 2,
				maxConfirmations: 20,
			},
		}

		const policy = new ConfirmationPolicy(policyConfig)

		// Test boundaries with decimals
		expect(policy.getConfirmationBlocks(1, new Decimal(5.0))).toBe(2) // Below min
		expect(policy.getConfirmationBlocks(1, new Decimal(10.5))).toBe(2) // At min
		expect(policy.getConfirmationBlocks(1, new Decimal(100.75))).toBe(20) // At max
		expect(policy.getConfirmationBlocks(1, new Decimal(150.25))).toBe(20) // Above max

		// Test interpolation with decimals
		expect(policy.getConfirmationBlocks(1, new Decimal(55.625))).toBe(11)

		// Test with very precise decimals
		expect(policy.getConfirmationBlocks(1, new Decimal(33.1875))).toBe(7) // ~25% point
		expect(policy.getConfirmationBlocks(1, new Decimal(78.0625))).toBe(15) // ~75% point
	})

	it("Should throw error for unknown chainId", () => {
		const policyConfig = {
			"1": {
				minAmount: "100",
				maxAmount: "1000",
				minConfirmations: 2,
				maxConfirmations: 12,
			},
		}

		const policy = new ConfirmationPolicy(policyConfig)

		expect(() => policy.getConfirmationBlocks(999, new Decimal(500))).toThrow(
			"No confirmation policy found for chainId 999",
		)
	})
})

async function setUp() {
	const bscChapelId = "EVM-97"
	const mainnetId = "EVM-1"
	const gnosisChiadoId = "EVM-10200"
	const bscMainnet = "EVM-56"

	const chains = [bscMainnet, mainnetId, gnosisChiadoId, bscChapelId]

	// Load test configuration from TOML file
	const config = loadTestConfig()

	// Convert TOML chain configs to UserProvidedChainConfig format
	const testChainConfigs: UserProvidedChainConfig[] = config.chains.map((chain: any) => ({
		chainId: chain.chainId,
		rpcUrl: chain.rpcUrl,
	}))

	// Convert TOML filler config including CoinGecko
	const fillerConfigForService = config.filler.coingecko
		? {
				privateKey: config.filler.privateKey,
				maxConcurrentOrders: config.filler.maxConcurrentOrders,
				coingecko: config.filler.coingecko,
			}
		: undefined

	// Create the custom config service
	const chainConfigService = new FillerConfigService(testChainConfigs, fillerConfigForService)
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	// Use confirmation policies from TOML config
	const confirmationPolicy = new ConfirmationPolicy(config.confirmationPolicies)

	const fillerConfig: FillerConfig = {
		confirmationPolicy: {
			getConfirmationBlocks: (chainId: number, amountUsd: number) =>
				confirmationPolicy.getConfirmationBlocks(chainId, new Decimal(amountUsd)),
		},
		maxConcurrentOrders: config.filler.maxConcurrentOrders,
		pendingQueueConfig: config.filler.pendingQueue,
	}

	const chainClientManager = new ChainClientManager(chainConfigService, process.env.PRIVATE_KEY as HexString)
	const contractInteractionService = new ContractInteractionService(
		chainClientManager,
		process.env.PRIVATE_KEY as HexString,
		chainConfigService,
	)
	const bscWalletClient = chainClientManager.getWalletClient(bscChapelId)
	const gnosisChiadoWalletClient = chainClientManager.getWalletClient(gnosisChiadoId)
	const bscPublicClient = chainClientManager.getPublicClient(bscChapelId)
	const gnosisChiadoPublicClient = chainClientManager.getPublicClient(gnosisChiadoId)
	const intentGatewayAddress = chainConfigService.getChainConfig(bscChapelId).intentGatewayAddress
	const feeTokenBscAddress = (await contractInteractionService.getFeeTokenWithDecimals(bscChapelId)).address
	const feeTokenGnosisChiadoAddress = (await contractInteractionService.getFeeTokenWithDecimals(gnosisChiadoId))
		.address
	const bscIsmpHostAddress = "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7" as HexString
	const gnosisChiadoIsmpHostAddress = "0x58a41b89f4871725e5d898d98ef4bf917601c5eb" as HexString
	const bscHandlerAddress = "0x4638945E120846366cB7Abc08DB9c0766E3a663F" as HexString
	const gnosisChiadoHandlerAddress = "0x4638945E120846366cB7Abc08DB9c0766E3a663F" as HexString
	const bscIntentGateway = getContract({
		address: intentGatewayAddress as HexString,
		abi: INTENT_GATEWAY_ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const gnosisChiadoIntentGateway = getContract({
		address: intentGatewayAddress as HexString,
		abi: INTENT_GATEWAY_ABI,
		client: { public: gnosisChiadoPublicClient, wallet: gnosisChiadoWalletClient },
	})

	const bscIsmpHost = getContract({
		address: bscIsmpHostAddress,
		abi: EVM_HOST,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const gnosisChiadoIsmpHost = getContract({
		address: gnosisChiadoIsmpHostAddress,
		abi: EVM_HOST,
		client: { public: gnosisChiadoPublicClient, wallet: gnosisChiadoWalletClient },
	})

	const bscHandler = getContract({
		address: bscHandlerAddress,
		abi: HandlerV1_ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const gnosisChiadoHandler = getContract({
		address: gnosisChiadoHandlerAddress,
		abi: HandlerV1_ABI,
		client: { public: gnosisChiadoPublicClient, wallet: gnosisChiadoWalletClient },
	})

	const bscEvmHelper = new EvmChain({
		chainId: 97,
		host: bscIsmpHostAddress,
		url: process.env.BSC_CHAPEL!,
	})
	const gnosisChiadoEvmHelper = new EvmChain({
		chainId: 10200,
		host: gnosisChiadoIsmpHostAddress,
		url: process.env.GNOSIS_CHIADO!,
	})

	return {
		chainClientManager,
		bscWalletClient,
		gnosisChiadoWalletClient,
		bscPublicClient,
		gnosisChiadoPublicClient,
		bscIntentGateway,
		gnosisChiadoIntentGateway,
		bscIsmpHostAddress,
		gnosisChiadoIsmpHostAddress,
		bscIsmpHost,
		gnosisChiadoIsmpHost,
		feeTokenBscAddress,
		feeTokenGnosisChiadoAddress,
		contractInteractionService,
		bscHandler,
		bscChapelId,
		gnosisChiadoId,
		chainConfigService,
		fillerConfig,
		chainConfigs,
		gnosisChiadoHandler,
		bscEvmHelper,
		gnosisChiadoEvmHelper,
		mainnetId,
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

	if (approval == 0n) {
		console.log("Approving tokens for test")
		const tx = await walletClient.writeContract({
			abi: ERC20_ABI,
			address: tokenAddress,
			functionName: "approve",
			args: [spender, maxUint256],
			chain: walletClient.chain,
			account: walletClient.account!,
		})

		console.log("Approved tokens for test:", tx)
		// Wait for the 5 seconds, to make sure the transaction is mined
		await new Promise((resolve) => setTimeout(resolve, 5000))
	}
}

async function addLiquidity(
	walletClient: WalletClient,
	publicClient: PublicClient,
	uniswapRouterAddress: HexString,
	tokenA: HexString,
	tokenB: HexString,
	tokenAAmount: bigint,
	tokenBAmount: bigint,
	lpRecipient: HexString,
) {
	await approveTokens(walletClient, publicClient, tokenA, uniswapRouterAddress)
	await approveTokens(walletClient, publicClient, tokenB, uniswapRouterAddress)
	try {
		console.log("Adding liquidity to uniswap router:", uniswapRouterAddress)
		const tx = await walletClient.writeContract({
			abi: UNISWAP_ROUTER_V2_ABI,
			address: uniswapRouterAddress,
			functionName: "addLiquidity",
			args: [
				tokenA,
				tokenB,
				tokenAAmount,
				tokenBAmount,
				tokenAAmount - 1n,
				tokenBAmount - 1n,
				lpRecipient,
				6533729700n,
			],
			chain: walletClient.chain,
			account: walletClient.account!,
		})

		console.log("Added liquidity:", tx)
	} catch (error) {
		console.error("Error adding liquidity:", error)
		throw error
	}
}

async function getPairAddress(
	publicClient: PublicClient,
	tokenA: HexString,
	tokenB: HexString,
	factoryAddress: HexString,
) {
	const pairAddress = await publicClient.readContract({
		abi: parseAbi(["function getPair(address tokenA, address tokenB) view returns (address)"]),
		address: factoryAddress,
		functionName: "getPair",
		args: [tokenA, tokenB],
	})
	return pairAddress as HexString
}

async function checkIfOrderFilled(
	commitment: HexString,
	client: PublicClient,
	intentGatewayAddress: HexString,
): Promise<boolean> {
	try {
		const mappingSlot = 5n

		const slot = keccak256(encodePacked(["bytes32", "uint256"], [commitment, mappingSlot]))

		const filledStatus = await client.getStorageAt({
			address: intentGatewayAddress,
			slot: slot,
		})

		console.log("Filled status:", filledStatus)
		return filledStatus !== "0x0000000000000000000000000000000000000000000000000000000000000000"
	} catch (error) {
		console.error(`Error checking if order filled:`, error)
		return false
	}
}

async function clearOutputTokenBalance(
	walletClient: WalletClient,
	publicClient: PublicClient,
	tokenAddresses: HexString[],
	pairAddresses: HexString[],
) {
	// Send each token to the pair address from user
	for (const tokenAddress of tokenAddresses) {
		// First check if the balance is greater than 0
		const balance = await publicClient.readContract({
			abi: ERC20_ABI,
			address: tokenAddress,
			functionName: "balanceOf",
			args: [walletClient.account?.address as HexString],
		})

		if (balance > 0n) {
			console.log("Clearing balance of", tokenAddress, "from", walletClient.account?.address)
			const tx = await walletClient.writeContract({
				abi: ERC20_ABI,
				address: tokenAddress,
				functionName: "transfer",
				args: [pairAddresses[0], balance],
				chain: walletClient.chain,
				account: walletClient.account!,
			})
		}
	}
}
