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
import { FXFiller, type TradingPair } from "@/strategies/fx"
import { AssetRegistry } from "@/config/asset-registry"
import { Decimal } from "decimal.js"
import { VaultFundingPlanner } from "@/funding/vault/VaultFundingPlanner"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import {
	type ChainConfig,
	type FillerConfig,
	type HexString,
	type Order,
	type TokenInfo,
	type SelectBidResult,
	bytes20ToBytes32,
	EvmChain,
	IntentGateway,
	IntentsCoprocessor,
	DEFAULT_GRAFFITI,
} from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"
import { ConfirmationPolicy, FillerPricePolicy } from "@/config/interpolated-curve"
import {
	formatUnits,
	getContract,
	maxUint256,
	parseUnits,
	type PublicClient,
	type WalletClient,
	encodePacked,
	keccak256,
	toHex,
} from "viem"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { privateKeyToAccount } from "viem/accounts"
import "../setup"
import { pimlicoBundlerUrlForChain as bundlerUrl } from "../pimlicoBundler"
import { ERC20_ABI } from "@/config/abis/ERC20"

/** Same-token USDC/USDC + USDT/USDT pairs at a flat 50 bps spread (ask price 0.995). */
function sameTokenPairs(maxOrderSize: number): TradingPair[] {
	return ["USDC", "USDT"].map((symbol) => ({
		token0: symbol,
		token1: symbol,
		maxOrderSize: new Decimal(maxOrderSize),
		askPricePolicy: new FillerPricePolicy({ points: [{ amount: "0", price: "0.995" }] }),
	}))
}


// ============================================================================
// StreamingYieldVault deployments (ERC-4626, owner = test wallet)
// ============================================================================

const CHAPEL_VAULT = "0x358a58b807E1d55f5F9359AdbDd5240dAb6Eb7c8" as HexString
const AMOY_VAULT = "0x89Fd5d01dC44B6AB8E9f4F9549b39B91Bf1CED2E" as HexString

const bscChapelId = "EVM-97"
const polygonAmoyId = "EVM-80002"

describe("Vault funding venue - testnet", () => {
	it("fills an order by sourcing the output shortfall from the vault", async () => {
		const {
			bscIntentGatewayV2,
			polygonAmoyPublicClient,
			bscPublicClient,
			chainConfigs,
			fillerConfig,
			chainConfigService,
			bscWalletClient,
			chainClientManager,
			contractService,
		} = await setUp()

		const solver = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const venue = makeVenue(chainClientManager)

		const { intentFiller, strategy } = await createIntentFiller(chainConfigs, fillerConfig, chainConfigService, [
			venue,
		])
		await strategy.initialise()
		await intentFiller.initialize()
		intentFiller.start()

		const sourceUsdc = chainConfigService.getUsdcAsset(bscChapelId)
		const destUsdc = chainConfigService.getUsdcAsset(polygonAmoyId)
		const sourceUsdcDecimals = await contractService.getTokenDecimals(sourceUsdc, bscChapelId)
		const destUsdcDecimals = await contractService.getTokenDecimals(destUsdc, polygonAmoyId)

		// Force a destination-side shortfall: park all wallet USDC above a sliver
		// in the vault so the fill MUST be sourced via the venue's withdraw prepend.
		const keep = parseUnits("0.001", destUsdcDecimals)
		await depositWalletExcessIntoVault(chainClientManager, polygonAmoyId, destUsdc, AMOY_VAULT, solver, keep)

		const sharesBefore = await vaultShares(polygonAmoyPublicClient, AMOY_VAULT, solver)
		expect(sharesBefore).toBeGreaterThan(0n)

		const amount = parseUnits("0.1", sourceUsdcDecimals)
		const outputAmount = amount - parseUnits("0.094", destUsdcDecimals)
		// The wallet sliver cannot cover the requested output — the vault must.
		expect(keep).toBeLessThan(outputAmount)

		const inputs: TokenInfo[] = [{ token: bytes20ToBytes32(sourceUsdc), amount }]
		const outputs: TokenInfo[] = [{ token: bytes20ToBytes32(destUsdc), amount: outputAmount }]

		const beneficiary = bytes20ToBytes32(solver)

		let order: Order = {
			user: bytes20ToBytes32(solver),
			source: toHex(bscChapelId),
			destination: toHex(polygonAmoyId),
			deadline: 12545151568145n,
			nonce: 0n,
			fees: parseUnits("0.02", sourceUsdcDecimals),
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
		const polygonAmoyEvmChain = EvmChain.fromParams({
			chainId: 80002,
			host: chainConfigService.getHostAddress(polygonAmoyId),
			rpcUrl: chainConfigService.getRpcUrl(polygonAmoyId),
			bundlerUrl: chainConfigService.getBundlerUrl(polygonAmoyId),
		})

		const feeToken = await contractService.getFeeTokenWithDecimals(bscChapelId)
		await approveTokens(bscWalletClient, bscPublicClient, feeToken.address, bscIntentGatewayV2.address)
		await approveTokens(bscWalletClient, bscPublicClient, sourceUsdc, bscIntentGatewayV2.address)

		const userSdkHelper = await IntentGateway.create(bscEvmChain, polygonAmoyEvmChain, intentsCoprocessor)

		const gen = userSdkHelper.execute(order, DEFAULT_GRAFFITI, { auctionTimeMs: 15_000, pollIntervalMs: 5_000 })
		let result = await gen.next()
		if (result.value?.status === "AWAITING_PLACE_ORDER") {
			const { to, data } = result.value
			const signedTx = (await bscWalletClient.signTransaction(
				(await bscPublicClient.prepareTransactionRequest({
					to,
					data,
					value: 0n,
					account: bscWalletClient.account!,
					chain: bscWalletClient.chain,
				})) as any,
			)) as HexString
			result = await gen.next(signedTx)
		}
		let userOpHash: HexString | undefined
		let selectedSolver: HexString | undefined
		let finalizedOrder: Order | undefined
		while (!result.done) {
			let feedback: SelectBidResult | undefined
			if (result.value && "status" in result.value) {
				const status = result.value
				if (status.status === "ORDER_PLACED") {
					finalizedOrder = status.order
				}
				if (status.status === "BIDS_RECEIVED") {
					if (!finalizedOrder) throw new Error("Order was not finalized")
					expect(status.bids.length).toBeGreaterThan(0)
					const ranked = await userSdkHelper.sortBids(finalizedOrder, status.bids)
					const chosen = ranked[0]
					await chosen.simulate()
					feedback = await chosen.execute()
				}
				if (status.status === "BID_SELECTED") {
					selectedSolver = status.selectedSolver as HexString
					userOpHash = status.userOpHash as HexString
					// Cross-chain settles asynchronously via Hyperbridge — BID_SELECTED is
					// terminal here. Close the generator and stop driving it.
					void gen.return(undefined).catch(() => {})
					break
				}
				if (status.status === "FAILED") {
					throw new Error(`Order execution failed: ${status.error}`)
				}
			}
			result = await gen.next(feedback)
		}
		expect(userOpHash).toBeDefined()
		expect(selectedSolver).toBeDefined()
		const finalizedOrderId = finalizedOrder?.id
		expect(finalizedOrderId).toBeDefined()

		const isFilled = await pollForOrderFilled(
			finalizedOrderId as HexString,
			polygonAmoyPublicClient,
			chainConfigService.getIntentGatewayAddress(polygonAmoyId),
		)
		expect(isFilled).toBe(true)

		// The fill batch must have burned vault shares to cover the shortfall.
		const sharesAfter = await vaultShares(polygonAmoyPublicClient, AMOY_VAULT, solver)
		expect(sharesAfter).toBeLessThan(sharesBefore)

		await intentFiller.stop()
		await intentsCoprocessor.disconnect()
	}, 600_000)

	it("sweeps wallet excess above threshold into the vault", async () => {
		const { chainClientManager, chainConfigService } = await setUp()
		const solver = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address
		const chapelClient = chainClientManager.getPublicClient(bscChapelId)
		const usdc = chainConfigService.getUsdcAsset(bscChapelId)

		const decimals = (await chapelClient.readContract({
			abi: ERC20_ABI,
			address: usdc,
			functionName: "decimals",
		})) as number
		const balance = (await chapelClient.readContract({
			abi: ERC20_ABI,
			address: usdc,
			functionName: "balanceOf",
			args: [solver],
		})) as bigint

		// Threshold = current balance minus 50 USDC, so exactly ~50 gets swept.
		const sweepable = parseUnits("50", decimals)
		expect(balance).toBeGreaterThan(sweepable)
		const threshold = formatUnits(balance - sweepable, decimals)

		const venue = new VaultFundingPlanner(chainClientManager, {
			vaultsByChain: { [bscChapelId]: [{ vault: CHAPEL_VAULT, threshold }] },
		})
		await venue.initialise(solver as HexString)

		const sharesBefore = await vaultShares(chapelClient, CHAPEL_VAULT, solver)
		await venue.sweepExcessToVault(bscChapelId)

		const balanceAfter = (await chapelClient.readContract({
			abi: ERC20_ABI,
			address: usdc,
			functionName: "balanceOf",
			args: [solver],
		})) as bigint
		const sharesAfter = await vaultShares(chapelClient, CHAPEL_VAULT, solver)

		expect(balanceAfter).toBe(parseUnits(threshold, decimals))
		expect(sharesAfter).toBeGreaterThan(sharesBefore)
	}, 300_000)

	it("redeemAll exits every vault position back to the underlying", async () => {
		const { chainClientManager, chainConfigService } = await setUp()
		const solver = privateKeyToAccount(process.env.PRIVATE_KEY as HexString).address

		const venue = makeVenue(chainClientManager)
		await venue.initialise(solver as HexString)
		await venue.redeemAll()

		for (const [chain, vault] of [
			[bscChapelId, CHAPEL_VAULT],
			[polygonAmoyId, AMOY_VAULT],
		] as const) {
			const shares = await vaultShares(chainClientManager.getPublicClient(chain), vault, solver)
			expect(shares).toBe(0n)
		}

		// Leave the environment as found: restore a base position in each vault
		// so reruns (and the fill test) start with withdrawable liquidity.
		for (const [chain, vault, amountHuman] of [
			[bscChapelId, CHAPEL_VAULT, "1000"],
			[polygonAmoyId, AMOY_VAULT, "300"],
		] as const) {
			const usdc = chainConfigService.getUsdcAsset(chain)
			const client = chainClientManager.getPublicClient(chain)
			const decimals = (await client.readContract({
				abi: ERC20_ABI,
				address: usdc,
				functionName: "decimals",
			})) as number
			await depositIntoVault(chainClientManager, chain, usdc, vault, parseUnits(amountHuman, decimals))
		}
	}, 300_000)
})

// ============================================================================
// Shared Helpers
// ============================================================================

function makeVenue(chainClientManager: ChainClientManager): VaultFundingPlanner {
	return new VaultFundingPlanner(chainClientManager, {
		vaultsByChain: {
			[bscChapelId]: [{ vault: CHAPEL_VAULT }],
			[polygonAmoyId]: [{ vault: AMOY_VAULT }],
		},
	})
}

async function vaultShares(client: PublicClient, vault: HexString, owner: string): Promise<bigint> {
	return (await client.readContract({
		abi: ERC20_ABI,
		address: vault,
		functionName: "balanceOf",
		args: [owner as HexString],
	})) as bigint
}

/** Deposits the wallet's balance above `keep` into the vault via plain EOA transactions. */
async function depositWalletExcessIntoVault(
	chainClientManager: ChainClientManager,
	chain: string,
	asset: HexString,
	vault: HexString,
	solver: string,
	keep: bigint,
): Promise<void> {
	const publicClient = chainClientManager.getPublicClient(chain)
	const balance = (await publicClient.readContract({
		abi: ERC20_ABI,
		address: asset,
		functionName: "balanceOf",
		args: [solver as HexString],
	})) as bigint
	if (balance <= keep) return
	await depositIntoVault(chainClientManager, chain, asset, vault, balance - keep)
}

async function depositIntoVault(
	chainClientManager: ChainClientManager,
	chain: string,
	asset: HexString,
	vault: HexString,
	amount: bigint,
): Promise<void> {
	const publicClient = chainClientManager.getPublicClient(chain)
	const walletClient = chainClientManager.getWalletClient(chain)

	const approveTx = await walletClient.writeContract({
		abi: ERC20_ABI,
		address: asset,
		functionName: "approve",
		args: [vault, amount],
		chain: walletClient.chain,
		account: walletClient.account!,
	})
	await publicClient.waitForTransactionReceipt({ hash: approveTx, confirmations: 1 })

	const depositTx = await walletClient.writeContract({
		abi: ERC4626_ABI,
		address: vault,
		functionName: "deposit",
		args: [amount, walletClient.account!.address],
		chain: walletClient.chain,
		account: walletClient.account!,
	})
	await publicClient.waitForTransactionReceipt({ hash: depositTx, confirmations: 1 })
}

async function createIntentFiller(
	chainConfigs: ChainConfig[],
	fillerConfig: FillerConfig,
	chainConfigService: FillerConfigService,
	fundingVenues: VaultFundingPlanner[],
): Promise<{ intentFiller: IntentFiller; strategy: FXFiller }> {
	const privateKey = process.env.PRIVATE_KEY as HexString
	const signer = await createSimplexSigner({ type: SignerType.PrivateKey, key: privateKey })
	const cacheService = new CacheService()
	const chainClientManager = new ChainClientManager(chainConfigService, signer)
	const contractService = new ContractInteractionService(chainClientManager, chainConfigService, signer, cacheService)

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

	const strategy = new FXFiller(
		signer,
		chainConfigService,
		chainClientManager,
		contractService,
		sameTokenPairs(10000),
		new AssetRegistry(chainConfigService),
		{ confirmationPolicy, fundingVenues },
	)

	const intentFiller = new IntentFiller(
		chainConfigs,
		[strategy],
		fillerConfig,
		chainConfigService,
		chainClientManager,
		contractService,
		signer,
	)
	return { intentFiller, strategy }
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
	const chains = [bscChapelId, polygonAmoyId]

	const testChainConfigs: ResolvedChainConfig[] = [
		{ chainId: 97, rpcUrls: [process.env.BSC_CHAPEL!], bundlerUrl: bundlerUrl(97) },
		{ chainId: 80002, rpcUrls: [process.env.POLYGON_AMOY!], bundlerUrl: bundlerUrl(80002) },
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
	const contractService = new ContractInteractionService(chainClientManager, chainConfigService, signer, cacheService)

	const bscWalletClient = chainClientManager.getWalletClient(bscChapelId)
	const bscPublicClient = chainClientManager.getPublicClient(bscChapelId)
	const polygonAmoyPublicClient = chainClientManager.getPublicClient(polygonAmoyId)

	const bscIntentGatewayV2 = getContract({
		address: chainConfigService.getIntentGatewayAddress(bscChapelId),
		abi: INTENT_GATEWAY_V2_ABI,
		client: { public: bscPublicClient, wallet: bscWalletClient },
	})

	return {
		bscWalletClient,
		bscPublicClient,
		polygonAmoyPublicClient,
		bscIntentGatewayV2,
		chainClientManager,
		contractService,
		chainConfigService,
		fillerConfig,
		chainConfigs,
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
