import { IntentFiller } from "@/core/filler"
import { ChainClientManager, ChainConfigService, ContractInteractionService } from "@/services"
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
} from "hyperbridge-sdk"
import { describe, it, expect } from "vitest"
import { ConfirmationPolicy } from "@/config/confirmation-policy"
import {
	decodeFunctionData,
	encodePacked,
	getContract,
	hexToString,
	keccak256,
	maxUint256,
	parseEventLogs,
	PublicClient,
	WalletClient,
} from "viem"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet } from "viem/chains"
import "./setup"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { HandlerV1_ABI } from "@/config/abis/HandlerV1"
describe.sequential("Basic", () => {
	let intentFiller: IntentFiller
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

		const { intentFiller: intentFillerInstance } = await setUp()
		intentFiller = intentFillerInstance
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

			gnosisChiadoPublicClient,
		} = await setUp()

		intentFiller.start()

		const inputs: TokenInfo[] = [
			{
				token: "0x0000000000000000000000000000000000000000000000000000000000000000",
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
			value: 100n,
		})

		const receipt = await bscPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		console.log("Order placed on BSC:", receipt.transactionHash)

		console.log("Waiting for event monitor to detect the order...")
		const detectedOrder = await orderDetectedPromise
		console.log("Order successfully detected by event monitor:", detectedOrder)

		const orderFilledPromise = new Promise<string>((resolve) => {
			const eventMonitor = intentFiller.monitor
			if (!eventMonitor) {
				console.error("Event monitor not found on intentFiller")
				resolve("")
				return
			}

			eventMonitor.on("orderFilled", (data: { orderId: string }) => {
				console.log("Order filled by event monitor:", data.orderId)
				resolve(data.orderId)
			})
		})

		const orderFilledId = await orderFilledPromise
		console.log("Order filled:", orderFilledId)

		let isFilled = await checkIfOrderFilled(
			orderFilledId as HexString,
			gnosisChiadoPublicClient,
			gnosisChiadoIntentGateway.address,
		)

		expect(isFilled).toBe(true)

		console.log("Checking if order is filled at the source chain...")
		await new Promise((resolve) => setTimeout(resolve, 60 * 1000))

		isFilled = await checkIfOrderFilled(orderFilledId as HexString, bscPublicClient, bscIntentGateway.address)
		let maxAttempts = 20
		while (!isFilled && maxAttempts > 0) {
			console.log("Order not filled at the source chain, retrying storage check in 30 seconds...")
			console.log("Max storage checks left:", maxAttempts)
			await new Promise((resolve) => setTimeout(resolve, 30 * 1000))
			isFilled = await checkIfOrderFilled(orderFilledId as HexString, bscPublicClient, bscIntentGateway.address)
			maxAttempts--
		}

		expect(isFilled).toBe(true)

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
		} = await setUp()

		const inputs: TokenInfo[] = [
			{
				token: "0x0000000000000000000000000000000000000000000000000000000000000000",
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
			value: 100n,
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
})

async function setUp() {
	const bscChapelId = "EVM-97"
	const gnosisChiadoId = "EVM-10200"

	const chains = [bscChapelId, gnosisChiadoId]

	let chainConfigService = new ChainConfigService()
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	let strategies = [new BasicFiller(process.env.PRIVATE_KEY as HexString)]

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

	let intentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig)

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

	return {
		intentFiller,
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
