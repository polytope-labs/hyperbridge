import "log-timestamp"

import {
	createPublicClient,
	createWalletClient,
	getContract,
	http,
	parseEventLogs,
	type PublicClient,
	type WalletClient,
	maxUint256,
} from "viem"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { bscTestnet, gnosisChiado } from "viem/chains"

import { IndexerClient } from "@/client"
import { ChainConfig, FillerConfig, type HexString, IPostRequest, Order, OrderStatus } from "@/types"
import { orderCommitment, hexToString, bytes20ToBytes32, constructRedeemEscrowRequestBody } from "@/utils"

import ERC6160 from "@/abis/erc6160"
import INTENT_GATEWAY_ABI from "@/abis/IntentGateway"
import EVM_HOST from "@/abis/evmHost"
import HANDLER from "@/abis/handler"
import { EvmChain, EvmChainParams, SubstrateChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents"
import { createQueryClient } from "@/query-client"
import { IntentFiller, BasicFiller, ConfirmationPolicy } from "@hyperbridge/filler"
import { ChainConfigService } from "@/configs/ChainConfigService"

describe.sequential(
	"Order Status Stream",
	() => {
		let indexer: IndexerClient
		let hyperbridgeInstance: SubstrateChain

		beforeAll(async () => {
			const { gnosisChiadoIsmpHost, bscIsmpHost, hyperbridge } = await setUp()

			const query_client = createQueryClient({
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
				queryClient: query_client,
				pollInterval: 1_000,
			})

			await hyperbridge.connect()
			hyperbridgeInstance = hyperbridge
		}, 10_000)

		it.skip("should successfully stream and query the order status", async () => {
			const {
				bscIntentGateway,
				bscWalletClient,
				bscPublicClient,
				bscIsmpHost,
				gnosisChiadoIsmpHost,
				bscFeeToken,
				chainConfigs,
				fillerConfig,
				chainConfigService,
				bscChapelId,
			} = await setUp()

			const strategies = [new BasicFiller(process.env.PRIVATE_KEY as HexString)]
			const intentFiller = new IntentFiller(chainConfigs, strategies, fillerConfig)
			intentFiller.start()

			const daiAsset = chainConfigService.getDaiAsset(bscChapelId)
			const inputs = [
				{
					token: bytes20ToBytes32(daiAsset),
					amount: 100n,
				},
			]
			const outputs = [
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

			await approveTokens(bscWalletClient, bscPublicClient, bscFeeToken.address, bscIntentGateway.address)

			const hash = await bscIntentGateway.write.placeOrder([order as any], {
				account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
				chain: bscTestnet,
			})

			const receipt = await bscPublicClient.waitForTransactionReceipt({
				hash,
				confirmations: 1,
			})

			console.log("Order placed on BSC:", receipt.transactionHash)

			const orderPlaceEvent = parseEventLogs({
				abi: INTENT_GATEWAY_ABI.ABI,
				logs: receipt.logs,
				strict: false,
			})[0] as { eventName: "OrderPlaced"; args: any }

			if (orderPlaceEvent.eventName !== "OrderPlaced") {
				throw new Error("Unexpected Event type")
			}

			const orderPlaced = orderPlaceEvent.args
			const commitment = orderCommitment({
				...orderPlaced,
				sourceChain: hexToString(orderPlaced.sourceChain),
				destChain: hexToString(orderPlaced.destChain),
				outputs: orderPlaced.outputs,
				inputs: orderPlaced.inputs,
			})

			console.log("Order Commitment:", commitment)

			for await (const status of indexer.orderStatusStream(commitment)) {
				console.log(
					JSON.stringify(status, (_, value) => (typeof value === "bigint" ? value.toString() : value), 4),
				)
				switch (status.status) {
					case OrderStatus.PLACED: {
						console.log(
							`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
						)
						break
					}
					case OrderStatus.FILLED: {
						console.log(
							`Status ${status.status}, Transaction: https://gnosis-chiado.blockscout.com/tx/${status.metadata.transactionHash}`,
						)
						console.log("Filled by:", status.metadata.filler)
						break
					}
					case OrderStatus.REDEEMED: {
						console.log(
							`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
						)
						break
					}
					case OrderStatus.REFUNDED: {
						console.log(
							`Status ${status.status}, Transaction: https://testnet.bscscan.com/tx/${status.metadata.transactionHash}`,
						)
						break
					}
				}
			}

			const orderStatus = await indexer.queryOrder(commitment)
			expect(orderStatus?.statuses.length).toBe(2)

			intentFiller.stop()
			await hyperbridgeInstance.disconnect()
		}, 1_000_000)

		it("should successfully get the quotes and swap estimates", async () => {
			const {
				bscIsmpHost,
				gnosisChiadoIsmpHost,
				chainConfigService,
				bscChapelId,
				bscPublicClient,
				gnosisChiadoId,
			} = await setUp()

			let bscEvmStructParams: EvmChainParams = {
				chainId: 97,
				host: "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7",
				url: process.env.BSC_CHAPEL!,
			}

			let gnosisChiadoEvmStructParams: EvmChainParams = {
				chainId: 10200,
				host: "0x58a41b89f4871725e5d898d98ef4bf917601c5eb",
				url: process.env.GNOSIS_CHIADO!,
			}

			let gnosisChiadoEvmChain = new EvmChain(gnosisChiadoEvmStructParams) // Source Chain
			let bscEvmChain = new EvmChain(bscEvmStructParams) // Destination Chain
			let intentGateway = new IntentGateway(gnosisChiadoEvmChain, bscEvmChain)

			let wrappedNativeTokenSourceChain = chainConfigService.getWrappedNativeAssetWithDecimals(gnosisChiadoId)

			let usdtAsset = chainConfigService.getUsdtAsset(bscChapelId)
			let daiAsset = chainConfigService.getDaiAsset(bscChapelId)
			let usdcAsset = chainConfigService.getUsdcAsset(bscChapelId)

			// Order

			let order: Order = {
				user: "0x0000000000000000000000000000000000000000000000000000000000000000" as HexString,
				sourceChain: await gnosisChiadoIsmpHost.read.host(),
				destChain: await bscIsmpHost.read.host(),
				deadline: 65337297000n,
				nonce: 0n,
				fees: 0n,
				outputs: [
					{
						token: bytes20ToBytes32(daiAsset),
						amount: 100n,
						beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E",
					},
				],
				inputs: [
					{
						token: "0x0000000000000000000000000000000000000000000000000000000000000000",
						amount: 100n,
					},
				],
				callData: "0x" as HexString,
			}

			console.log("order", order)

			let commitment = orderCommitment(order)
			order.id = commitment
			order.destChain = hexToString(order.destChain)
			order.sourceChain = hexToString(order.sourceChain)

			let fillerWalletAddress = privateKeyToAddress(process.env.PRIVATE_KEY as HexString)

			const postRequest: IPostRequest = {
				source: order.destChain, // Destination Chain
				dest: order.sourceChain, // Source Chain
				body: constructRedeemEscrowRequestBody(order, fillerWalletAddress),
				timeoutTimestamp: 0n,
				nonce: await gnosisChiadoEvmChain.getHostNonce(),
				from: chainConfigService.getIntentGatewayAddress(order.destChain),
				to: chainConfigService.getIntentGatewayAddress(order.sourceChain),
			}

			let postGasEstimate = await gnosisChiadoEvmChain.estimateGas(postRequest) // Source Chain Post Estimate

			console.log("Post gas estimate:", postGasEstimate)

			assert(postGasEstimate > 0n)

			let gasEstimate = await intentGateway.estimateFillOrder(order)

			console.log("Gas estimate for fill order:", gasEstimate)

			assert(gasEstimate > 160000n)

			let initialAmountIn = 100n

			let bestQuoteWithAmountOut = await intentGateway.findBestProtocolWithAmountIn(
				order.destChain,
				daiAsset,
				usdtAsset,
				initialAmountIn,
			)

			console.log("Best quote with amount out:", bestQuoteWithAmountOut)

			assert(bestQuoteWithAmountOut.amountOut > 0n)

			let bestQuoteWithAmountIn = await intentGateway.findBestProtocolWithAmountOut(
				order.destChain,
				usdtAsset,
				daiAsset,
				bestQuoteWithAmountOut.amountOut,
			)

			console.log("Best quote with amount in:", bestQuoteWithAmountIn)

			assert(bestQuoteWithAmountIn.amountIn === initialAmountIn)

			// Order filled checker
			const filledOrderCommitment =
				"0x1dede1bc4939f194e8a06a9086377d1e64c5c1c77c055e4430ff7141c774528c" as HexString
			let isFilled = await intentGateway.isOrderFilled(order)

			assert(isFilled === false)

			// Create a mock order with the filled commitment for testing
			let filledOrder = { ...order, id: filledOrderCommitment }
			isFilled = await intentGateway.isOrderFilled(filledOrder)

			assert(isFilled === true)
		})
	},
	1_000_000,
)

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

	const account = privateKeyToAccount(process.env.PRIVATE_KEY as any)

	const bscWalletClient = createWalletClient({
		chain: bscTestnet,
		account,
		transport: http(process.env.BSC_CHAPEL),
	})

	const gnosisChiadoWallet = createWalletClient({
		chain: gnosisChiado,
		account,
		transport: http(process.env.GNOSIS_CHIADO),
	})

	const bscPublicClient = createPublicClient({
		chain: bscTestnet,
		transport: http(process.env.BSC_CHAPEL),
	})

	const gnosisChiadoPublicClient = createPublicClient({
		chain: gnosisChiado,
		transport: http(process.env.GNOSIS_CHIADO),
	})

	const bscIntentGateway = getContract({
		address: chainConfigService.getIntentGatewayAddress(bscChapelId),
		abi: INTENT_GATEWAY_ABI.ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const gnosisChiadoIntentGateway = getContract({
		address: chainConfigService.getIntentGatewayAddress(gnosisChiadoId),
		abi: INTENT_GATEWAY_ABI.ABI,
		client: { public: gnosisChiadoPublicClient, wallet: gnosisChiadoWallet },
	})

	const bscIsmpHostAddress = "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7" as HexString
	const gnosisChiadoIsmpHostAddress = "0x58a41b89f4871725e5d898d98ef4bf917601c5eb" as HexString

	const bscIsmpHost = getContract({
		address: bscIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: bscPublicClient,
	})

	const gnosisChiadoIsmpHost = getContract({
		address: gnosisChiadoIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: gnosisChiadoPublicClient,
	})

	const bscHostParams = await bscIsmpHost.read.hostParams()

	const bscHandler = getContract({
		address: bscHostParams.handler,
		abi: HANDLER.ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const bscFeeToken = getContract({
		address: bscHostParams.feeToken,
		abi: ERC6160.ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	const hyperbridge = new SubstrateChain({
		ws: process.env.HYPERBRIDGE_GARGANTUA!,
		hasher: "Keccak",
	})

	return {
		bscPublicClient,
		bscFeeToken,
		account,
		gnosisChiadoPublicClient,
		bscHandler,
		bscIntentGateway,
		gnosisChiadoIntentGateway,
		bscIsmpHost,
		gnosisChiadoIsmpHost,
		hyperbridge,
		chainConfigs,
		fillerConfig,
		chainConfigService,
		bscChapelId,
		bscWalletClient,
		gnosisChiadoId,
	}
}

export async function approveTokens(
	walletClient: WalletClient,
	publicClient: PublicClient,
	tokenAddress: HexString,
	spender: HexString,
) {
	const approval = await publicClient.readContract({
		abi: ERC6160.ABI,
		address: tokenAddress,
		functionName: "allowance",
		args: [walletClient.account?.address as HexString, spender],
		account: walletClient.account,
	})

	if (approval == 0n) {
		console.log("Approving tokens for test")
		const tx = await walletClient.writeContract({
			abi: ERC6160.ABI,
			address: tokenAddress,
			functionName: "approve",
			args: [spender, maxUint256],
			chain: walletClient.chain,
			account: walletClient.account!,
		})

		const receipt = await publicClient.waitForTransactionReceipt({ hash: tx })
		console.log("Approved tokens for test:", receipt)
	}
}
