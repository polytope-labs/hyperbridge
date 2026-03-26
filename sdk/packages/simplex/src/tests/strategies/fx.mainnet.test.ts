import { IntentFiller } from "@/core/filler"
import {
	BidStorageService,
	CacheService,
	ChainClientManager,
	ContractInteractionService,
	FillerConfigService,
	type UserProvidedChainConfig,
	type FillerConfig as FillerServiceConfig,
} from "@/services"
import { createSimplexSigner, SignerType, type SigningAccount } from "@/services/wallet"
import { FXFiller } from "@/strategies/fx"
import { ConfirmationPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
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
	DEFAULT_GRAFFITI,
} from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import {
	getContract,
	maxUint256,
	parseUnits,
	type PublicClient,
	type WalletClient,
	encodePacked,
	keccak256,
	toHex,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { ERC20_ABI } from "@/config/abis/ERC20"
import "../setup"

// NOTE: This is a live mainnet integration test.
// It is skipped by default to avoid accidental execution in CI.

describe.skip("Filler V2 FX - Polygon mainnet same-chain swap", () => {
	it("Should place USDC->EXT order on Polygon and fill on Polygon using FX strategy only", async () => {
		const {
			polygonIntentGatewayV2,
			polygonPublicClient,
			polygonWalletClient,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			polygonMainnetId,
			contractService,
		} = await setUpMainnetFx()

		const intentFiller = createFxOnlyIntentFiller(
			chainConfigs,
			fillerConfig,
			chainConfigService,
			contractService,
			polygonMainnetId,
		)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(polygonMainnetId)
		const destExt = chainConfigService.getExtAsset(polygonMainnetId)!

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, polygonMainnetId)
		const destExtDecimals = await contractService.getTokenDecimals(destExt, polygonMainnetId)
		const amountIn = parseUnits("0.01", sourceUsdcDecimals)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount: amountIn }]

		// For the test, request slightly less than notional amount after a simple fee/spread
		const requestedExtOut = parseUnits("0.006", destExtDecimals)
		const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(destExt), amount: requestedExtOut }]

		const beneficiaryAddress = "0xdab14BdBF23d10F062eAA1a527cE2e9354E9e07F"
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)
		const user = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address

		let order: Order = {
			user: bytes20ToBytes32(user),
			source: toHex(polygonMainnetId),
			destination: toHex(polygonMainnetId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: 0n,
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_NEXUS!,
			process.env.SECRET_PHRASE!,
		)

		const destBundlerUrl = chainConfigService.getBundlerUrl(polygonMainnetId)
		const polygonEvmChain = EvmChain.fromParams({
			chainId: 137,
			host: chainConfigService.getHostAddress(polygonMainnetId),
			rpcUrl: chainConfigService.getRpcUrl(polygonMainnetId),
			bundlerUrl: destBundlerUrl,
		})

		const feeToken = await contractService.getFeeTokenWithDecimals(polygonMainnetId)
		await approveTokens(polygonWalletClient, polygonPublicClient, feeToken.address, polygonIntentGatewayV2.address)
		await approveTokens(polygonWalletClient, polygonPublicClient, sourceUsdc, polygonIntentGatewayV2.address)

		// Same-chain: source and destination EvmChain are both Polygon
		const userSdkHelper = await IntentGateway.create(polygonEvmChain, polygonEvmChain, intentsCoprocessor)

		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, {
			bidTimeoutMs: 600_000,
			pollIntervalMs: 5_000,
		})

		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data, value } = result.value

			const signedTx = (await polygonWalletClient.signTransaction(
				(await polygonPublicClient.prepareTransactionRequest({
					to,
					data,
					value,
					account: polygonWalletClient.account!,
					chain: polygonWalletClient.chain,
				})) as any,
			)) as HexString
			result = await gen.next(signedTx)
		}

		let userOpHash: HexString | undefined
		let selectedSolver: HexString | undefined

		while (!result.done) {
			if (result.value && "status" in result.value) {
				const status = result.value
				console.log("status", status)

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
			polygonPublicClient,
			chainConfigService.getIntentGatewayV2Address(polygonMainnetId),
		)
		expect(isFilled).toBe(true)

		await intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)
})

describe.skip("Filler V2 FX - Base mainnet same-chain swap", () => {
	it("Should place USDC->EXT order on Base and fill on Base using FX strategy only", async () => {
		const {
			baseIntentGatewayV2,
			basePublicClient,
			baseWalletClient,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			baseMainnetId,
			contractService,
		} = await setUpMainnetFxBase()

		const intentFiller = createFxOnlyIntentFiller(
			chainConfigs,
			fillerConfig,
			chainConfigService,
			contractService,
			baseMainnetId,
		)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(baseMainnetId)
		const destExt = chainConfigService.getExtAsset(baseMainnetId)!

		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, baseMainnetId)
		const destExtDecimals = await contractService.getTokenDecimals(destExt, baseMainnetId)
		const amountIn = parseUnits("0.01", sourceUsdcDecimals)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount: amountIn }]

		const requestedExtOut = parseUnits("0.006", destExtDecimals)
		const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(destExt), amount: requestedExtOut }]

		const beneficiaryAddress = "0xdab14BdBF23d10F062eAA1a527cE2e9354E9e07F"
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)
		const user = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address

		let order: Order = {
			user: bytes20ToBytes32(user),
			source: toHex(baseMainnetId),
			destination: toHex(baseMainnetId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: 0n,
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_NEXUS!,
			process.env.SECRET_PHRASE!,
		)

		const destBundlerUrl = chainConfigService.getBundlerUrl(baseMainnetId)
		const baseEvmChain = EvmChain.fromParams({
			chainId: 8453,
			host: chainConfigService.getHostAddress(baseMainnetId),
			rpcUrl: chainConfigService.getRpcUrl(baseMainnetId),
			bundlerUrl: destBundlerUrl,
		})

		const feeToken = await contractService.getFeeTokenWithDecimals(baseMainnetId)
		await approveTokens(baseWalletClient, basePublicClient, feeToken.address, baseIntentGatewayV2.address)
		await approveTokens(baseWalletClient, basePublicClient, sourceUsdc, baseIntentGatewayV2.address)

		const userSdkHelper = await IntentGateway.create(baseEvmChain, baseEvmChain, intentsCoprocessor)

		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, {
			bidTimeoutMs: 600_000_00,
			pollIntervalMs: 5_000,
		})

		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data, value } = result.value

			const signedTx = (await baseWalletClient.signTransaction(
				(await basePublicClient.prepareTransactionRequest({
					to,
					data,
					value: value ?? 0n,
					account: baseWalletClient.account!,
					chain: baseWalletClient.chain,
				})) as any,
			)) as HexString
			result = await gen.next(signedTx)
		}

		let userOpHash: HexString | undefined
		let selectedSolver: HexString | undefined

		while (!result.done) {
			if (result.value && "status" in result.value) {
				const status = result.value
				console.log("status", status)

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
			basePublicClient,
			chainConfigService.getIntentGatewayV2Address(baseMainnetId),
		)
		expect(isFilled).toBe(true)

		await new Promise((resolve) => setTimeout(resolve, 10000000))

		await intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)
})

describe.skip("Filler V2 FX - Arbitrum mainnet same-chain swap", () => {
	it("Should place EXT->USDC order on Arbitrum and fill on Arbitrum using FX strategy only", async () => {
		const {
			arbitrumIntentGatewayV2,
			arbitrumPublicClient,
			arbitrumWalletClient,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			arbitrumMainnetId,
			contractService,
		} = await setUpMainnetFxArbitrum()

		const intentFiller = createFxOnlyIntentFiller(
			chainConfigs,
			fillerConfig,
			chainConfigService,
			contractService,
			arbitrumMainnetId,
		)
		await intentFiller.initialize()
		intentFiller.start()

		const sourceExt = chainConfigService.getExtAsset(arbitrumMainnetId)!
		const destUsdc = chainConfigService.getUsdcAsset(arbitrumMainnetId)

		const sourceExtDecimals = await contractService.getTokenDecimals(sourceExt, arbitrumMainnetId)
		const destUsdcDecimals = await contractService.getTokenDecimals(destUsdc, arbitrumMainnetId)
		const amountIn = parseUnits("100", sourceExtDecimals)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceExt), amount: amountIn }]

		const requestedUsdcOut = parseUnits("0.01", destUsdcDecimals)
		const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(destUsdc), amount: requestedUsdcOut }]

		const beneficiaryAddress = "0xdab14BdBF23d10F062eAA1a527cE2e9354E9e07F"
		const beneficiary = bytes20ToBytes32(beneficiaryAddress)
		const user = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address

		let order: Order = {
			user: bytes20ToBytes32(user),
			source: toHex(arbitrumMainnetId),
			destination: toHex(arbitrumMainnetId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: 0n,
			session: "0x0000000000000000000000000000000000000000" as HexString,
			predispatch: { assets: [], call: "0x" as HexString },
			inputs,
			output: { beneficiary, assets: outputs, call: "0x" as HexString },
		}

		const intentsCoprocessor = await IntentsCoprocessor.connect(
			process.env.HYPERBRIDGE_NEXUS!,
			process.env.SECRET_PHRASE!,
		)

		const destBundlerUrl = chainConfigService.getBundlerUrl(arbitrumMainnetId)
		const arbitrumEvmChain = EvmChain.fromParams({
			chainId: 42161,
			host: chainConfigService.getHostAddress(arbitrumMainnetId),
			rpcUrl: chainConfigService.getRpcUrl(arbitrumMainnetId),
			bundlerUrl: destBundlerUrl,
		})

		const feeToken = await contractService.getFeeTokenWithDecimals(arbitrumMainnetId)
		await approveTokens(
			arbitrumWalletClient,
			arbitrumPublicClient,
			feeToken.address,
			arbitrumIntentGatewayV2.address,
		)
		await approveTokens(arbitrumWalletClient, arbitrumPublicClient, sourceExt, arbitrumIntentGatewayV2.address)

		const userSdkHelper = await IntentGateway.create(arbitrumEvmChain, arbitrumEvmChain, intentsCoprocessor)

		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, {
			bidTimeoutMs: 600_000,
			pollIntervalMs: 5_000,
		})

		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data, value } = result.value

			const signedTx = (await arbitrumWalletClient.signTransaction(
				(await arbitrumPublicClient.prepareTransactionRequest({
					to,
					data,
					value,
					account: arbitrumWalletClient.account!,
					chain: arbitrumWalletClient.chain,
				})) as any,
			)) as HexString
			result = await gen.next(signedTx)
		}

		let userOpHash: HexString | undefined
		let selectedSolver: HexString | undefined

		while (!result.done) {
			if (result.value && "status" in result.value) {
				const status = result.value
				console.log("status", status)

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
			arbitrumPublicClient,
			chainConfigService.getIntentGatewayV2Address(arbitrumMainnetId),
		)
		expect(isFilled).toBe(true)

		await new Promise((resolve) => setTimeout(resolve, 600_000_000))

		await intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000_000)
})

describe.skip("Filler V2 FX - Arbitrum to Base cross-chain swap", () => {
	it.skipIf(!hasMpcVaultFillCredentials())(
		"Should place USDC order on Arbitrum and fill with EXT on Base using FX strategy only (user private key, filler MPC)",
		async () => {
			const {
				arbitrumIntentGatewayV2,
				arbitrumPublicClient,
				arbitrumWalletClient,
				basePublicClient,
				chainConfigs,
				fillerConfig,
				chainConfigService,
				arbitrumMainnetId,
				baseMainnetId,
				contractService,
			} = await setUpMainnetFxArbitrumToBase()

			const fillSigner = createMpcVaultFillSigner()
			const intentFiller = createCrossChainFxIntentFiller(
				chainConfigs,
				fillerConfig,
				chainConfigService,
				[arbitrumMainnetId, baseMainnetId],
				fillSigner,
			)
			await intentFiller.initialize()
			intentFiller.start()

			const sourceUsdc = chainConfigService.getUsdcAsset(arbitrumMainnetId)
			const destExt = chainConfigService.getExtAsset(baseMainnetId)!

			const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, arbitrumMainnetId)
			const destExtDecimals = await contractService.getTokenDecimals(destExt, baseMainnetId)
			const amountIn = parseUnits("0.01", sourceUsdcDecimals)

			const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount: amountIn }]

			const requestedExtOut = parseUnits("0.006", destExtDecimals)
			const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(destExt), amount: requestedExtOut }]

			const beneficiaryAddress = "0xdab14BdBF23d10F062eAA1a527cE2e9354E9e07F"
			const beneficiary = bytes20ToBytes32(beneficiaryAddress)
			const user = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address

			let order: Order = {
				user: bytes20ToBytes32(user),
				source: toHex(arbitrumMainnetId),
				destination: toHex(baseMainnetId),
				deadline: 12545151568145n,
				nonce: 0n,
				fees: 0n,
				session: "0x0000000000000000000000000000000000000000" as HexString,
				predispatch: { assets: [], call: "0x" as HexString },
				inputs,
				output: { beneficiary, assets: outputs, call: "0x" as HexString },
			}

			const intentsCoprocessor = await IntentsCoprocessor.connect(
				process.env.HYPERBRIDGE_NEXUS!,
				process.env.SECRET_PHRASE!,
			)

			const arbitrumEvmChain = EvmChain.fromParams({
				chainId: 42161,
				host: chainConfigService.getHostAddress(arbitrumMainnetId),
				rpcUrl: chainConfigService.getRpcUrl(arbitrumMainnetId),
				bundlerUrl: chainConfigService.getBundlerUrl(arbitrumMainnetId),
			})

			const baseEvmChain = EvmChain.fromParams({
				chainId: 8453,
				host: chainConfigService.getHostAddress(baseMainnetId),
				rpcUrl: chainConfigService.getRpcUrl(baseMainnetId),
				bundlerUrl: chainConfigService.getBundlerUrl(baseMainnetId),
			})

			const feeToken = await contractService.getFeeTokenWithDecimals(arbitrumMainnetId)
			await approveTokens(
				arbitrumWalletClient,
				arbitrumPublicClient,
				feeToken.address,
				arbitrumIntentGatewayV2.address,
			)
			await approveTokens(arbitrumWalletClient, arbitrumPublicClient, sourceUsdc, arbitrumIntentGatewayV2.address)

			const userSdkHelper = await IntentGateway.create(arbitrumEvmChain, baseEvmChain, intentsCoprocessor)

			const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, {
				bidTimeoutMs: 600_000,
				pollIntervalMs: 5_000,
			})

			let result = await gen.next()
			if (result.value?.status === "AWAITING_PLACE_ORDER") {
				const { to, data, value } = result.value

				const signedTx = (await arbitrumWalletClient.signTransaction(
					(await arbitrumPublicClient.prepareTransactionRequest({
						to,
						data,
						value: value ?? 0n,
						account: arbitrumWalletClient.account!,
						chain: arbitrumWalletClient.chain,
					})) as any,
				)) as HexString
				result = await gen.next(signedTx)
			}

			let userOpHash: HexString | undefined
			let selectedSolver: HexString | undefined

			while (!result.done) {
				if (result.value && "status" in result.value) {
					const status = result.value
					console.log("status", status)

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
				basePublicClient,
				chainConfigService.getIntentGatewayV2Address(baseMainnetId),
			)
			expect(isFilled).toBe(true)

			await intentFiller.stop()
			await intentsCoprocessor.disconnect()
		},
		600_000,
	)
})

function hasMpcVaultFillCredentials(): boolean {
	return Boolean(
		process.env.MPCVAULT_API_TOKEN &&
			process.env.MPCVAULT_VAULT_UUID &&
			process.env.MPCVAULT_ACCOUNT_ADDRESS &&
			process.env.MPCVAULT_CALLBACK_CLIENT_SIGNER_PUBLIC_KEY,
	)
}

function createMpcVaultFillSigner() {
	return createSimplexSigner({
		type: SignerType.MpcVault,
		mpcVault: {
			apiToken: process.env.MPCVAULT_API_TOKEN!,
			vaultUuid: process.env.MPCVAULT_VAULT_UUID!,
			accountAddress: process.env.MPCVAULT_ACCOUNT_ADDRESS as HexString,
			callbackClientSignerPublicKey: process.env.MPCVAULT_CALLBACK_CLIENT_SIGNER_PUBLIC_KEY!,
		},
	})
}

function bundlerUrl(chainId: number): string | undefined {
	const apiKey = process.env.BUNDLER_API_KEY
	return apiKey ? `https://api.pimlico.io/v2/${chainId}/rpc?apikey=${apiKey}` : undefined
}

async function setUpMainnetFx() {
	const polygonMainnetId = "EVM-137"
	const chains = [polygonMainnetId]

	const testChainConfigs: UserProvidedChainConfig[] = [
		{ chainId: 137, rpcUrl: process.env.POLYGON_MAINNET!, bundlerUrl: bundlerUrl(137) },
	]

	const fillerConfigForService: FillerServiceConfig = {
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_NEXUS,
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
	const signer = createSimplexSigner({ type: SignerType.PrivateKey, privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)
	const contractService = new ContractInteractionService(chainClientManager, chainConfigService, signer, cacheService)

	const polygonWalletClient = chainClientManager.getWalletClient(polygonMainnetId)
	const polygonPublicClient = chainClientManager.getPublicClient(polygonMainnetId)

	const polygonIntentGatewayV2 = getContract({
		address: chainConfigService.getIntentGatewayV2Address(polygonMainnetId),
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: polygonPublicClient, wallet: polygonWalletClient },
	})

	return {
		polygonWalletClient,
		polygonPublicClient,
		polygonIntentGatewayV2,
		contractService,
		polygonMainnetId,
		chainConfigService,
		fillerConfig,
		chainConfigs,
	}
}

async function setUpMainnetFxBase() {
	const baseMainnetId = "EVM-8453"
	const chains = [baseMainnetId]

	const testChainConfigs: UserProvidedChainConfig[] = [
		{ chainId: 8453, rpcUrl: process.env.BASE_MAINNET!, bundlerUrl: bundlerUrl(8453) },
	]

	const fillerConfigForService: FillerServiceConfig = {
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_NEXUS,
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
	const signer = createSimplexSigner({ type: SignerType.PrivateKey, privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)
	const contractService = new ContractInteractionService(chainClientManager, chainConfigService, signer, cacheService)

	const baseWalletClient = chainClientManager.getWalletClient(baseMainnetId)
	const basePublicClient = chainClientManager.getPublicClient(baseMainnetId)

	const baseIntentGatewayV2 = getContract({
		address: chainConfigService.getIntentGatewayV2Address(baseMainnetId),
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: basePublicClient, wallet: baseWalletClient },
	})

	return {
		baseWalletClient,
		basePublicClient,
		baseIntentGatewayV2,
		contractService,
		baseMainnetId,
		chainConfigService,
		fillerConfig,
		chainConfigs,
	}
}

async function setUpMainnetFxArbitrum() {
	const arbitrumMainnetId = "EVM-42161"
	const chains = [arbitrumMainnetId]

	const testChainConfigs: UserProvidedChainConfig[] = [
		{ chainId: 42161, rpcUrl: process.env.ARBITRUM_MAINNET!, bundlerUrl: bundlerUrl(42161) },
	]

	const fillerConfigForService: FillerServiceConfig = {
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_NEXUS,
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
	const signer = createSimplexSigner({ type: SignerType.PrivateKey, privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)
	const contractService = new ContractInteractionService(chainClientManager, chainConfigService, signer, cacheService)

	const arbitrumWalletClient = chainClientManager.getWalletClient(arbitrumMainnetId)
	const arbitrumPublicClient = chainClientManager.getPublicClient(arbitrumMainnetId)

	const arbitrumIntentGatewayV2 = getContract({
		address: chainConfigService.getIntentGatewayV2Address(arbitrumMainnetId),
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: arbitrumPublicClient, wallet: arbitrumWalletClient },
	})

	return {
		arbitrumWalletClient,
		arbitrumPublicClient,
		arbitrumIntentGatewayV2,
		contractService,
		arbitrumMainnetId,
		chainConfigService,
		fillerConfig,
		chainConfigs,
	}
}

async function setUpMainnetFxArbitrumToBase() {
	const arbitrumMainnetId = "EVM-42161"
	const baseMainnetId = "EVM-8453"
	const chains = [arbitrumMainnetId, baseMainnetId]

	const testChainConfigs: UserProvidedChainConfig[] = [
		{ chainId: 42161, rpcUrl: process.env.ARBITRUM_MAINNET!, bundlerUrl: bundlerUrl(42161) },
		{ chainId: 8453, rpcUrl: process.env.BASE_MAINNET!, bundlerUrl: bundlerUrl(8453) },
	]

	const fillerConfigForService: FillerServiceConfig = {
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: process.env.HYPERBRIDGE_NEXUS,
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

	// User EOA (PRIVATE_KEY): Arbitrum wallet for approvals and placing the order.
	const privateKey = process.env.PRIVATE_KEY as HexString
	const userSigner = createSimplexSigner({ type: SignerType.PrivateKey, privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, userSigner)
	const contractService = new ContractInteractionService(
		chainClientManager,
		chainConfigService,
		userSigner,
		cacheService,
	)

	const arbitrumWalletClient = chainClientManager.getWalletClient(arbitrumMainnetId)
	const arbitrumPublicClient = chainClientManager.getPublicClient(arbitrumMainnetId)
	const basePublicClient = chainClientManager.getPublicClient(baseMainnetId)

	const arbitrumIntentGatewayV2 = getContract({
		address: chainConfigService.getIntentGatewayV2Address(arbitrumMainnetId),
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: arbitrumPublicClient, wallet: arbitrumWalletClient },
	})

	return {
		arbitrumWalletClient,
		arbitrumPublicClient,
		basePublicClient,
		arbitrumIntentGatewayV2,
		contractService,
		arbitrumMainnetId,
		baseMainnetId,
		chainConfigService,
		fillerConfig,
		chainConfigs,
	}
}

function createCrossChainFxIntentFiller(
	chainConfigs: ChainConfig[],
	fillerConfig: FillerConfig,
	chainConfigService: FillerConfigService,
	chainIds: string[],
	fillerSigner: SigningAccount,
): IntentFiller {
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, fillerSigner)
	const contractService = new ContractInteractionService(
		chainClientManager,
		chainConfigService,
		fillerSigner,
		cacheService,
	)

	const bidPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: "10000" },
			{ amount: "10000", price: "10000" },
		],
	})
	const askPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: "9500" },
			{ amount: "10000", price: "9500" },
		],
	})

	const exoticTokenAddresses: Record<string, HexString> = {}
	for (const id of chainIds) {
		const extAsset = chainConfigService.getExtAsset(id)
		if (extAsset) {
			exoticTokenAddresses[id] = extAsset as HexString
		}
	}

	const confirmationPolicy = new ConfirmationPolicy({
		"42161": {
			points: [
				{ amount: "0", value: 5 },
				{ amount: "10000", value: 10 },
			],
		},
		"8453": {
			points: [
				{ amount: "0", value: 5 },
				{ amount: "10000", value: 10 },
			],
		},
	})

	const fxStrategy = new FXFiller(
		fillerSigner,
		chainConfigService,
		chainClientManager,
		contractService,
		bidPricePolicy,
		askPricePolicy,
		"5000",
		exoticTokenAddresses,
		confirmationPolicy,
	)

	const strategies = [fxStrategy]
	const bidStorage = new BidStorageService(chainConfigService.getDataDir())

	return new IntentFiller(
		chainConfigs,
		strategies,
		fillerConfig,
		chainConfigService,
		chainClientManager,
		contractService,
		fillerSigner,
		undefined,
		bidStorage,
	)
}

function createFxOnlyIntentFiller(
	chainConfigs: ChainConfig[],
	fillerConfig: FillerConfig,
	chainConfigService: FillerConfigService,
	contractService: ContractInteractionService,
	mainnetId: string,
): IntentFiller {
	const privateKey = process.env.PRIVATE_KEY as HexString
	const signer = createSimplexSigner({ type: SignerType.PrivateKey, privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)

	// Bid: filler buys exotic from user → 1 USD = 10000 EXT (filler pays fewer USD per exotic)
	// Ask: filler sells exotic to user → 1 USD = 9500 EXT (filler gives fewer exotic per USD = spread profit)
	const bidPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: "10000" },
			{ amount: "10000", price: "10000" },
		],
	})
	const askPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: "10000" },
			{ amount: "10000", price: "10000" },
		],
	})

	const extAsset = chainConfigService.getExtAsset(mainnetId)
	const exoticTokenAddresses: Record<string, HexString> = extAsset ? { [mainnetId]: extAsset as HexString } : {}

	const fxStrategy = new FXFiller(
		signer,
		chainConfigService,
		chainClientManager,
		contractService,
		bidPricePolicy,
		askPricePolicy,
		"5000",
		exoticTokenAddresses,
	)

	const strategies = [fxStrategy]
	const bidStorage = new BidStorageService(chainConfigService.getDataDir())

	return new IntentFiller(
		chainConfigs,
		strategies,
		fillerConfig,
		chainConfigService,
		chainClientManager,
		contractService,
		signer,
		undefined,
		bidStorage,
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
