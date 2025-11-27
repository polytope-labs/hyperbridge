import { getContract, toHex, encodePacked, keccak256, maxUint256, formatUnits, parseUnits } from "viem"
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
	estimateGasForPost,
	constructRedeemEscrowRequestBody,
	IPostRequest,
	getStorageSlot,
	ERC20Method,
	fetchPrice,
	maxBigInt,
	getGasPriceFromEtherscan,
	USE_ETHERSCAN_CHAINS,
	retryPromise,
	adjustFeeDecimals,
} from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ChainClientManager } from "./ChainClientManager"
import { FillerConfigService } from "./FillerConfigService"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { orderCommitment } from "@hyperbridge/sdk"
import { ApiPromise, WsProvider } from "@polkadot/api"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { CacheService } from "./CacheService"
import { UNISWAP_ROUTER_V2_ABI } from "@/config/abis/UniswapRouterV2"
import { getLogger } from "@/services/Logger"
import { Decimal } from "decimal.js"

// Configure for financial precision
Decimal.config({ precision: 28, rounding: 4 })
/**
 * Handles contract interactions for tokens and other contracts
 */
export class ContractInteractionService {
	private configService: FillerConfigService
	private api: ApiPromise | null = null
	public cacheService: CacheService
	private logger = getLogger("contract-service")

	constructor(
		private clientManager: ChainClientManager,
		private privateKey: HexString,
		configService: FillerConfigService,
		sharedCacheService?: CacheService,
	) {
		this.configService = configService
		this.cacheService = sharedCacheService || new CacheService()
		this.initCache()
	}

	async initCache(): Promise<void> {
		const chainIds = this.configService.getConfiguredChainIds()
		const chainNames = chainIds.map((id) => `EVM-${id}`)
		for (const chainName of chainNames) {
			await this.getFeeTokenWithDecimals(chainName)
		}

		for (const destChain of chainNames) {
			const destClient = this.clientManager.getPublicClient(destChain)
			const usdc = this.configService.getUsdcAsset(destChain)
			const usdt = this.configService.getUsdtAsset(destChain)
			await this.getTokenDecimals(usdc, destChain)
			await this.getTokenDecimals(usdt, destChain)
			for (const sourceChain of chainNames) {
				if (sourceChain === destChain) continue
				// Check cache before making RPC call to avoid duplicate requests when cache is shared
				const cachedPerByteFee = this.cacheService.getPerByteFee(destChain, sourceChain)
				if (cachedPerByteFee === null) {
					const perByteFee = await destClient.readContract({
						address: this.configService.getHostAddress(destChain),
						abi: EVM_HOST,
						functionName: "perByteFee",
						args: [toHex(sourceChain)],
					})
					this.cacheService.setPerByteFee(destChain, sourceChain, perByteFee)
				}
			}
		}
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

		const cachedTokenDecimals = this.cacheService.getTokenDecimals(chain, bytes20Address as HexString)
		if (cachedTokenDecimals) {
			return cachedTokenDecimals
		}

		const client = this.clientManager.getPublicClient(chain)

		try {
			const decimals = await client.readContract({
				address: bytes20Address as HexString,
				abi: ERC20_ABI,
				functionName: "decimals",
			})

			this.cacheService.setTokenDecimals(chain, bytes20Address as HexString, decimals)
			return decimals
		} catch (error) {
			this.logger.warn({ err: error }, "Error getting token decimals, defaulting to 18")
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
						this.logger.debug(
							{ tokenAddress, balance: balance.toString(), need: amount.toString() },
							"Insufficient token balance",
						)
						return false
					}
				}
			}

			// Check if we have enough native token
			if (totalNativeTokenNeeded > 0n) {
				const nativeBalance = await destClient.getBalance({ address: fillerWalletAddress })

				if (BigInt(nativeBalance.toString()) < totalNativeTokenNeeded) {
					this.logger.debug(
						{ have: nativeBalance.toString(), need: totalNativeTokenNeeded.toString() },
						"Insufficient native token balance",
					)
					return false
				}
			}

			return true
		} catch (error) {
			this.logger.error({ err: error }, "Error checking token balances")
			return false
		}
	}

	/**
	 * Approves ERC20 tokens for the contract if needed
	 */
	async approveTokensIfNeeded(order: Order): Promise<void> {
		const wallet = privateKeyToAccount(this.privateKey)
		const destClient = this.clientManager.getPublicClient(order.destChain)
		const walletClient = this.clientManager.getWalletClient(order.destChain)
		const intentGateway = this.configService.getIntentGatewayAddress(order.destChain)

		const tokens = [
			...new Set(order.outputs.map((o) => bytes32ToBytes20(o.token)).filter((addr) => addr !== ADDRESS_ZERO)),
			(await this.getFeeTokenWithDecimals(order.destChain)).address,
		].map((address) => ({
			address,
			amount: order.outputs.find((o) => bytes32ToBytes20(o.token) === address)?.amount || maxUint256 / 2n,
		}))

		for (const token of tokens) {
			const allowance = await destClient.readContract({
				abi: ERC20_ABI,
				address: token.address as HexString,
				functionName: "allowance",
				args: [wallet.address, intentGateway],
			})

			if (allowance < token.amount) {
				this.logger.info({ token: token.address }, "Approving token")
				const etherscanApiKey = this.configService.getEtherscanApiKey()
				const chain = order.destChain
				const useEtherscan = USE_ETHERSCAN_CHAINS.has(chain)
				const gasPrice =
					useEtherscan && etherscanApiKey
						? await retryPromise(() => getGasPriceFromEtherscan(order.destChain, etherscanApiKey), {
								maxRetries: 3,
								backoffMs: 250,
							}).catch(async () => {
								this.logger.warn(
									{ chain: order.destChain },
									"Error getting gas price from etherscan, using client's gas price",
								)
								return await destClient.getGasPrice()
							})
						: await destClient.getGasPrice()
				const tx = await walletClient.writeContract({
					abi: ERC20_ABI,
					address: token.address as HexString,
					functionName: "approve",
					args: [intentGateway, maxUint256],
					account: wallet,
					chain: walletClient.chain,
					gasPrice: gasPrice + (gasPrice * 2000n) / 10000n,
				})

				await destClient.waitForTransactionReceipt({ hash: tx })
				this.logger.info({ token: token.address }, "Approved token")
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
			this.logger.error({ err: error, orderId: order.id }, "Error checking if order filled")
			// Default to assuming it's not filled if we can't check
			return false
		}
	}

	/**
	 * Estimates gas for filling an order
	 */
	async estimateGasFillPost(
		order: Order,
	): Promise<{ fillGas: bigint; postGas: bigint; relayerFeeInFeeToken: bigint; relayerFeeInNativeToken: bigint }> {
		try {
			// Check cache first
			const cachedEstimate = this.cacheService.getGasEstimate(order.id!)
			if (cachedEstimate) {
				this.logger.debug({ orderId: order.id }, "Using cached gas estimate for order")
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

			let { gas_fee: postGasEstimate } = await estimateGasForPost({
				postRequest: postRequest,
				sourceClient: sourceClient as any,
				hostLatestStateMachineHeight: 6291991n,
				hostAddress: this.configService.getHostAddress(order.sourceChain),
			})

			const { decimals: destFeeTokenDecimals } = await this.getFeeTokenWithDecimals(order.destChain)

			let postGasEstimateInDestFeeToken = await this.convertGasToFeeToken(
				postGasEstimate,
				order.sourceChain,
				destFeeTokenDecimals,
			)

			const minRelayerFee = 5n * 10n ** BigInt(destFeeTokenDecimals - 2)
			const postGasWithIncentive = postGasEstimateInDestFeeToken + (postGasEstimateInDestFeeToken * 1n) / 100n
			postGasEstimateInDestFeeToken = maxBigInt(postGasWithIncentive, minRelayerFee)

			this.logger.debug(
				{
					orderId: order.id,
					postGasWei: postGasEstimate.toString(),
					postGasInDestFeeToken: postGasEstimateInDestFeeToken.toString(),
					destFeeTokenDecimals,
				},
				"Relayer fee estimates",
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
								this.logger.warn({ tokenAddress, err: e }, "Could not find allowance slot for token")
							}

							return { address: tokenAddress, stateDiff: stateDiffs }
						} catch (e) {
							this.logger.warn({ tokenAddress, err: e }, "Could not find balance slot for token")
							return null
						}
					}),
				)
			).filter(Boolean)

			const stateOverride = [
				{
					address: userAddress,
					balance: maxUint256,
				},
				...overrides.map((override) => ({
					address: override!.address,
					stateDiff: override!.stateDiff,
				})),
			]

			let gas = 0n
			let relayerFeeInNativeToken = 0n

			try {
				let protocolFeeInNativeToken = await this.quoteNative(
					postRequest,
					postGasEstimateInDestFeeToken,
					order.destChain,
				)

				// Add 0.5% markup
				protocolFeeInNativeToken = protocolFeeInNativeToken + (protocolFeeInNativeToken * 50n) / 10000n

				gas = await destClient.estimateContractGas({
					abi: INTENT_GATEWAY_ABI,
					address: this.configService.getIntentGatewayAddress(order.destChain),
					functionName: "fillOrder",
					args: [this.transformOrderForContract(order), fillOptions as any],
					account: privateKeyToAccount(this.privateKey),
					value: ethValue + protocolFeeInNativeToken,
					stateOverride: stateOverride as any,
				})
				this.logger.debug(
					{ orderId: order.id, fillGas: gas.toString(), feeMode: "native" },
					"Estimated fill gas",
				)
				relayerFeeInNativeToken = protocolFeeInNativeToken
			} catch {
				this.logger.warn(
					{ chain: order.destChain },
					"Could not estimate gas with native token fees; trying fee token",
				)
				const destChainFeeTokenAddress = (await this.getFeeTokenWithDecimals(order.destChain)).address

				// Check if fee token matches any order output
				const feeTokenMatchesOrderOutput = order.outputs.some(
					(output) => bytes32ToBytes20(output.token.toLowerCase()) === destChainFeeTokenAddress.toLowerCase(),
				)

				if (!feeTokenMatchesOrderOutput) {
					// Only create fee token overrides if it doesn't match any order output
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

					stateOverride.push({
						address: destChainFeeTokenAddress,
						stateDiff: feeTokenStateDiffs as any,
					})
				}

				gas = await destClient.estimateContractGas({
					abi: INTENT_GATEWAY_ABI,
					address: this.configService.getIntentGatewayAddress(order.destChain),
					functionName: "fillOrder",
					args: [this.transformOrderForContract(order), fillOptions as any],
					account: privateKeyToAccount(this.privateKey),
					value: ethValue,
					stateOverride: stateOverride as any,
				})
				this.logger.debug(
					{ orderId: order.id, fillGas: gas.toString(), feeMode: "feeToken" },
					"Estimated fill gas",
				)
			}

			// Cache the results
			this.cacheService.setGasEstimate(
				order.id!,
				gas,
				postGasEstimate,
				postGasEstimateInDestFeeToken,
				relayerFeeInNativeToken,
			)

			return {
				fillGas: gas,
				postGas: postGasEstimate,
				relayerFeeInFeeToken: postGasEstimateInDestFeeToken,
				relayerFeeInNativeToken,
			}
		} catch (error) {
			this.logger.error({ err: error }, "Error estimating gas")
			// Return a conservative estimate if we can't calculate precisely
			return { fillGas: 3000000n, postGas: 270000n, relayerFeeInFeeToken: 10000000n, relayerFeeInNativeToken: 0n }
		}
	}

	/**
	 * Gets a quote for the native token cost of dispatching a post request.
	 *
	 * @param postRequest - The post request to quote
	 * @param fee - The fee amount in fee token
	 * @param chain - The chain identifier where the quote will be executed
	 * @returns The native token amount required
	 */
	async quoteNative(postRequest: IPostRequest, fee: bigint, chain: string): Promise<bigint> {
		const client = this.clientManager.getPublicClient(chain)

		const dispatchPost: DispatchPost = {
			dest: toHex(postRequest.dest),
			to: postRequest.to,
			body: postRequest.body,
			timeout: postRequest.timeoutTimestamp,
			fee: fee,
			payer: postRequest.from,
		}

		const quoteNative = await client
			.readContract({
				abi: INTENT_GATEWAY_ABI,
				address: this.configService.getIntentGatewayAddress(postRequest.dest),
				functionName: "quoteNative",
				args: [dispatchPost] as any,
			})
			.catch(async () => {
				const quoteInFeeToken = await client.readContract({
					abi: INTENT_GATEWAY_ABI,
					address: this.configService.getIntentGatewayAddress(postRequest.dest),
					functionName: "quote",
					args: [dispatchPost] as any,
				})

				const feeToken = (await this.getFeeTokenWithDecimals(chain)).address
				const routerAddr = this.configService.getUniswapRouterV2Address(chain)
				const WETH = this.configService.getWrappedNativeAssetWithDecimals(chain).asset
				const quote = await client.simulateContract({
					abi: UNISWAP_ROUTER_V2_ABI,
					address: routerAddr,
					// @ts-ignore
					functionName: "getAmountsIn",
					// @ts-ignore
					args: [quoteInFeeToken, [WETH, feeToken]],
				})

				return quote.result[0]
			})
		return quoteNative
	}

	/**
	 * Converts gas costs to the equivalent amount in the fee token.
	 * Uses USD pricing to convert between native token gas costs and fee token amounts.
	 *
	 * @param gasEstimate - The estimated gas units
	 * @param chain - The chain identifier to get gas prices and native token info
	 * @param targetDecimals - The decimal places of the target fee token
	 * @returns The gas cost converted to fee token amount
	 */
	async convertGasToFeeToken(gasEstimate: bigint, chain: string, targetDecimals: number): Promise<bigint> {
		const client = this.clientManager.getPublicClient(chain)
		const useEtherscan = USE_ETHERSCAN_CHAINS.has(chain)
		const etherscanApiKey = this.configService.getEtherscanApiKey()
		const gasPrice =
			useEtherscan && etherscanApiKey
				? await retryPromise(() => getGasPriceFromEtherscan(chain, etherscanApiKey), {
						maxRetries: 3,
						backoffMs: 250,
					}).catch(async () => {
						this.logger.warn({ chain }, "Error getting gas price from etherscan, using client's gas price")
						return await client.getGasPrice()
					})
				: await client.getGasPrice()
		const gasCostInWei = gasEstimate * gasPrice

		const routerAddr = this.configService.getUniswapRouterV2Address(chain)
		const wethAddr = this.configService.getWrappedNativeAssetWithDecimals(chain).asset
		const feeToken = await this.getFeeTokenWithDecimals(chain)

		try {
			const quoteIn = await client.simulateContract({
				abi: UNISWAP_ROUTER_V2_ABI,
				address: routerAddr,
				// @ts-ignore
				functionName: "getAmountsIn",
				// @ts-ignore
				args: [gasCostInWei, [feeToken.address, wethAddr]],
			})

			return adjustFeeDecimals(quoteIn.result[0], feeToken.decimals, targetDecimals)
		} catch {
			// Testnet block
			this.logger.warn({ chain }, "On-chain quote failed, falling back to price API")
			const nativeToken = client.chain?.nativeCurrency
			const chainId = client.chain?.id
			if (!nativeToken?.symbol || !nativeToken?.decimals) {
				throw new Error("Chain native currency information not available")
			}
			const gasCostInToken = new Decimal(formatUnits(gasCostInWei, nativeToken.decimals))
			const tokenPriceUsd = new Decimal(
				await retryPromise(() => fetchPrice(nativeToken.symbol, chainId), { maxRetries: 3, backoffMs: 250 }),
			)
			const gasCostUsd = gasCostInToken.times(tokenPriceUsd)
			const feeTokenPriceUsd = new Decimal(1)
			const gasCostInFeeToken = gasCostUsd.dividedBy(feeTokenPriceUsd)
			return parseUnits(gasCostInFeeToken.toFixed(targetDecimals), targetDecimals)
		}
	}

	/**
	 * Gets the fee token address and decimals for a given chain.
	 *
	 * @param chain - The chain identifier to get fee token info for
	 * @returns An object containing the fee token address and its decimal places
	 */
	async getFeeTokenWithDecimals(chain: string): Promise<{ address: HexString; decimals: number }> {
		const cachedFeeToken = this.cacheService.getFeeTokenWithDecimals(chain)
		if (cachedFeeToken) {
			return cachedFeeToken
		}
		const client = this.clientManager.getPublicClient(chain)
		const feeTokenAddress = await client.readContract({
			abi: EVM_HOST,
			address: this.configService.getHostAddress(chain),
			functionName: "feeToken",
		})
		const feeTokenDecimals = await client.readContract({
			address: feeTokenAddress,
			abi: ERC20_ABI,
			functionName: "decimals",
		})
		this.cacheService.setFeeTokenWithDecimals(chain, feeTokenAddress, feeTokenDecimals)
		return { address: feeTokenAddress, decimals: feeTokenDecimals }
	}

	/**
	 * Calculates the fee required to send a post request to the destination chain.
	 * The fee is calculated based on the per-byte fee for the destination chain
	 * multiplied by the size of the request body.
	 *
	 * @param order - The order to calculate the fee for
	 * @returns The total fee in fee token required to send the post request
	 */
	async quote(order: Order): Promise<bigint> {
		const cachedPerByteFee = this.cacheService.getPerByteFee(order.destChain, order.sourceChain)
		if (cachedPerByteFee) {
			return cachedPerByteFee
		}
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
		this.cacheService.setPerByteFee(order.destChain, order.sourceChain, perByteFee)
		// Exclude 0x prefix from the body length, and get the byte length
		const bodyByteLength = Math.floor((postRequest.body.length - 2) / 2)
		const length = bodyByteLength < 32 ? 32 : bodyByteLength

		return perByteFee * BigInt(length)
	}

	/**
	 * Gets the current nonce from the host contract.
	 *
	 * @param chain - The chain identifier to get the host nonce for
	 * @returns The current nonce value
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
	 * Gets the latest state machine height from the host.
	 * If a chain is specified, gets the height for that chain's state machine.
	 * Otherwise, gets the current block number from the Hyperbridge API.
	 *
	 * @param chain - Optional chain identifier to get specific state machine height
	 * @returns The latest state machine height or current block number
	 */
	async getHostLatestStateMachineHeight(chain?: string): Promise<bigint> {
		if (!this.api) {
			// Get hyperbridge RPC URL from config service
			const hyperbridgeRpcUrl = this.configService.getHyperbridgeRpcUrl()

			this.api = await ApiPromise.create({
				provider: new WsProvider(hyperbridgeRpcUrl),
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

	/**
	 * Calculates the total USD value of tokens in an order's inputs and outputs.
	 *
	 * @param order - The order to calculate token values for
	 * @returns An object containing the total USD values of outputs and inputs
	 */
	async getTokenUsdValue(order: Order): Promise<{ outputUsdValue: Decimal; inputUsdValue: Decimal }> {
		let outputUsdValue = new Decimal(0)
		let inputUsdValue = new Decimal(0)
		const outputs = order.outputs
		const inputs = order.inputs

		// Restrict to only USDC and USDT on both sides; otherwise throw error
		const destUsdc = this.configService.getUsdcAsset(order.destChain)
		const destUsdt = this.configService.getUsdtAsset(order.destChain)
		const sourceUsdc = this.configService.getUsdcAsset(order.sourceChain)
		const sourceUsdt = this.configService.getUsdtAsset(order.sourceChain)

		const outputsAreStableOnly = outputs.every((o) => {
			const addr = bytes32ToBytes20(o.token).toLowerCase()
			return addr === destUsdc || addr === destUsdt
		})
		const inputsAreStableOnly = inputs.every((i) => {
			const addr = bytes32ToBytes20(i.token).toLowerCase()
			return addr === sourceUsdc || addr === sourceUsdt
		})

		if (!outputsAreStableOnly || !inputsAreStableOnly) {
			throw new Error("Only USDC and USDT are supported for token value calculation")
		}

		// For stables, USD value equals the normalized token amount (peg ~ $1)
		for (const output of outputs) {
			const tokenAddress = bytes32ToBytes20(output.token)
			const decimals = await this.getTokenDecimals(tokenAddress, order.destChain)
			const amount = output.amount
			const tokenAmount = new Decimal(formatUnits(amount, decimals))
			outputUsdValue = outputUsdValue.plus(tokenAmount)
		}

		for (const input of inputs) {
			const tokenAddress = bytes32ToBytes20(input.token)
			const decimals = await this.getTokenDecimals(tokenAddress, order.sourceChain)
			const amount = input.amount
			const tokenAmount = new Decimal(formatUnits(amount, decimals))
			inputUsdValue = inputUsdValue.plus(tokenAmount)
		}

		return { outputUsdValue, inputUsdValue }
	}
}
