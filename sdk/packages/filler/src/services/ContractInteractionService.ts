import { getContract, toHex, encodePacked, keccak256, maxUint256, PublicClient, encodeAbiParameters } from "viem"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import {
	ADDRESS_ZERO,
	Order,
	PaymentInfo,
	HexString,
	FillOptions,
	DispatchPost,
	bytes32ToBytes20,
	bytes20ToBytes32,
	HostParams,
	estimateGasForPost,
	constructRedeemEscrowRequestBody,
	IPostRequest,
	getStorageSlot,
	ERC20Method,
} from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ChainClientManager } from "./ChainClientManager"
import { ChainConfigService } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { orderCommitment } from "@hyperbridge/sdk"
import { ApiPromise, WsProvider } from "@polkadot/api"
import { fetchTokenUsdPrice } from "@hyperbridge/sdk"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { Chains } from "@hyperbridge/sdk"
import { CacheService } from "./CacheService"
import { UNISWAP_V2_FACTORY_ABI } from "@/config/abis/UniswapV2Factory"
import { UNISWAP_ROUTER_V2_ABI } from "@/config/abis/UniswapRouterV2"
import { UNISWAP_V3_FACTORY_ABI } from "@/config/abis/UniswapV3Factory"
import { UNISWAP_V3_POOL_ABI } from "@/config/abis/UniswapV3Pool"
import { UNISWAP_V3_QUOTER_V2_ABI } from "@/config/abis/UniswapV3QuoterV2"
import { UNISWAP_V4_QUOTER_ABI } from "@/config/abis/UniswapV4Quoter"
/**
 * Handles contract interactions for tokens and other contracts
 */
export class ContractInteractionService {
	private configService: ChainConfigService
	private api: ApiPromise | null = null
	public cacheService: CacheService

	constructor(
		private clientManager: ChainClientManager,
		private privateKey: HexString,
	) {
		this.configService = new ChainConfigService()
		this.cacheService = new CacheService()
	}

	/**
	 * Gets the balance of a token for a wallet
	 */
	async getTokenBalance(tokenAddress: string, walletAddress: string, chain: string): Promise<bigint> {
		const client = this.clientManager.getPublicClient(chain)

		if (tokenAddress === ADDRESS_ZERO) {
			return await client.getBalance({ address: walletAddress as HexString })
		}

		const tokenContract = getContract({
			address: tokenAddress as HexString,
			abi: ERC20_ABI,
			client,
		})

		const balance = await tokenContract.read.balanceOf([walletAddress as HexString])

		return balance
	}

	/**
	 * Gets the decimals for a token
	 */
	async getTokenDecimals(tokenAddress: string, chain: string): Promise<number> {
		const bytes20Address = tokenAddress.length === 66 ? bytes32ToBytes20(tokenAddress) : tokenAddress

		if (bytes20Address === ADDRESS_ZERO) {
			return 18 // Native token (ETH, MATIC, etc.)
		}

		const client = this.clientManager.getPublicClient(chain)

		try {
			const decimals = await client.readContract({
				address: bytes20Address as HexString,
				abi: ERC20_ABI,
				functionName: "decimals",
			})

			return decimals
		} catch (error) {
			console.warn(`Error getting token decimals, defaulting to 18:`, error)
			return 18 // Default to 18 if we can't determine
		}
	}

	/**
	 * Checks if we have sufficient token balances to fill the order
	 */
	async checkTokenBalances(outputs: PaymentInfo[], destChain: string): Promise<boolean> {
		try {
			let totalNativeTokenNeeded = BigInt(0)
			const fillerWalletAddress = privateKeyToAddress(this.privateKey)
			const destClient = this.clientManager.getPublicClient(destChain)

			// Check all token balances
			for (const output of outputs) {
				const tokenAddress = bytes32ToBytes20(output.token)
				const amount = output.amount

				if (tokenAddress === ADDRESS_ZERO) {
					// Native token
					totalNativeTokenNeeded = totalNativeTokenNeeded + amount
				} else {
					// ERC20 token
					const balance = await this.getTokenBalance(tokenAddress, fillerWalletAddress, destChain)

					if (balance < amount) {
						console.debug(
							`Insufficient ${tokenAddress} balance. Have ${balance.toString()}, need ${amount.toString()}`,
						)
						return false
					}
				}
			}

			// Check if we have enough native token
			if (totalNativeTokenNeeded > 0n) {
				const nativeBalance = await destClient.getBalance({ address: fillerWalletAddress })

				if (BigInt(nativeBalance.toString()) < totalNativeTokenNeeded) {
					console.debug(
						`Insufficient native token balance. Have ${nativeBalance.toString()}, need ${totalNativeTokenNeeded.toString()}`,
					)
					return false
				}
			}

			return true
		} catch (error) {
			console.error(`Error checking token balances:`, error)
			return false
		}
	}

	/**
	 * Approves ERC20 tokens for the contract if needed
	 */
	async approveTokensIfNeeded(order: Order): Promise<void> {
		const uniqueTokens: string[] = []
		const wallet = privateKeyToAccount(this.privateKey)
		const outputs = order.outputs
		const destClient = this.clientManager.getPublicClient(order.destChain)
		const walletClient = this.clientManager.getWalletClient(order.destChain)
		const intentGateway = this.configService.getIntentGatewayAddress(order.destChain)

		// Collect unique ERC20 tokens
		for (const output of outputs) {
			const bytes20Address = bytes32ToBytes20(output.token)
			if (bytes20Address !== ADDRESS_ZERO) {
				uniqueTokens.push(bytes20Address)
			}
		}

		// Approve each token
		for (const tokenAddress of [...uniqueTokens, (await this.getFeeTokenWithDecimals(order.destChain)).address]) {
			const currentAllowance = await destClient.readContract({
				abi: ERC20_ABI,
				address: tokenAddress as HexString,
				functionName: "allowance",
				args: [wallet.address, intentGateway],
			})

			// If allowance is too low, approve a very large amount
			if (currentAllowance < maxUint256) {
				console.log(`Approving ${tokenAddress} for the contract`)

				const { request } = await destClient.simulateContract({
					abi: ERC20_ABI,
					address: tokenAddress as HexString,
					functionName: "approve",
					args: [intentGateway, maxUint256],
					account: wallet,
				})

				const tx = await walletClient.writeContract(request)
				await destClient.waitForTransactionReceipt({ hash: tx })
				console.log(`Approval confirmed for ${tokenAddress}`)
			}
		}
	}

	/**
	 * Calculates the ETH value to send with the transaction
	 */
	calculateRequiredEthValue(outputs: PaymentInfo[]): bigint {
		let totalEthValue = 0n

		for (const output of outputs) {
			const bytes20Address = bytes32ToBytes20(output.token)
			if (bytes20Address === ADDRESS_ZERO) {
				// Native token output
				totalEthValue = totalEthValue + output.amount
			}
		}

		return totalEthValue
	}

	/**
	 * Transforms the order object to match the contract's expected format
	 */
	transformOrderForContract(order: Order) {
		return {
			sourceChain: toHex(order.sourceChain),
			destChain: toHex(order.destChain),
			fees: order.fees,
			callData: order.callData,
			deadline: order.deadline,
			nonce: order.nonce,
			inputs: order.inputs.map((input) => ({
				token: input.token,
				amount: input.amount,
			})),
			outputs: order.outputs.map((output) => ({
				token: output.token,
				amount: output.amount,
				beneficiary: output.beneficiary,
			})),
			user: order.user,
		}
	}

	/**
	 * Checks if an order is already filled by querying contract storage
	 */
	async checkIfOrderFilled(order: Order): Promise<boolean> {
		try {
			const commitment = orderCommitment(order)
			const sourceClient = this.clientManager.getPublicClient(order.sourceChain)
			const intentGatewayAddress = this.configService.getIntentGatewayAddress(order.sourceChain)

			const mappingSlot = 5n

			const filledSlot = keccak256(encodePacked(["bytes32", "uint256"], [commitment, mappingSlot]))

			const filledStatus = await sourceClient.getStorageAt({
				address: intentGatewayAddress,
				slot: filledSlot,
			})
			return filledStatus !== "0x0000000000000000000000000000000000000000000000000000000000000000"
		} catch (error) {
			console.error(`Error checking if order filled:`, error)
			// Default to assuming it's not filled if we can't check
			return false
		}
	}

	/**
	 * Estimates gas for filling an order
	 */
	async estimateGasFillPost(
		order: Order,
	): Promise<{ fillGas: bigint; postGas: bigint; relayerFeeInFeeToken: bigint }> {
		try {
			// Check cache first
			const cachedEstimate = this.cacheService.getGasEstimate(order.id!)
			if (cachedEstimate) {
				console.log(`Using cached gas estimate for order ${order.id}`)
				return cachedEstimate
			}

			const { sourceClient, destClient } = this.clientManager.getClientsForOrder(order)
			const postRequest: IPostRequest = {
				source: order.destChain,
				dest: order.sourceChain,
				body: constructRedeemEscrowRequestBody(order, privateKeyToAddress(this.privateKey)),
				timeoutTimestamp: 0n,
				nonce: await this.getHostNonce(order.sourceChain),
				from: this.configService.getIntentGatewayAddress(order.destChain),
				to: this.configService.getIntentGatewayAddress(order.sourceChain),
			}

			let postGasEstimate = await estimateGasForPost({
				postRequest: postRequest,
				sourceClient: sourceClient as any,
				hostLatestStateMachineHeight: await this.getHostLatestStateMachineHeight(),
				hostAddress: this.configService.getHostAddress(order.sourceChain),
			})

			const { decimals: destFeeTokenDecimals } = await this.getFeeTokenWithDecimals(order.destChain)

			// Add 2% markup
			postGasEstimate = postGasEstimate + (postGasEstimate * 200n) / 10000n

			const postGasEstimateInDestFeeToken = await this.convertGasToFeeToken(
				postGasEstimate,
				order.sourceChain,
				destFeeTokenDecimals,
			)

			const fillOptions: FillOptions = {
				relayerFee: postGasEstimateInDestFeeToken,
			}

			const ethValue = this.calculateRequiredEthValue(order.outputs)
			const userAddress = privateKeyToAddress(this.privateKey)
			const testValue = toHex(maxUint256 / 2n)
			const intentGatewayAddress = this.configService.getIntentGatewayAddress(order.destChain)

			const overrides = (
				await Promise.all(
					order.outputs.map(async (output) => {
						const tokenAddress = bytes32ToBytes20(output.token)
						if (tokenAddress === ADDRESS_ZERO) return null

						try {
							const balanceData = ERC20Method.BALANCE_OF + bytes20ToBytes32(userAddress).slice(2)
							const balanceSlot = await getStorageSlot(
								destClient as any,
								tokenAddress,
								balanceData as HexString,
							)
							const stateDiffs = [{ slot: balanceSlot as HexString, value: testValue }]

							try {
								const allowanceData =
									ERC20Method.ALLOWANCE +
									bytes20ToBytes32(userAddress).slice(2) +
									bytes20ToBytes32(intentGatewayAddress).slice(2)
								const allowanceSlot = await getStorageSlot(
									destClient as any,
									tokenAddress,
									allowanceData as HexString,
								)
								stateDiffs.push({ slot: allowanceSlot as HexString, value: testValue })
							} catch (e) {
								console.warn(`Could not find allowance slot for token ${tokenAddress}`, e)
							}

							return { address: tokenAddress, stateDiff: stateDiffs }
						} catch (e) {
							console.warn(`Could not find balance slot for token ${tokenAddress}`, e)
							return null
						}
					}),
				)
			).filter(Boolean)

			const destChainFeeTokenAddress = (await this.getFeeTokenWithDecimals(order.destChain)).address

			const destFeeTokenBalanceData = ERC20Method.BALANCE_OF + bytes20ToBytes32(userAddress).slice(2)
			const destFeeTokenBalanceSlot = await getStorageSlot(
				destClient as any,
				destChainFeeTokenAddress,
				destFeeTokenBalanceData as HexString,
			)
			const destFeeTokenAllowanceData =
				ERC20Method.ALLOWANCE +
				bytes20ToBytes32(userAddress).slice(2) +
				bytes20ToBytes32(intentGatewayAddress).slice(2)
			const destFeeTokenAllowanceSlot = await getStorageSlot(
				destClient as any,
				destChainFeeTokenAddress,
				destFeeTokenAllowanceData as HexString,
			)
			const feeTokenStateDiffs = [
				{ slot: destFeeTokenBalanceSlot, value: testValue },
				{ slot: destFeeTokenAllowanceSlot, value: testValue },
			]

			overrides.push({
				address: destChainFeeTokenAddress,
				stateDiff: feeTokenStateDiffs as any,
			})

			const gas = await destClient.estimateContractGas({
				abi: INTENT_GATEWAY_ABI,
				address: this.configService.getIntentGatewayAddress(order.destChain),
				functionName: "fillOrder",
				args: [this.transformOrderForContract(order), fillOptions as any],
				account: privateKeyToAccount(this.privateKey),
				value: ethValue,
				stateOverride: overrides as any,
			})

			// Cache the results
			this.cacheService.setGasEstimate(order.id!, gas, postGasEstimate, postGasEstimateInDestFeeToken)

			return { fillGas: gas, postGas: postGasEstimate, relayerFeeInFeeToken: postGasEstimateInDestFeeToken }
		} catch (error) {
			console.error(`Error estimating gas:`, error)
			// Return a conservative estimate if we can't calculate precisely
			return { fillGas: 3000000n, postGas: 270000n, relayerFeeInFeeToken: 10000000n }
		}
	}

	/**
	 * Gets the current Native token price (including Decimals)
	 */
	async getNativeTokenPrice(chain: string): Promise<bigint> {
		let client = this.clientManager.getPublicClient(chain)
		const nativeToken = client.chain?.nativeCurrency

		if (!nativeToken?.symbol || !nativeToken?.decimals) {
			throw new Error("Chain native currency information not available")
		}

		const nativeTokenPriceUsd = await fetchTokenUsdPrice(nativeToken.symbol)

		return BigInt(Math.floor(nativeTokenPriceUsd * Math.pow(10, 18)))
	}

	async convertGasToFeeToken(gasEstimate: bigint, chain: string, targetDecimals: number): Promise<bigint> {
		const client = this.clientManager.getPublicClient(chain)
		const gasPrice = await client.getGasPrice()
		const gasCostInWei = gasEstimate * gasPrice
		const nativeToken = client.chain?.nativeCurrency

		if (!nativeToken?.symbol || !nativeToken?.decimals) {
			throw new Error("Chain native currency information not available")
		}

		const gasCostInToken = Number(gasCostInWei) / Math.pow(10, nativeToken.decimals)
		const tokenPriceUsd = await fetchTokenUsdPrice(nativeToken.symbol)
		const gasCostUsd = gasCostInToken * tokenPriceUsd

		const feeTokenPriceUsd = await fetchTokenUsdPrice("DAI") // Using DAI as default
		let gasCostInFeeToken = gasCostUsd / feeTokenPriceUsd

		return BigInt(Math.floor(gasCostInFeeToken * Math.pow(10, targetDecimals)))
	}

	async getFeeTokenWithDecimals(chain: string): Promise<{ address: HexString; decimals: number }> {
		const client = this.clientManager.getPublicClient(chain)
		const hostParams = await client.readContract({
			abi: EVM_HOST,
			address: this.configService.getHostAddress(chain),
			functionName: "hostParams",
		})
		const feeTokenAddress = hostParams.feeToken
		const feeTokenDecimals = await client.readContract({
			address: feeTokenAddress,
			abi: ERC20_ABI,
			functionName: "decimals",
		})
		return { address: feeTokenAddress, decimals: feeTokenDecimals }
	}

	/**
	 * Calculates the fee required to send a post request to the destination chain.
	 * The fee is calculated based on the per-byte fee for the destination chain
	 * multiplied by the size of the request body.
	 *
	 * @param request - The post request to calculate the fee for
	 * @returns The total fee in wei required to send the post request
	 */
	async quote(order: Order): Promise<bigint> {
		const { destClient } = this.clientManager.getClientsForOrder(order)
		const postRequest: IPostRequest = {
			source: order.destChain,
			dest: order.sourceChain,
			body: constructRedeemEscrowRequestBody(order, privateKeyToAddress(this.privateKey)),
			timeoutTimestamp: 0n,
			nonce: await this.getHostNonce(order.sourceChain),
			from: this.configService.getIntentGatewayAddress(order.destChain),
			to: this.configService.getIntentGatewayAddress(order.sourceChain),
		}
		const perByteFee = await destClient.readContract({
			address: this.configService.getHostAddress(order.destChain),
			abi: EVM_HOST,
			functionName: "perByteFee",
			args: [toHex(order.sourceChain)],
		})

		// Exclude 0x prefix from the body length, and get the byte length
		const bodyByteLength = Math.floor((postRequest.body.length - 2) / 2)
		const length = bodyByteLength < 32 ? 32 : bodyByteLength

		return perByteFee * BigInt(length)
	}

	/**
	 * Gets the host nonce
	 */
	async getHostNonce(chain: string): Promise<bigint> {
		const client = this.clientManager.getPublicClient(chain)
		const nonce = await client.readContract({
			abi: EVM_HOST,
			address: this.configService.getHostAddress(chain),
			functionName: "nonce",
		})

		return nonce
	}

	/**
	 * Gets the host latest state machine height
	 */
	async getHostLatestStateMachineHeight(chain?: string): Promise<bigint> {
		if (!this.api) {
			this.api = await ApiPromise.create({
				provider: new WsProvider(this.configService.getRpcUrl(Chains.HYPERBRIDGE_GARGANTUA)),
				typesBundle: {
					spec: {
						gargantua: {
							hasher: keccakAsU8a,
						},
					},
				},
			})
			if (!(await this.api.isConnected)) {
				await this.api.connect()
			}
		}
		let latestHeight: any

		if (chain) {
			latestHeight = await this.api.query.ismp.latestStateMachineHeight({
				stateId: {
					Evm: this.configService.getChainId(chain),
				},
				consensusStateId: this.configService.getConsensusStateId(chain),
			})

			return BigInt(latestHeight.toString())
		}

		latestHeight = await this.api.query.system.number()

		return BigInt(latestHeight.toString())
	}

	async getTokenUsdValue(order: Order): Promise<{ outputUsdValue: bigint; inputUsdValue: bigint }> {
		const { destClient, sourceClient } = this.clientManager.getClientsForOrder(order)
		let outputUsdValue = BigInt(0)
		let inputUsdValue = BigInt(0)
		const outputs = order.outputs
		const inputs = order.inputs

		for (const output of outputs) {
			let tokenAddress = bytes32ToBytes20(output.token)
			let decimals = 18
			let amount = output.amount
			let priceIdentifier: string

			if (tokenAddress === ADDRESS_ZERO) {
				priceIdentifier = destClient.chain?.nativeCurrency?.symbol!
				decimals = destClient.chain?.nativeCurrency?.decimals!
			} else {
				decimals = await this.getTokenDecimals(tokenAddress, order.destChain)
				priceIdentifier = tokenAddress
			}

			// Always use 18 decimals for USD due to multiple token decimal inconsistencies
			const pricePerToken = await this.getTokenPrice(priceIdentifier, 18)
			const tokenUsdValue = (amount * pricePerToken) / BigInt(10 ** decimals)
			outputUsdValue = outputUsdValue + tokenUsdValue
		}

		for (const input of inputs) {
			let tokenAddress = bytes32ToBytes20(input.token)
			let decimals = 18
			let amount = input.amount
			let priceIdentifier: string

			if (tokenAddress === ADDRESS_ZERO) {
				priceIdentifier = sourceClient.chain?.nativeCurrency?.symbol!
				decimals = sourceClient.chain?.nativeCurrency?.decimals!
			} else {
				decimals = await this.getTokenDecimals(tokenAddress, order.sourceChain)
				priceIdentifier = tokenAddress
			}

			// Always use 18 decimals for USD due to multiple token decimal inconsistencies
			const pricePerToken = await this.getTokenPrice(priceIdentifier, 18)
			const tokenUsdValue = (amount * pricePerToken) / BigInt(10 ** decimals)
			inputUsdValue = inputUsdValue + tokenUsdValue
		}

		return { outputUsdValue, inputUsdValue }
	}

	async getTokenPrice(tokenAddress: string, decimals: number): Promise<bigint> {
		const usdValue = await fetchTokenUsdPrice(tokenAddress)
		return BigInt(Math.floor(usdValue * Math.pow(10, decimals)))
	}

	async getFillerBalanceUSD(chain: string): Promise<{
		nativeTokenBalance: bigint
		daiBalance: bigint
		usdtBalance: bigint
		usdcBalance: bigint
		totalBalanceUsd: bigint
	}> {
		const fillerWalletAddress = privateKeyToAddress(this.privateKey)
		const destClient = this.clientManager.getPublicClient(chain)

		const nativeTokenBalance = await destClient.getBalance({ address: fillerWalletAddress })
		const nativeTokenPrice = await this.getNativeTokenPrice(chain)
		const nativeDecimals = destClient.chain?.nativeCurrency?.decimals || 18
		const nativeTokenUsdValue = (nativeTokenBalance * nativeTokenPrice) / BigInt(10 ** nativeDecimals)

		const daiAddress = this.configService.getDaiAsset(chain)
		const daiBalance = await destClient.readContract({
			abi: ERC20_ABI,
			address: daiAddress,
			functionName: "balanceOf",
			args: [fillerWalletAddress],
		})
		const daiDecimals = await this.getTokenDecimals(daiAddress, chain)
		const daiPrice = await this.getTokenPrice(daiAddress, 18) // Normalize to 18 decimals
		const daiBalanceUsd = (daiBalance * daiPrice) / BigInt(10 ** daiDecimals)

		const usdtAddress = this.configService.getUsdtAsset(chain)
		const usdtBalance = await destClient.readContract({
			abi: ERC20_ABI,
			address: usdtAddress,
			functionName: "balanceOf",
			args: [fillerWalletAddress],
		})
		const usdtDecimals = await this.getTokenDecimals(usdtAddress, chain)
		const usdtPrice = await this.getTokenPrice(usdtAddress, 18) // Normalize to 18 decimals
		const usdtBalanceUsd = (usdtBalance * usdtPrice) / BigInt(10 ** usdtDecimals)

		const usdcAddress = this.configService.getUsdcAsset(chain)
		const usdcBalance = await destClient.readContract({
			abi: ERC20_ABI,
			address: usdcAddress,
			functionName: "balanceOf",
			args: [fillerWalletAddress],
		})
		const usdcDecimals = await this.getTokenDecimals(usdcAddress, chain)
		const usdcPrice = await this.getTokenPrice(usdcAddress, 18) // Normalize to 18 decimals
		const usdcBalanceUsd = (usdcBalance * usdcPrice) / BigInt(10 ** usdcDecimals)

		const totalBalanceUsd = nativeTokenUsdValue + daiBalanceUsd + usdtBalanceUsd + usdcBalanceUsd

		return {
			nativeTokenBalance,
			daiBalance,
			usdtBalance,
			usdcBalance,
			totalBalanceUsd,
		}
	}

	async getV2Quote(tokenIn: HexString, tokenOut: HexString, amountOut: bigint, destChain: string): Promise<bigint> {
		try {
			const v2Router = this.configService.getUniswapRouterV2Address(destChain)
			const v2Factory = this.configService.getUniswapV2FactoryAddress(destChain)
			const destClient = this.clientManager.getPublicClient(destChain)

			// For V2/V3, convert native addresses to WETH for quotes
			const wethAsset = this.configService.getWrappedNativeAssetWithDecimals(destChain).asset
			const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
			const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

			const v2PairExists = (await destClient.readContract({
				address: v2Factory,
				abi: UNISWAP_V2_FACTORY_ABI,
				functionName: "getPair",
				args: [tokenInForQuote, tokenOutForQuote],
			})) as HexString

			if (v2PairExists !== ADDRESS_ZERO) {
				const v2AmountIn = (await destClient.readContract({
					address: v2Router,
					abi: UNISWAP_ROUTER_V2_ABI,
					functionName: "getAmountsIn",
					args: [amountOut, [tokenInForQuote, tokenOutForQuote]],
				})) as bigint[]

				return v2AmountIn[0]
			}

			return maxUint256
		} catch (error) {
			console.warn("V2 quote failed:", error)
			return maxUint256
		}
	}

	async getV3Quote(
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		destChain: string,
	): Promise<{ amountIn: bigint; fee: number }> {
		const commonFees = [100, 500, 3000, 10000]
		let bestAmountIn = maxUint256
		let bestFee = 0

		const v3Factory = this.configService.getUniswapV3FactoryAddress(destChain)
		const v3Quoter = this.configService.getUniswapV3QuoterAddress(destChain)
		const destClient = this.clientManager.getPublicClient(destChain)

		// For V2/V3, convert native addresses to WETH for quotes
		const wethAsset = this.configService.getWrappedNativeAssetWithDecimals(destChain).asset
		const tokenInForQuote = tokenIn === ADDRESS_ZERO ? wethAsset : tokenIn
		const tokenOutForQuote = tokenOut === ADDRESS_ZERO ? wethAsset : tokenOut

		for (const fee of commonFees) {
			try {
				const pool = await destClient.readContract({
					address: v3Factory,
					abi: UNISWAP_V3_FACTORY_ABI,
					functionName: "getPool",
					args: [tokenInForQuote, tokenOutForQuote, fee],
				})

				if (pool !== ADDRESS_ZERO) {
					const liquidity = await destClient.readContract({
						address: pool,
						abi: UNISWAP_V3_POOL_ABI,
						functionName: "liquidity",
					})

					if (liquidity > BigInt(0)) {
						// Use simulateContract for V3 quoter (handles revert-based returns)
						const quoteResult = (
							await destClient.simulateContract({
								address: v3Quoter,
								abi: UNISWAP_V3_QUOTER_V2_ABI,
								functionName: "quoteExactOutputSingle",
								args: [
									{
										tokenIn: tokenInForQuote,
										tokenOut: tokenOutForQuote,
										fee: fee,
										amount: amountOut,
										sqrtPriceLimitX96: BigInt(0),
									},
								],
							})
						).result as [bigint, bigint, number, bigint] // [amountIn, sqrtPriceX96After, initializedTicksCrossed, gasEstimate]

						const amountIn = quoteResult[0]

						if (amountIn < bestAmountIn) {
							bestAmountIn = amountIn
							bestFee = fee
						}
					}
				}
			} catch (error) {
				console.warn(`V3 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		return { amountIn: bestAmountIn, fee: bestFee }
	}

	async getV4Quote(
		tokenIn: HexString,
		tokenOut: HexString,
		amountOut: bigint,
		destChain: string,
	): Promise<{ amountIn: bigint; fee: number }> {
		const commonFees = [100, 500, 3000, 10000]
		let bestAmountIn = maxUint256
		let bestFee = 0

		const v4Quoter = this.configService.getUniswapV4QuoterAddress(destChain)
		const destClient = this.clientManager.getPublicClient(destChain)

		for (const fee of commonFees) {
			try {
				const currency0 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenIn : tokenOut
				const currency1 = tokenIn.toLowerCase() < tokenOut.toLowerCase() ? tokenOut : tokenIn

				const zeroForOne = tokenIn.toLowerCase() === currency0.toLowerCase()

				const poolKey = {
					currency0: currency0,
					currency1: currency1,
					fee: fee,
					tickSpacing: this.getTickSpacing(fee),
					hooks: ADDRESS_ZERO, // No hooks
				}

				const quoteResult = (
					await destClient.simulateContract({
						address: v4Quoter,
						abi: UNISWAP_V4_QUOTER_ABI,
						functionName: "quoteExactOutputSingle",
						args: [
							{
								poolKey: poolKey,
								zeroForOne: zeroForOne,
								exactAmount: amountOut,
								hookData: "0x", // Empty hook data
							},
						],
					})
				).result as [bigint, bigint] // [amountIn, gasEstimate]

				const amountIn = quoteResult[0]

				if (amountIn < bestAmountIn) {
					console.log("Found a better amountIn with fee", amountIn, fee)
					bestAmountIn = amountIn
					bestFee = fee
				}
			} catch (error) {
				console.warn(`V4 quote failed for fee ${fee}, continuing to next fee tier`)
			}
		}

		return { amountIn: bestAmountIn, fee: bestFee }
	}

	createV2SwapCalldata(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountOut: bigint,
		amountInMax: bigint,
		recipient: HexString,
		assets: TokenAssets,
	): { commands: HexString; inputs: HexString[] } {
		const V2_SWAP_EXACT_OUT = 0x09
		const isPermit2 = false

		const swapSourceAddress = sourceTokenAddress === ADDRESS_ZERO ? assets.wethAsset : sourceTokenAddress
		const swapTargetAddress = targetTokenAddress === ADDRESS_ZERO ? assets.wethAsset : targetTokenAddress

		const path = [swapSourceAddress, swapTargetAddress]
		const commands = encodePacked(["uint8"], [V2_SWAP_EXACT_OUT])
		const inputs = [
			encodeAbiParameters(
				[
					{ type: "address", name: "recipient" },
					{ type: "uint256", name: "amountOut" },
					{ type: "uint256", name: "amountInMax" },
					{ type: "address[]", name: "path" },
					{ type: "bool", name: "isPermit2" },
				],
				[recipient, amountOut, amountInMax, path, isPermit2],
			),
		]

		return { commands, inputs }
	}

	createV3SwapCalldata(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountOut: bigint,
		amountInMax: bigint,
		fee: number,
		recipient: HexString,
		assets: TokenAssets,
	): { commands: HexString; inputs: HexString[] } {
		const V3_SWAP_EXACT_OUT = 0x01
		const isPermit2 = false

		const swapSourceAddress = sourceTokenAddress === ADDRESS_ZERO ? assets.wethAsset : sourceTokenAddress
		const swapTargetAddress = targetTokenAddress === ADDRESS_ZERO ? assets.wethAsset : targetTokenAddress

		const pathV3 = encodePacked(["address", "uint24", "address"], [swapTargetAddress, fee, swapSourceAddress])
		const commands = encodePacked(["uint8"], [V3_SWAP_EXACT_OUT])
		const inputs = [
			encodeAbiParameters(
				[
					{ type: "address", name: "recipient" },
					{ type: "uint256", name: "amountOut" },
					{ type: "uint256", name: "amountInMax" },
					{ type: "bytes", name: "path" },
					{ type: "bool", name: "isPermit2" },
				],
				[recipient, amountOut, amountInMax, pathV3, isPermit2],
			),
		]

		return { commands, inputs }
	}

	createV4SwapCalldata(
		sourceTokenAddress: HexString,
		targetTokenAddress: HexString,
		amountOut: bigint,
		amountInMax: bigint,
		fee: number,
	): { commands: HexString; inputs: HexString[] } {
		const V4_SWAP = 0x10

		const currency0 =
			sourceTokenAddress.toLowerCase() < targetTokenAddress.toLowerCase()
				? sourceTokenAddress
				: targetTokenAddress
		const currency1 =
			sourceTokenAddress.toLowerCase() < targetTokenAddress.toLowerCase()
				? targetTokenAddress
				: sourceTokenAddress

		const zeroForOne = sourceTokenAddress.toLowerCase() === currency0.toLowerCase()

		const poolKey = {
			currency0: currency0,
			currency1: currency1,
			fee: fee,
			tickSpacing: this.getTickSpacing(fee),
			hooks: ADDRESS_ZERO,
		}

		const SWAP_EXACT_OUT_SINGLE = 0x08
		const SETTLE_ALL = 0x0c
		const TAKE_ALL = 0x0f

		const actions = encodePacked(["uint8", "uint8", "uint8"], [SWAP_EXACT_OUT_SINGLE, SETTLE_ALL, TAKE_ALL])

		const swapParams = encodeAbiParameters(
			[
				{
					type: "tuple",
					name: "ExactOutputSingleParams",
					components: [
						{
							type: "tuple",
							name: "poolKey",
							components: [
								{ type: "address", name: "currency0" },
								{ type: "address", name: "currency1" },
								{ type: "uint24", name: "fee" },
								{ type: "int24", name: "tickSpacing" },
								{ type: "address", name: "hooks" },
							],
						},
						{ type: "bool", name: "zeroForOne" },
						{ type: "uint128", name: "amountOut" },
						{ type: "uint128", name: "amountInMaximum" },
						{ type: "bytes", name: "hookData" },
					],
				},
			],
			[
				{
					poolKey,
					zeroForOne: zeroForOne,
					amountOut: amountOut,
					amountInMaximum: amountInMax,
					hookData: "0x",
				},
			],
		)

		const settleParams = encodeAbiParameters(
			[
				{ type: "address", name: "currency" },
				{ type: "uint128", name: "amount" },
			],
			[sourceTokenAddress, amountInMax],
		)

		const takeParams = encodeAbiParameters(
			[
				{ type: "address", name: "currency" },
				{ type: "uint128", name: "amount" },
			],
			[targetTokenAddress, amountOut],
		)

		const params = [swapParams, settleParams, takeParams]

		const commands = encodePacked(["uint8"], [V4_SWAP])
		const inputs = [
			encodeAbiParameters(
				[
					{ type: "bytes", name: "actions" },
					{ type: "bytes[]", name: "params" },
				],
				[actions, params],
			),
		]

		return { commands, inputs }
	}

	getTickSpacing(fee: number): number {
		switch (fee) {
			case 100: // 0.01%
				return 1
			case 500: // 0.05%
				return 10
			case 3000: // 0.30%
				return 60
			case 10000: // 1.00%
				return 200
			default:
				return 60 // Default to medium
		}
	}
}

export interface TokenBalances {
	dai: bigint
	usdt: bigint
	usdc: bigint
	native: bigint
}

export interface TokenAssets {
	daiAsset: HexString
	usdtAsset: HexString
	usdcAsset: HexString
	wethAsset: HexString
}

export interface TokenDecimals {
	daiDecimals: number
	usdtDecimals: number
	usdcDecimals: number
}

export interface SwapContext {
	contractService: ContractInteractionService
	fillerWalletAddress: HexString
	destClient: PublicClient
	destChain: string
	assets: TokenAssets
	decimals: TokenDecimals
	initialBalances: TokenBalances
	universalRouterAddress: HexString
}

export interface SwapContextWithRequirements extends SwapContext {
	remainingBalances: TokenBalances
	shortfalls: TokenBalances
}

export interface BestProtocol {
	protocol: "v2" | "v3" | "v4" | null
	amountIn: bigint
	fee?: number
}

export type TokenType = keyof TokenBalances
