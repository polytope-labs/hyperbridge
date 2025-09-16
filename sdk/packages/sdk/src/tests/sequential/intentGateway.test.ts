import "log-timestamp"

import { createPublicClient, getContract, http } from "viem"
import { bsc, mainnet } from "viem/chains"
import { ChainConfig, type HexString, Order } from "@/types"
import { orderCommitment, hexToString, bytes20ToBytes32 } from "@/utils"
import EVM_HOST from "@/abis/evmHost"
import { EvmChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents"
import { ChainConfigService } from "@/configs/ChainConfigService"

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

		console.log("order", order)

		const {
			feeTokenAmount: estimatedFee,
			nativeTokenAmount,
			postRequestCalldata,
		} = await bscIntentGateway.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})
		console.log("Estimated fee:", estimatedFee)
		console.log("Native token amount:", nativeTokenAmount)
		console.log("Post request calldata:", postRequestCalldata)
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

		const { feeTokenAmount: estimatedFee } = await bscIntentGateway.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})

		console.log("BSC => Arbitrum")
		console.log("Order.fees", estimatedFee)

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

		const { feeTokenAmount: estimatedFee } = await baseIntentGateway.estimateFillOrder({
			...order,
			id: orderCommitment(order),
			destChain: hexToString(order.destChain as HexString),
			sourceChain: hexToString(order.sourceChain as HexString),
		})

		console.log("Base => BSC")
		console.log("Order.fees", estimatedFee)

		assert(estimatedFee > 0n)
	}, 1_000_000)
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
