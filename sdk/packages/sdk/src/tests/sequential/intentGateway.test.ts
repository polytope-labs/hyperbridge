import "log-timestamp"

import {
	createPublicClient,
	createWalletClient,
	getContract,
	http,
	maxUint256,
	parseEventLogs,
	PublicClient,
	toHex,
	WalletClient,
} from "viem"
import { bsc, bscTestnet, mainnet, sepolia } from "viem/chains"
import {
	ChainConfig,
	FillerConfig,
	type HexString,
	IGetRequest,
	IHyperbridgeConfig,
	Order,
	TokenInfo,
	PaymentInfo,
} from "@/types"
import { orderCommitment, hexToString, bytes20ToBytes32, DEFAULT_GRAFFITI } from "@/utils"
import EVM_HOST from "@/abis/evmHost"
import { EvmChain, EvmChainParams, SubstrateChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { privateKeyToAccount } from "viem/accounts"
import IntentGatewayABI from "@/abis/IntentGateway"
import erc6160 from "@/abis/erc6160"
import handler from "@/abis/handler"
import { IndexerClient } from "@/client"
import { createQueryClient } from "@/query-client"

describe.sequential("Intents protocol tests", () => {
	it.skip("Should generate the estimatedFee while doing bsc mainnet to eth mainnet", async () => {
		const { chainConfigService, bscMainnetIsmpHost, mainnetIsmpHost } = await setUp()
		const bscMainnetId = "EVM-56"
		const mainnetId = "EVM-1"
		const bscEvmChain = new EvmChain({
			chainId: 56,
			host: chainConfigService.getHostAddress(bscMainnetId),
			url: process.env.BSC_MAINNET!,
		})
		const mainnetEvmChain = new EvmChain({
			chainId: 1,
			host: chainConfigService.getHostAddress(mainnetId),
			url: process.env.ETH_MAINNET!,
		})

		const bscIntentGateway = new IntentGateway(bscEvmChain, mainnetEvmChain)

		const bscUsdcAsset = chainConfigService.getUsdcAsset(bscMainnetId)
		const mainnetUsdcAsset = chainConfigService.getUsdcAsset(mainnetId)

		const order: Order = {
			user: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
			sourceChain: await bscMainnetIsmpHost.read.host(),
			destChain: await mainnetIsmpHost.read.host(),
			deadline: 65337297000n,
			nonce: 0n,
			fees: 0n,
			outputs: [
				{
					token: bytes20ToBytes32(mainnetUsdcAsset),
					amount: 100n,
					beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
				},
			],
			inputs: [
				{
					token: bytes20ToBytes32(bscUsdcAsset),
					amount: 100n,
				},
			],
			callData: "0x" as HexString,
		}

		const { feeTokenAmount: estimatedFee, nativeTokenAmount } = await bscIntentGateway.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})
		console.log("BSC => ETH")
		console.log("Estimated fee:", estimatedFee)
		console.log("Native token amount:", nativeTokenAmount)

		assert(estimatedFee > 0n)
	}, 1_000_000)

	it.skip("Should generate the estimatedFee while doing bsc mainnet to arbitrum mainnet", async () => {
		const { chainConfigService, bscMainnetIsmpHost, arbitrumMainnetIsmpHost } = await setUpBscToArbitrum()
		const bscMainnetId = "EVM-56"
		const arbitrumMainnetId = "EVM-42161"
		const bscEvmChain = new EvmChain({
			chainId: 56,
			host: chainConfigService.getHostAddress(bscMainnetId),
			url: process.env.BSC_MAINNET!,
		})
		const arbitrumEvmChain = new EvmChain({
			chainId: 42161,
			host: chainConfigService.getHostAddress(arbitrumMainnetId),
			url: process.env.ARBITRUM_MAINNET!,
		})

		const bscIntentGateway = new IntentGateway(bscEvmChain, arbitrumEvmChain)

		const bscUsdcAsset = chainConfigService.getUsdcAsset(bscMainnetId)
		const arbitrumUsdcAsset = chainConfigService.getUsdcAsset(arbitrumMainnetId)

		const order: Order = {
			user: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
			sourceChain: await bscMainnetIsmpHost.read.host(),
			destChain: await arbitrumMainnetIsmpHost.read.host(),
			deadline: 65337297000n,
			nonce: 0n,
			fees: 0n,
			outputs: [
				{
					token: bytes20ToBytes32(arbitrumUsdcAsset),
					amount: 100n,
					beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
				},
			],
			inputs: [
				{
					token: bytes20ToBytes32(bscUsdcAsset),
					amount: 100n,
				},
			],
			callData: "0x" as HexString,
		}

		const { feeTokenAmount: estimatedFee, nativeTokenAmount } = await bscIntentGateway.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})

		console.log("BSC => Arbitrum")
		console.log("Estimated Fee", estimatedFee)
		console.log("Native Token Amount", nativeTokenAmount)

		assert(estimatedFee > 0n)
	}, 1_000_000)

	it.skip("Should generate the estimatedFee while doing base mainnet to bsc mainnet", async () => {
		const { chainConfigService, baseMainnetIsmpHost, bscMainnetIsmpHost } = await setUpBaseToBsc()
		const baseMainnetId = "EVM-8453"
		const bscMainnetId = "EVM-56"
		const baseEvmChain = new EvmChain({
			chainId: 8453,
			host: chainConfigService.getHostAddress(baseMainnetId),
			url: process.env.BASE_MAINNET!,
		})
		const bscEvmChain = new EvmChain({
			chainId: 56,
			host: chainConfigService.getHostAddress(bscMainnetId),
			url: process.env.BSC_MAINNET!,
		})

		const baseIntentGateway = new IntentGateway(baseEvmChain, bscEvmChain)

		const baseUsdcAsset = chainConfigService.getUsdcAsset(baseMainnetId)
		const bscUsdcAsset = chainConfigService.getUsdcAsset(bscMainnetId)

		const order: Order = {
			user: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
			sourceChain: await baseMainnetIsmpHost.read.host(),
			destChain: await bscMainnetIsmpHost.read.host(),
			deadline: 65337297000n,
			nonce: 0n,
			fees: 0n,
			outputs: [
				{
					token: bytes20ToBytes32(bscUsdcAsset),
					amount: 100n,
					beneficiary: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
				},
			],
			inputs: [
				{
					token: bytes20ToBytes32(baseUsdcAsset),
					amount: 100n,
				},
			],
			callData: "0x" as HexString,
		}

		const { feeTokenAmount: estimatedFee, nativeTokenAmount } = await baseIntentGateway.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})

		console.log("Base => BSC")
		console.log("Estimated Fee", estimatedFee)
		console.log("Native Token Amount", nativeTokenAmount)

		assert(estimatedFee > 0n)
	}, 1_000_000)
})

describe("Order Cancellation tests", () => {
	let indexer: IndexerClient
	let hyperbridgeInstance: SubstrateChain

	beforeAll(async () => {
		const { bscChapelIsmpHost, ethSepoliaIsmpHost, hyperbridge } = await setUpBscToSepoliaOrder()

		const query_client = createQueryClient({
			url: process.env.INDEXER_URL!,
		})

		indexer = new IndexerClient({
			source: {
				consensusStateId: "BSC0",
				rpcUrl: process.env.BSC_CHAPEL!,
				stateMachineId: "EVM-97",
				host: bscChapelIsmpHost.address,
			},
			dest: {
				consensusStateId: "ETH0",
				rpcUrl: process.env.SEPOLIA!,
				stateMachineId: "EVM-11155111",
				host: ethSepoliaIsmpHost.address,
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

	it("Should cancel order when deadline has reached and yield the necessary proofs", async () => {
		const {
			bscChapelId,
			chainConfigService,
			bscChapelIntentGateway,
			feeTokenBscChapelAddress,
			bscChapelWalletClient,
			bscChapelPublicClient,
			bscChapelIsmpHost,
			ethSepoliaIsmpHost,
		} = await setUpBscToSepoliaOrder()

		let bscChapelEvmStructParams: EvmChainParams = {
			chainId: 97,
			host: "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7",
			url: process.env.BSC_CHAPEL!,
		}

		let ethSepoliaEvmStructParams: EvmChainParams = {
			chainId: 11155111,
			host: "0x2EdB74C269948b60ec1000040E104cef0eABaae8",
			url: process.env.SEPOLIA!,
		}

		let bscEvmChain = new EvmChain(bscChapelEvmStructParams) // Source Chain
		let ethSepoliaEvmChain = new EvmChain(ethSepoliaEvmStructParams) // Dest Chain
		let intentGateway = new IntentGateway(bscEvmChain, ethSepoliaEvmChain)

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
			sourceChain: await bscChapelIsmpHost.read.host(),
			destChain: await ethSepoliaIsmpHost.read.host(),
			deadline: 0n, // Expired deadline
			nonce: 0n,
			fees: 1000000n,
			outputs,
			inputs,
			callData: "0x" as HexString,
		}

		await approveTokens(
			bscChapelWalletClient,
			bscChapelPublicClient,
			feeTokenBscChapelAddress,
			bscChapelIntentGateway.address,
		)

		let hash = await bscChapelIntentGateway.write.placeOrder([order, DEFAULT_GRAFFITI], {
			account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
			chain: bscTestnet,
		})

		let receipt = await bscChapelPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		const orderPlaceEvent = parseEventLogs({ abi: IntentGatewayABI.ABI, logs: receipt.logs })[0]

		if (orderPlaceEvent.eventName !== "OrderPlaced") {
			throw new Error("Unexpected Event type")
		}

		const orderPlaced = orderPlaceEvent.args

		console.log("Order placed on BSC:", orderPlaced)

		const hyperbridgeConfig: IHyperbridgeConfig = {
			wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			consensusStateId: "PAS0",
			stateMachineId: "KUSAMA-4009",
		}

		const cancelGenerator = intentGateway.cancelOrder(order, hyperbridgeConfig, indexer)

		let result = await cancelGenerator.next()
		while (!result.done && result.value?.status !== "DESTINATION_FINALIZED") {
			console.log("Status:", result.value?.status)
			result = await cancelGenerator.next()
		}

		expect(result.value?.status).toBe("DESTINATION_FINALIZED")
		expect((result.value as any).data).toBeDefined()

		const finalizedHeight = (result.value as any).data.height as bigint
		console.log("DESTINATION_FINALIZED height:", finalizedHeight)

		result = await cancelGenerator.next()

		expect(result.value?.status).toBe("STATE_PROOF_RECEIVED")
		expect((result.value as any).data?.height).toBe(finalizedHeight)
		expect((result.value as any).data?.proof).toBeDefined()

		result = await cancelGenerator.next()

		expect(result.done).toBe(false)
		expect(result.value?.status).toBe("AWAITING_GET_REQUEST")

		const cancelOptions = {
			relayerFee: 10000000000n,
			height: finalizedHeight,
		}

		hash = await bscChapelIntentGateway.write.cancelOrder([orderPlaced, cancelOptions], {
			account: privateKeyToAccount(process.env.PRIVATE_KEY as HexString),
			chain: bscTestnet,
		})

		receipt = await bscChapelPublicClient.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		console.log("Order cancelled on BSC:", receipt.transactionHash)

		// parse EvmHost GetRequestEvent emitted in the transaction logs
		const event = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })[0]

		if (event.eventName !== "GetRequestEvent") {
			throw new Error("Unexpected Event type")
		}

		const { source, dest, from, nonce, height, keys, timeoutTimestamp, context } = event.args

		const getRequest: IGetRequest = {
			source,
			dest,
			from,
			nonce,
			height,
			keys: Array.from(keys),
			timeoutTimestamp,
			context,
		}

		console.log("Get Request:", getRequest)

		// Resume generator with the GetRequest and continue until finalized
		result = await cancelGenerator.next(getRequest)

		console.log("Result after SOURCE FINALIZED:", result)

		while (!result.done) {
			console.log("Status:", result.value?.status)

			if (result.value?.status === "HYPERBRIDGE_FINALIZED") {
				if ("metadata" in result.value && result.value.metadata) {
					console.log(
						`Status ${result.value.status}, Transaction: https://sepolia.etherscan.io/tx/${result.value.metadata.transactionHash}`,
					)
				}
				break
			}

			result = await cancelGenerator.next()
		}

		console.log("Generator completed:", result)
	}, 1_000_0000)
})

async function setUp() {
	const bscMainnetId = "EVM-56"
	const mainnetId = "EVM-1"
	const chains = [bscMainnetId, mainnetId]

	let chainConfigService = new ChainConfigService()
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const bscMainnetPublicClient = createPublicClient({
		chain: bsc,
		transport: http(process.env.BSC_MAINNET!),
	})

	const mainnetPublicClient = createPublicClient({
		chain: mainnet,
		transport: http(process.env.ETH_MAINNET!),
	})

	const bscMainnetIsmpHostAddress = "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7" as HexString
	const mainnetIsmpHostAddress = "0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20" as HexString

	const bscMainnetIsmpHost = getContract({
		address: bscMainnetIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: bscMainnetPublicClient,
	})

	const mainnetIsmpHost = getContract({
		address: mainnetIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: mainnetPublicClient,
	})

	return {
		chainConfigs,
		chainConfigService,
		bscMainnetIsmpHost,
		mainnetIsmpHost,
	}
}

async function setUpBscToArbitrum() {
	const bscMainnetId = "EVM-56"
	const arbitrumMainnetId = "EVM-42161"
	const chains = [bscMainnetId, arbitrumMainnetId]

	let chainConfigService = new ChainConfigService()
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const bscMainnetPublicClient = createPublicClient({
		chain: bsc,
		transport: http(process.env.BSC_MAINNET!),
	})

	const arbitrumPublicClient = createPublicClient({
		chain: {
			id: 42161,
			name: "arbitrum",
			nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
			rpcUrls: { default: { http: [process.env.ARBITRUM_MAINNET!] } },
		},
		transport: http(process.env.ARBITRUM_MAINNET!),
	})

	const bscMainnetIsmpHostAddress = "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7" as HexString
	const arbitrumIsmpHostAddress = "0xE05AFD4Eb2ce6d65c40e1048381BD0Ef8b4B299e" as HexString

	const bscMainnetIsmpHost = getContract({
		address: bscMainnetIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: bscMainnetPublicClient,
	})

	const arbitrumMainnetIsmpHost = getContract({
		address: arbitrumIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: arbitrumPublicClient,
	})

	return {
		chainConfigs,
		chainConfigService,
		bscMainnetIsmpHost,
		arbitrumMainnetIsmpHost,
	}
}

async function setUpBaseToBsc() {
	const baseMainnetId = "EVM-8453"
	const bscMainnetId = "EVM-56"
	const chains = [baseMainnetId, bscMainnetId]

	let chainConfigService = new ChainConfigService()
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const basePublicClient = createPublicClient({
		chain: {
			id: 8453,
			name: "base",
			nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
			rpcUrls: { default: { http: [process.env.BASE_MAINNET!] } },
		},
		transport: http(process.env.BASE_MAINNET!),
	})

	const bscMainnetPublicClient = createPublicClient({
		chain: bsc,
		transport: http(process.env.BSC_MAINNET!),
	})

	const baseIsmpHostAddress = "0x6FFe92e4d7a9D589549644544780e6725E84b248" as HexString
	const bscMainnetIsmpHostAddress = "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7" as HexString

	const baseMainnetIsmpHost = getContract({
		address: baseIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: basePublicClient,
	})

	const bscMainnetIsmpHost = getContract({
		address: bscMainnetIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: bscMainnetPublicClient,
	})

	return {
		chainConfigs,
		chainConfigService,
		baseMainnetIsmpHost,
		bscMainnetIsmpHost,
	}
}

async function setUpBscToSepoliaOrder() {
	const bscChapelId = "EVM-97"
	const ethSepoliaId = "EVM-11155111"

	const chains = [bscChapelId, ethSepoliaId]

	let chainConfigService = new ChainConfigService()
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const account = privateKeyToAccount(process.env.PRIVATE_KEY as any)

	const bscChapelWalletClient = createWalletClient({
		chain: bscTestnet,
		account,
		transport: http(process.env.BSC_CHAPEL),
	})

	const bscChapelPublicClient = createPublicClient({
		chain: bscTestnet,
		transport: http(process.env.BSC_CHAPEL),
	})

	const ethSepoliaPublicClient = createPublicClient({
		chain: sepolia,
		transport: http(process.env.SEPOLIA!),
	})

	const bscChapelIntentGateway = getContract({
		address: chainConfigService.getIntentGatewayAddress(bscChapelId),
		abi: IntentGatewayABI.ABI,
		client: { public: bscChapelPublicClient, wallet: bscChapelWalletClient },
	})

	const bscChapelIsmpHostAddress = "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7" as HexString
	const ethSepoliaIsmpHostAddress = "0x2EdB74C269948b60ec1000040E104cef0eABaae8" as HexString

	const bscChapelIsmpHost = getContract({
		address: bscChapelIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: bscChapelPublicClient,
	})

	const ethSepoliaIsmpHost = getContract({
		address: ethSepoliaIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: ethSepoliaPublicClient,
	})

	const bscChapelHostParams = await bscChapelIsmpHost.read.hostParams()

	const bscChapelHandler = getContract({
		address: bscChapelHostParams.handler,
		abi: handler.ABI,
		client: { public: bscChapelPublicClient, wallet: bscChapelWalletClient },
	})

	const bscChapelFeeToken = getContract({
		address: bscChapelHostParams.feeToken,
		abi: erc6160.ABI,
		client: { public: bscChapelPublicClient, wallet: bscChapelWalletClient },
	})

	const hyperbridge = new SubstrateChain({
		ws: process.env.HYPERBRIDGE_GARGANTUA!,
		hasher: "Keccak",
	})

	const feeTokenBscChapelAddress = bscChapelHostParams.feeToken

	return {
		account,
		hyperbridge,
		chainConfigs,
		chainConfigService,
		bscChapelId,
		ethSepoliaIsmpHost,
		bscChapelIntentGateway,
		bscChapelWalletClient,
		bscChapelPublicClient,
		feeTokenBscChapelAddress,
		bscChapelIsmpHost,
		bscChapelFeeToken,
		bscChapelHandler,
	}
}

export async function approveTokens(
	walletClient: WalletClient,
	publicClient: PublicClient,
	tokenAddress: HexString,
	spender: HexString,
) {
	const approval = await publicClient.readContract({
		abi: erc6160.ABI,
		address: tokenAddress,
		functionName: "allowance",
		args: [walletClient.account?.address as HexString, spender],
		account: walletClient.account,
	})

	if (approval == 0n) {
		console.log("Approving tokens for test")
		const tx = await walletClient.writeContract({
			abi: erc6160.ABI,
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
