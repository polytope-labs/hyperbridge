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
	encodeFunctionData,
	decodeFunctionResult,
	parseUnits,
} from "viem"
import { bsc, bscTestnet, mainnet, polygon, sepolia } from "viem/chains"
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
import {
	orderCommitment,
	hexToString,
	bytes20ToBytes32,
	bytes32ToBytes20,
	DEFAULT_GRAFFITI,
	getStorageSlot,
	ADDRESS_ZERO,
	ERC20Method,
	getGasPriceFromEtherscan,
} from "@/utils"
import EVM_HOST from "@/abis/evmHost"
import { EvmChain, EvmChainParams, IProof, SubstrateChain } from "@/chain"
import { IntentGateway } from "@/protocols/intents"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import IntentGatewayABI from "@/abis/IntentGateway"
import erc6160 from "@/abis/erc6160"
import handler from "@/abis/handler"
import { PERMIT2_ABI } from "@/abis/permit2"
import universalRouter from "@/abis/universalRouter"
import { IndexerClient } from "@/client"
import { createQueryClient } from "@/query-client"
import { strict as assert } from "assert"

describe.sequential("Intents protocol tests", () => {
	it("Should generate the estimatedFee while doing bsc mainnet to eth mainnet", async () => {
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

	it("Should generate the estimatedFee while doing bsc mainnet to arbitrum mainnet", async () => {
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

	it("Should generate the estimatedFee while doing base mainnet to bsc mainnet", async () => {
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

	it("Should generate the estimatedFee while doing bsc mainnet to polygon mainnet", async () => {
		const { chainConfigService, bscMainnetIsmpHost, polygonMainnetIsmpHost } = await setUpBscToPolygon()
		const bscMainnetId = "EVM-56"
		const polygonMainnetId = "EVM-137"
		const bscEvmChain = new EvmChain({
			chainId: 56,
			host: chainConfigService.getHostAddress(bscMainnetId),
			url: process.env.BSC_MAINNET!,
		})
		const polygonEvmChain = new EvmChain({
			chainId: 137,
			host: chainConfigService.getHostAddress(polygonMainnetId),
			url: process.env.POLYGON_MAINNET!,
		})

		const bscIntentGateway = new IntentGateway(bscEvmChain, polygonEvmChain)

		const bscUsdcAsset = chainConfigService.getUsdcAsset(bscMainnetId)
		const polygonUsdcAsset = chainConfigService.getUsdcAsset(polygonMainnetId)

		const order: Order = {
			user: "0x000000000000000000000000Ea4f68301aCec0dc9Bbe10F15730c59FB79d237E" as HexString,
			sourceChain: await bscMainnetIsmpHost.read.host(),
			destChain: await polygonMainnetIsmpHost.read.host(),
			deadline: 65337297000n,
			nonce: 0n,
			fees: 0n,
			outputs: [
				{
					token: bytes20ToBytes32(polygonUsdcAsset),
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

		console.log("BSC => Polygon")
		console.log("Estimated Fee", estimatedFee)
		console.log("Native Token Amount", nativeTokenAmount)

		assert(estimatedFee > 0n)
	}, 1_000_000)
})

describe.sequential("Swap Tests", () => {
	const mainnetId = "EVM-1"

	let intentGateway: IntentGateway
	let chainConfigService: ChainConfigService

	beforeAll(async () => {
		const setup = await setUp()
		chainConfigService = setup.chainConfigService

		const chainId = Number.parseInt(mainnetId.split("-")[1])
		const mainnetEvmChain = new EvmChain({
			chainId: chainId,
			host: chainConfigService.getHostAddress(mainnetId),
			url: process.env.ETH_MAINNET!,
		})

		intentGateway = new IntentGateway(mainnetEvmChain, mainnetEvmChain)
	})

	it("should get V2 quote and simulate swap with exact output", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = chainConfigService.getDaiAsset(mainnetId)
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)

		const amountOut = parseUnits("1000", 6)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				selectedProtocol: "v2",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v2", "Should select V2 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.amountIn > parseUnits("1000", 18), "Amount in should be greater than 1000 DAI")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V2 Exact Output - Amount In:", result.amountIn)
		console.log("V2 Exact Output - Protocol:", result.protocol)
		console.log("V2 Exact Output - Number of transactions:", result.transactions.length)

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: tx.value,
		}))

		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client
		const slot = await getStorageSlot(
			client as any,
			tokenIn,
			(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
		)

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: tokenIn,
					stateDiff: [
						{
							slot: slot as `0x${string}`,
							value: toHex(maxUint256),
						},
					],
				},
			],
		})

		assert(simulationResult.results[1].status === "success", "Swap should succeed")

		const balanceResult = simulationResult.results[2]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance === amountOut, "Balance should equal expected output amount")
		console.log("V2 simulation successful - Output balance:", balance)
	}, 1_000_000)

	it("should get V3 quote and simulate swap with exact output", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = chainConfigService.getDaiAsset(mainnetId)
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountOut = parseUnits("1000", 6)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				selectedProtocol: "v3",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v3", "Should select V3 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.fee !== undefined, "Should return fee tier")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		const amountIn = result.amountIn

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: tx.value,
		}))

		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client
		const slot = await getStorageSlot(
			client as any,
			tokenIn,
			(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
		)

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: tokenIn,
					stateDiff: [
						{
							slot: slot as `0x${string}`,
							value: toHex(maxUint256),
						},
					],
				},
			],
		})

		assert(simulationResult.results[1].status === "success", "Swap should succeed")

		const balanceResult = simulationResult.results[2]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance === amountOut, "Balance should equal expected output amount")
		console.log("V3 simulation successful - Fee tier:", result.fee, "Output balance:", balance)
	}, 1_000_000)

	it("should get V2 quote and simulate ETH to USDC swap", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = ADDRESS_ZERO // ETH
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountOut = parseUnits("1000", 6)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				selectedProtocol: "v2",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v2", "Should select V2 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V2 ETH => USDC - Amount In:", result.amountIn)

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: tx.value,
		}))

		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: fillerWalletAddress,
					balance: result.amountIn * 2n,
				},
			],
		})

		assert(simulationResult.results[0].status === "success", "Swap should succeed")

		const balanceResult = simulationResult.results[1]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance === amountOut, "Balance should equal expected output amount")
		console.log("V2 ETH => USDC simulation successful - Output balance:", balance)
	}, 1_000_000)

	it("should get V2 quote and simulate USDC to ETH swap", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = chainConfigService.getUsdcAsset(mainnetId)
		const tokenOut = ADDRESS_ZERO // ETH
		const amountOut = parseUnits("1", 18)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				selectedProtocol: "v2",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v2", "Should select V2 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V2 USDC => ETH - Amount In:", result.amountIn)

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: tx.value,
		}))

		const client = intentGateway.dest.client
		const slot = await getStorageSlot(
			client as any,
			tokenIn,
			(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
		)

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: tokenIn,
					stateDiff: [
						{
							slot: slot as `0x${string}`,
							value: toHex(maxUint256 / 3n),
						},
					],
				},
			],
		})

		assert(simulationResult.results[1].status === "success", "Swap should succeed")

		console.log("V2 USDC => ETH simulation successful")
	}, 1_000_000)

	it("should get V3 quote and simulate ETH to USDC swap", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = ADDRESS_ZERO // ETH
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountOut = parseUnits("1000", 6)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				selectedProtocol: "v3",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v3", "Should select V3 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.fee !== undefined, "Should return fee tier")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V3 ETH => USDC - Amount In:", result.amountIn, "Fee tier:", result.fee)

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: tx.value,
		}))

		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: fillerWalletAddress,
					balance: result.amountIn * 2n,
				},
			],
		})

		assert(simulationResult.results[0].status === "success", "Swap should succeed")

		const balanceResult = simulationResult.results[1]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance === amountOut, "Balance should equal expected output amount")
		console.log("V3 ETH => USDC simulation successful - Output balance:", balance)
	}, 1_000_000)

	it("should get V3 quote and simulate USDC to ETH swap", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = chainConfigService.getUsdcAsset(mainnetId)
		const tokenOut = ADDRESS_ZERO // ETH
		const amountOut = parseUnits("1", 18)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				selectedProtocol: "v3",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v3", "Should select V3 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.fee !== undefined, "Should return fee tier")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V3 USDC => ETH - Amount In:", result.amountIn, "Fee tier:", result.fee)

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: tx.value,
		}))

		const client = intentGateway.dest.client
		const slot = await getStorageSlot(
			client as any,
			tokenIn,
			(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
		)

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: tokenIn,
					stateDiff: [
						{
							slot: slot as `0x${string}`,
							value: toHex(maxUint256 / 3n),
						},
					],
				},
			],
		})

		assert(simulationResult.results[1].status === "success", "Swap should succeed")

		console.log("V3 USDC => ETH simulation successful")
	}, 1_000_000)

	it("should get V4 quote and simulate ETH to USDC swap", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = ADDRESS_ZERO // ETH
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountOut = parseUnits("1000", 6)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,

			{
				selectedProtocol: "v4",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v4", "Should select V4 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.fee !== undefined, "Should return fee tier")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		const amountIn = result.amountIn

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: 0n,
		}))
		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client

		// Simulate with balance overrides for universal router
		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: chainConfigService.getUniversalRouterAddress(mainnetId),
					balance: amountIn,
				},
			],
		})

		assert(simulationResult.results[0].status === "success", "Swap should succeed")

		const balanceResult = simulationResult.results[1]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance === amountOut, "Balance should equal expected output amount")
		console.log("V4 ETH/USDC simulation successful - Fee tier:", result.fee, "Output balance:", balance)
	}, 1_000_000)

	it("should get V4 quote and simulate USDT to USDC swap with Permit2", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = chainConfigService.getUsdtAsset(mainnetId)
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountOut = parseUnits("1000", 6)

		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				selectedProtocol: "v4",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v4", "Should select V4 protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: 0n,
		}))
		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client
		const slot = await getStorageSlot(
			client as any,
			tokenIn,
			(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
		)

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: tokenIn,
					stateDiff: [
						{
							slot: slot as `0x${string}`,
							value: toHex(maxUint256 / 3n),
						},
					],
				},
			],
		})

		const balanceResult = simulationResult.results[3]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance === amountOut, "Balance should equal expected output amount")
		console.log("V4 USDT/USDC simulation successful - Fee tier:", result.fee, "Output balance:", balance)
	}, 1_000_000)

	it("should get V2 quote with exact input and generate calldata", async () => {
		const tokenIn = chainConfigService.getDaiAsset(mainnetId)
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountIn = parseUnits("1000", 18)

		const result = await intentGateway.findBestProtocolWithAmountIn(
			"dest",
			tokenIn,
			tokenOut,
			amountIn,
			mainnetId,
			{
				selectedProtocol: "v2",
				generateCalldata: true,
				recipient: ADDRESS_ZERO,
			},
		)

		assert(result.protocol === "v2", "Should select V2 protocol")
		assert(result.amountOut !== BigInt(0), "Should return valid amount out")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V2 Exact Input - Amount Out:", result.amountOut)
		console.log("V2 Exact Input - Protocol:", result.protocol)
		console.log("V2 Exact Input - Number of transactions:", result.transactions.length)
	}, 1_000_000)

	it("should get V3 quote with exact input and generate calldata", async () => {
		const fillerWalletAddress = privateKeyToAddress(process.env.PRIVATE_KEY as HexString)
		const tokenIn = chainConfigService.getDaiAsset(mainnetId)
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountIn = parseUnits("1000", 18)

		const result = await intentGateway.findBestProtocolWithAmountIn(
			"dest",
			tokenIn,
			tokenOut,
			amountIn,
			mainnetId,
			{
				selectedProtocol: "v3",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v3", "Should select V3 protocol")
		assert(result.amountOut !== BigInt(0), "Should return valid amount out")
		assert(result.fee !== undefined, "Should return fee tier")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V3 Exact Input - Amount Out:", result.amountOut)
		console.log("V3 Exact Input - Fee tier:", result.fee)
		console.log("V3 Exact Input - Number of transactions:", result.transactions.length)
	}, 1_000_000)

	it("should get V4 quote and simulate ETH to USDC swap with exact input", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = ADDRESS_ZERO // ETH
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountIn = parseUnits("1", 18)

		const result = await intentGateway.findBestProtocolWithAmountIn(
			"dest",
			tokenIn,
			tokenOut,
			amountIn,
			mainnetId,
			{
				selectedProtocol: "v4",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v4", "Should select V4 protocol")
		assert(result.amountOut !== BigInt(0), "Should return valid amount out")
		assert(result.fee !== undefined, "Should return fee tier")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V4 Exact Input ETH/USDC - Amount Out:", result.amountOut)
		console.log("V4 Exact Input ETH/USDC - Fee tier:", result.fee)

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: 0n,
		}))
		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: chainConfigService.getUniversalRouterAddress(mainnetId),
					balance: amountIn,
				},
			],
		})

		assert(simulationResult.results[0].status === "success", "Swap should succeed")

		const balanceResult = simulationResult.results[1]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance >= result.amountOut, "Balance should be at least the quoted amount out")
		console.log("V4 ETH/USDC exact input simulation successful - Output balance:", balance)
	}, 1_000_000)

	it("should get V4 quote and simulate USDT to USDC swap with exact input", async () => {
		const fillerWalletAddress = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const tokenIn = chainConfigService.getUsdtAsset(mainnetId)
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountIn = parseUnits("1000", 6)

		const result = await intentGateway.findBestProtocolWithAmountIn(
			"dest",
			tokenIn,
			tokenOut,
			amountIn,
			mainnetId,
			{
				selectedProtocol: "v4",
				generateCalldata: true,
				recipient: fillerWalletAddress,
			},
		)

		assert(result.protocol === "v4", "Should select V4 protocol")
		assert(result.amountOut !== BigInt(0), "Should return valid amount out")
		assert(result.fee !== undefined, "Should return fee tier")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("V4 Exact Input USDT/USDC - Amount Out:", result.amountOut)
		console.log("V4 Exact Input USDT/USDC - Fee tier:", result.fee)

		const calls = result.transactions.map((tx) => ({
			to: tx.to,
			data: tx.data,
			value: 0n,
		}))
		calls.push({
			to: tokenOut,
			data: encodeFunctionData({
				abi: erc6160.ABI,
				functionName: "balanceOf",
				args: [fillerWalletAddress],
			}),
			value: 0n,
		})

		const client = intentGateway.dest.client
		const slot = await getStorageSlot(
			client as any,
			tokenIn,
			(ERC20Method.BALANCE_OF + bytes20ToBytes32(fillerWalletAddress).slice(2)) as `0x${string}`,
		)

		const simulationResult = await client.simulateCalls({
			account: fillerWalletAddress,
			calls,
			stateOverrides: [
				{
					address: tokenIn,
					stateDiff: [
						{
							slot: slot as `0x${string}`,
							value: toHex(maxUint256 / 3n),
						},
					],
				},
			],
		})

		const balanceResult = simulationResult.results[3]
		assert(balanceResult.status === "success", "Balance check should succeed")

		const balance = decodeFunctionResult({
			abi: erc6160.ABI,
			functionName: "balanceOf",
			data: balanceResult.data,
		})

		assert(balance >= result.amountOut, "Balance should be at least the quoted amount out")
		console.log("V4 USDT/USDC exact input simulation successful - Output balance:", balance)
	}, 1_000_000)

	it("should find best protocol automatically without selection", async () => {
		const tokenIn = chainConfigService.getDaiAsset(mainnetId)
		const tokenOut = chainConfigService.getUsdcAsset(mainnetId)
		const amountOut = parseUnits("1000", 6)

		// Test without specifying protocol - should auto-select best
		const result = await intentGateway.findBestProtocolWithAmountOut(
			"dest",
			tokenIn,
			tokenOut,
			amountOut,
			mainnetId,
			{
				generateCalldata: true,
				recipient: ADDRESS_ZERO,
			},
		)

		assert(result.protocol !== null, "Should select a protocol")
		assert(result.amountIn !== maxUint256, "Should return valid amount in")
		assert(result.transactions, "Should generate transactions")
		assert(result.transactions.length > 0, "Should have at least one transaction")

		console.log("Best Protocol Auto-Selected:", result.protocol)
		console.log("Amount In:", result.amountIn)
		console.log("Number of transactions:", result.transactions.length)
		if (result.fee !== undefined) {
			console.log("Fee tier:", result.fee)
		}
	}, 1_000_000)
})

describe("Order Cancellation tests", () => {
	let indexer: IndexerClient
	let hyperbridgeInstance: SubstrateChain

	beforeAll(async () => {
		const { bscChapelIsmpHost, ethSepoliaIsmpHost, hyperbridge } = await setUpBscToSepoliaOrder()

		const query_client = createQueryClient({
			url: "https://gargantua.indexer.polytope.technology",
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
	}, 1_000_000)

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

		console.log("Order placed on BSC")

		const orderPlaceEvent = parseEventLogs({ abi: IntentGatewayABI.ABI, logs: receipt.logs })[0]
		if (orderPlaceEvent.eventName !== "OrderPlaced") {
			throw new Error("Unexpected Event type")
		}
		const orderPlaced = orderPlaceEvent.args

		const hyperbridgeConfig: IHyperbridgeConfig = {
			wsUrl: process.env.HYPERBRIDGE_GARGANTUA!,
			consensusStateId: "PAS0",
			stateMachineId: "KUSAMA-4009",
		}

		const cancelGenerator = intentGateway.cancelOrder(order, hyperbridgeConfig, indexer)

		let result = await cancelGenerator.next()

		while (!result.done && result.value?.status !== "DESTINATION_FINALIZED") {
			const status = result.value?.status
			const data = result.value && "data" in result.value ? (result.value as any).data : undefined

			switch (status) {
				case "AWAITING_DESTINATION_FINALIZED":
					if (data) {
						console.log(
							`Waiting for destination finalized. Current height: ${data.currentHeight}, Deadline: ${data.deadline}`,
						)
					}
					break
				case "PROOF_FETCH_FAILED":
					if (data) {
						console.log(`Proof fetch failed at height: ${data.failedHeight}`)
					}
					break
				default:
					break
			}

			result = await cancelGenerator.next()
		}

		expect(result.value?.status).toBe("DESTINATION_FINALIZED")

		if (result.value?.status === "DESTINATION_FINALIZED" && result.value && "data" in result.value) {
			const data = (result.value as any).data as { proof: IProof }
			expect(data.proof).toBeDefined()
		}
		const finalizedHeight = (result.value as any).data.proof.height as bigint

		result = await cancelGenerator.next()
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

		console.log("Order cancelled on BSC")

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

		result = await cancelGenerator.next(getRequest)

		while (!result.done && result.value?.status !== "SOURCE_FINALIZED") {
			result = await cancelGenerator.next()
		}
		expect(result.value?.status).toBe("SOURCE_FINALIZED")

		while (!result.done && result.value?.status !== "SOURCE_PROOF_RECEIVED") {
			result = await cancelGenerator.next()
		}
		expect(result.value?.status).toBe("SOURCE_PROOF_RECEIVED")

		while (!result.done) {
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

		expect(result.value?.status).toBe("HYPERBRIDGE_FINALIZED")
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

async function setUpBscToPolygon() {
	const bscMainnetId = "EVM-56"
	const polygonMainnetId = "EVM-137"
	const chains = [bscMainnetId, polygonMainnetId]

	let chainConfigService = new ChainConfigService()
	let chainConfigs: ChainConfig[] = chains.map((chain) => chainConfigService.getChainConfig(chain))

	const bscMainnetPublicClient = createPublicClient({
		chain: bsc,
		transport: http(process.env.BSC_MAINNET!),
	})

	const polygonPublicClient = createPublicClient({
		chain: {
			id: 137,
			name: "polygon",
			nativeCurrency: { name: "MATIC", symbol: "MATIC", decimals: 18 },
			rpcUrls: { default: { http: [process.env.POLYGON_MAINNET!] } },
		},
		transport: http(process.env.POLYGON_MAINNET!),
	})

	const bscMainnetIsmpHostAddress = "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7" as HexString
	const polygonIsmpHostAddress = "0xD8d3db17C1dF65b301D45C84405CcAC1395C559a" as HexString

	const bscMainnetIsmpHost = getContract({
		address: bscMainnetIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: bscMainnetPublicClient,
	})

	const polygonMainnetIsmpHost = getContract({
		address: polygonIsmpHostAddress,
		abi: EVM_HOST.ABI,
		client: polygonPublicClient,
	})

	return {
		chainConfigs,
		chainConfigService,
		bscMainnetIsmpHost,
		polygonMainnetIsmpHost,
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
