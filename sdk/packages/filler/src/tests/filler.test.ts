import { IntentFiller } from "@/core/filler"
import { ChainClientManager, ChainConfigService, ContractInteractionService } from "@/services"
import { BasicFiller } from "@/strategies/basic"
import { StableSwapFiller } from "@/strategies/swap"
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
	bytes32ToBytes20,
	postRequestCommitment,
} from "hyperbridge-sdk"
import { describe, it, expect } from "vitest"
import { ConfirmationPolicy } from "@/config/confirmation-policy"
import {
	decodeFunctionData,
	encodePacked,
	getContract,
	hexToBigInt,
	hexToString,
	keccak256,
	maxUint256,
	parseEventLogs,
	PublicClient,
	toHex,
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
import { UNISWAP_V2_FACTORY_ABI } from "@/config/abis/UniswapV2Factory"
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
	})

	it("Should listen, place order, fill order, and check if filled at the source chain", async () => {
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
			bscHandler,
			chainConfigService,
			bscChapelId,
		} = await setUp()

		const strategies = [new BasicFiller(process.env.PRIVATE_KEY as HexString)]
		const intentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig)
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

		const order = {
			user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
			sourceChain: await bscIsmpHost.read.host(),
			destChain: await gnosisChiadoIsmpHost.read.host(),
			deadline: 65337297n,
			nonce: 0n,
			fees: 1000000n,
			outputs,
			inputs,
			callData: "0x" as HexString,
		}

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

		const hash = await bscIntentGateway.write.placeOrder([order], {
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

	it("Should timeout if order deadline is reached", async () => {
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

		let hash = await bscIntentGateway.write.placeOrder([order], {
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

	it("Should handle order filling with token swaps", async () => {
		const {
			bscIntentGateway,
			gnosisChiadoIntentGateway,
			bscPublicClient,
			bscIsmpHost,
			gnosisChiadoIsmpHost,
			gnosisChiadoPublicClient,
			gnosisChiadoWalletClient,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			feeTokenGnosisChiadoAddress,
			gnosisChiadoId,
			bscWalletClient,
			gnosisChiadoHandler,
			bscChapelId,
		} = await setUp()

		// Create a new intent filler with StableSwapFiller strategy
		const strategies = [new StableSwapFiller(process.env.PRIVATE_KEY as HexString)]
		const newIntentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig)

		newIntentFiller.start()

		const usdtAsset = chainConfigService.getUsdtAsset(bscChapelId)
		const daiAsset = chainConfigService.getDaiAsset(bscChapelId)

		// Create an order that requires token swaps
		const inputs: TokenInfo[] = [
			{
				token: "0x0000000000000000000000000000000000000000000000000000000000000000",
				amount: 100n,
			},
		]

		const outputs: PaymentInfo[] = [
			{
				token: bytes20ToBytes32(usdtAsset),
				amount: 1n,
				beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
			},
		]

		const order = {
			user: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
			sourceChain: await gnosisChiadoIsmpHost.read.host(),
			destChain: await bscIsmpHost.read.host(),
			deadline: 6533729700n,
			nonce: 0n,
			fees: 1000000n,
			outputs,
			inputs,
			callData: "0x" as HexString,
		}

		// Approve tokens for the order
		await approveTokens(
			gnosisChiadoWalletClient,
			gnosisChiadoPublicClient,
			feeTokenGnosisChiadoAddress,
			gnosisChiadoIntentGateway.address,
		)

		const pairAddress = await getPairAddress(
			bscPublicClient,
			daiAsset,
			usdtAsset,
			chainConfigService.getUniswapV2FactoryAddress(bscChapelId),
		)

		if (pairAddress === ADDRESS_ZERO) {
			console.log("Pair address is zero, creating pair and adding liquidity")
			await addLiquidity(
				bscWalletClient,
				bscPublicClient,
				chainConfigService.getUniswapRouterV2Address(bscChapelId),
				daiAsset,
				usdtAsset,
				100000000n,
				100000000n,
				privateKeyToAddress(process.env.PRIVATE_KEY as HexString),
			)
		}

		// Monitor for order detection
		const orderDetectedPromise = new Promise<Order>((resolve) => {
			const eventMonitor = newIntentFiller.monitor
			if (!eventMonitor) {
				console.error("Event monitor not found on intentFiller")
				resolve({} as Order)
				return
			}

			eventMonitor.on("newOrder", (data: { order: Order }) => {
				resolve(data.order)
			})
		})

		// Place the order
		const hash = await gnosisChiadoIntentGateway.write.placeOrder([order], {
			account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
			chain: gnosisChiado,
			value: 100n,
		})

		const receipt = await gnosisChiadoPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		console.log("Order placed on BSC:", receipt.transactionHash)

		// Wait for order detection
		console.log("Waiting for event monitor to detect the order...")
		const detectedOrder = await orderDetectedPromise
		console.log("Order successfully detected by event monitor:", detectedOrder)

		// Monitor for order filling
		const orderFilledPromise = new Promise<{ orderId: string; hash: string }>((resolve) => {
			const eventMonitor = newIntentFiller.monitor
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
						`Status ${status.status}, Transaction: https://gnosis-chiado.blockscout.com/tx/${status.metadata.transactionHash}`,
					)

					// Check if the order is filled at the source chain
					const isFilled = await checkIfOrderFilled(
						orderId as HexString,
						gnosisChiadoPublicClient,
						gnosisChiadoIntentGateway.address,
					)
					expect(isFilled).toBe(true)

					await clearOutputTokenBalance(bscWalletClient, bscPublicClient, [usdtAsset], [pairAddress])
					break
				}
			}
		}

		newIntentFiller.stop()
	}, 1_000_000)
})

async function setUp() {
	const bscChapelId = "EVM-97"

	const gnosisChiadoId = "EVM-10200"

	const chains = [bscChapelId, gnosisChiadoId]

	let chainConfigService = new ChainConfigService()
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const confirmationPolicy = new ConfirmationPolicy({
		"97": {
			minAmount: "1000000000000000000", // 1 token
			maxAmount: "1000000000000000000000", // 1000 tokens
			minConfirmations: 1,
			maxConfirmations: 5,
		},
		"10200": {
			minAmount: "1000000000000000000", // 1 token
			maxAmount: "1000000000000000000000", // 1000 tokens
			minConfirmations: 1,
			maxConfirmations: 5,
		},
	})

	const fillerConfig: FillerConfig = {
		confirmationPolicy: {
			getConfirmationBlocks: (chainId: number, amount: bigint) =>
				confirmationPolicy.getConfirmationBlocks(chainId, BigInt(amount)),
		},
		maxConcurrentOrders: 5,
		pendingQueueConfig: {
			maxRechecks: 10,
			recheckDelayMs: 30000,
		},
	}

	const chainClientManager = new ChainClientManager(process.env.PRIVATE_KEY as HexString)
	const contractInteractionService = new ContractInteractionService(
		chainClientManager,
		process.env.PRIVATE_KEY as HexString,
	)
	const bscWalletClient = chainClientManager.getWalletClient(bscChapelId)
	const gnosisChiadoWalletClient = chainClientManager.getWalletClient(gnosisChiadoId)
	const bscPublicClient = chainClientManager.getPublicClient(bscChapelId)
	const gnosisChiadoPublicClient = chainClientManager.getPublicClient(gnosisChiadoId)
	const intentGatewayAddress = chainConfigService.getChainConfig(bscChapelId).intentGatewayAddress
	const feeTokenBscAddress = (await contractInteractionService.getHostParams(bscChapelId)).feeToken
	const feeTokenGnosisChiadoAddress = (await contractInteractionService.getHostParams(gnosisChiadoId)).feeToken
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
		abi: UNISWAP_V2_FACTORY_ABI,
		address: factoryAddress,
		functionName: "getPair",
		args: [tokenA, tokenB],
	})

	console.log("Pair address:", pairAddress)
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
