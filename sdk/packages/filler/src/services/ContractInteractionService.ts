import { getContract, toHex, encodePacked, keccak256, maxUint256 } from "viem"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import {
	ADDRESS_ZERO,
	Order,
	PaymentInfo,
	HexString,
	FillOptions,
	DispatchPost,
	bytes32ToBytes20,
	HostParams,
	estimateGasForPost,
	constructRedeemEscrowRequestBody,
	IPostRequest,
	bytes20ToBytes32,
	EvmLanguage,
	calculateBalanceMappingLocation,
} from "hyperbridge-sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ChainClientManager } from "./ChainClientManager"
import { ChainConfigService } from "./ChainConfigService"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { orderCommitment } from "hyperbridge-sdk"
import { ApiPromise, WsProvider } from "@polkadot/api"
import { fetchTokenUsdPriceOnchain } from "@/utils"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { Chains } from "@/config/chain"
import { UNISWAP_ROUTER_V2_ABI } from "@/config/abis/UniswapRouterV2"
import { CacheService } from "./CacheService"
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

				// Add some buffer for gas
				const withGasBuffer = totalNativeTokenNeeded + 600000n // 600k gas buffer

				if (BigInt(nativeBalance.toString()) < withGasBuffer) {
					console.debug(
						`Insufficient native token balance. Have ${nativeBalance.toString()}, need ${withGasBuffer.toString()}`,
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
		for (const tokenAddress of [...uniqueTokens, (await this.getHostParams(order.destChain)).feeToken]) {
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
	async estimateGasFillPost(order: Order): Promise<{ fillGas: bigint; postGas: bigint }> {
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
				from: this.configService.getIntentGatewayAddress(order.sourceChain),
				to: this.configService.getIntentGatewayAddress(order.destChain),
			}
			const postGasEstimate = await estimateGasForPost({
				postRequest: postRequest,
				sourceClient: sourceClient as any,
				hostLatestStateMachineHeight: await this.getHostLatestStateMachineHeight(),
				hostAddress: this.configService.getHostAddress(order.sourceChain),
			})

			console.log(`Post gas estimate for filling order ${order.id} on ${order.sourceChain} is ${postGasEstimate}`)

			const fillOptions: FillOptions = {
				relayerFee: postGasEstimate + (postGasEstimate * BigInt(2)) / BigInt(100),
			}

			const ethValue = this.calculateRequiredEthValue(order.outputs)

			// Approve tokens if needed before estimating gas for failsafe
			await this.approveTokensIfNeeded(order)

			// For each output token, generate the storage slot of the token balance
			// Balance slot depends upon implementation of the token contract
			const overrides = (
				await Promise.all(
					order.outputs.map(async (output) => {
						const tokenAddress = bytes32ToBytes20(output.token)
						if (tokenAddress === ADDRESS_ZERO) return null

						const userAddress = privateKeyToAddress(this.privateKey)
						const testValue = toHex(maxUint256)

						// Try both Solidity and Vyper implementations, both have different storage slot implementations
						for (const language of [EvmLanguage.Solidity, EvmLanguage.Vyper]) {
							// Check common slots first (0, 1, 3, 51, 140, 151)
							const commonSlots = [0n, 1n, 3n, 51n, 140n, 151n]

							for (const slot of commonSlots) {
								const storageSlot = calculateBalanceMappingLocation(slot, userAddress, language)

								try {
									const balance = await destClient.readContract({
										abi: ERC20_ABI,
										address: tokenAddress,
										functionName: "balanceOf",
										args: [userAddress],
										stateOverride: [
											{
												address: tokenAddress,
												stateDiff: [{ slot: storageSlot, value: testValue }],
											},
										],
									})

									if (toHex(balance) === testValue) {
										return {
											address: tokenAddress,
											stateDiff: [{ slot: storageSlot, value: testValue }],
										}
									}
								} catch (error) {
									continue
								}
							}
						}

						// Start from slot 0 and go up to 200, but skip already checked slots
						const checkedSlots = new Set([0n, 1n, 3n, 51n, 140n, 151n])

						for (let i = 0n; i < 200n; i++) {
							if (checkedSlots.has(i)) continue

							for (const language of [EvmLanguage.Solidity, EvmLanguage.Vyper]) {
								const storageSlot = calculateBalanceMappingLocation(i, userAddress, language)

								try {
									const balance = await destClient.readContract({
										abi: ERC20_ABI,
										address: tokenAddress,
										functionName: "balanceOf",
										args: [userAddress],
										stateOverride: [
											{
												address: tokenAddress,
												stateDiff: [{ slot: storageSlot, value: testValue }],
											},
										],
									})

									if (toHex(balance) === testValue) {
										return {
											address: tokenAddress,
											stateDiff: [{ slot: storageSlot, value: testValue }],
										}
									}
								} catch (error) {
									continue
								}
							}
						}

						console.warn(`Could not find balance slot for token ${tokenAddress}`)
						return null
					}),
				)
			).filter(Boolean)

			const gas = await destClient.estimateContractGas({
				abi: INTENT_GATEWAY_ABI,
				address: this.configService.getIntentGatewayAddress(order.sourceChain),
				functionName: "fillOrder",
				args: [this.transformOrderForContract(order), fillOptions as any],
				account: privateKeyToAccount(this.privateKey),
				value: ethValue,
				stateOverride: overrides as any,
			})

			console.log(`Gas estimate for filling order ${order.id} on ${order.destChain} is ${gas}`)

			// Cache the results
			this.cacheService.setGasEstimate(order.id!, gas, postGasEstimate)

			return { fillGas: gas, postGas: postGasEstimate }
		} catch (error) {
			console.error(`Error estimating gas:`, error)
			// Return a conservative estimate if we can't calculate precisely
			return { fillGas: 3000000n, postGas: 270000n }
		}
	}

	/**
	 * Gets the current Native token price in USD
	 */
	async getNativeTokenPriceUsd(order: Order): Promise<bigint> {
		const { asset } = this.configService.getWrappedNativeAssetWithDecimals(order.destChain)
		const ethPriceUsd = await fetchTokenUsdPriceOnchain(asset)

		return ethPriceUsd
	}

	/**
	 * Gets the HyperBridge protocol fee in USD (DAI)
	 */
	async getProtocolFeeUSD(order: Order, relayerFee: bigint): Promise<bigint> {
		const destClient = this.clientManager.getPublicClient(order.destChain)
		const intentFillerAddr = privateKeyToAddress(this.privateKey)
		const requestBody = constructRedeemEscrowRequestBody(order, intentFillerAddr)

		const dispatchPost: DispatchPost = {
			dest: toHex(order.sourceChain),
			to: this.configService.getIntentGatewayAddress(order.sourceChain),
			body: requestBody,
			timeout: 0n,
			fee: relayerFee,
			payer: intentFillerAddr,
		}

		const protocolFeeEth = await destClient.readContract({
			abi: INTENT_GATEWAY_ABI,
			address: this.configService.getIntentGatewayAddress(order.destChain),
			functionName: "quote",
			args: [dispatchPost as any],
		})

		return protocolFeeEth
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

	/**
	 * Gets the host params for a given chain
	 */
	async getHostParams(chain: string): Promise<HostParams> {
		const client = this.clientManager.getPublicClient(chain)
		const hostParams = await client.readContract({
			abi: EVM_HOST,
			address: this.configService.getHostAddress(chain),
			functionName: "hostParams",
		})
		return hostParams
	}

	async getTokenUsdValue(order: Order): Promise<{ outputUsdValue: bigint; inputUsdValue: bigint }> {
		let outputUsdValue = BigInt(0)
		let inputUsdValue = BigInt(0)
		const outputs = order.outputs
		const inputs = order.inputs

		for (const output of outputs) {
			const tokenAddress = output.token
			const amount = output.amount
			const decimals = await this.getTokenDecimals(tokenAddress, order.destChain)
			const price = await this.getTokenPrice(tokenAddress)
			const tokenUsdValue = (amount / BigInt(10 ** decimals)) * price
			outputUsdValue = outputUsdValue + tokenUsdValue
		}

		for (const input of inputs) {
			const tokenAddress = input.token
			const amount = input.amount
			const decimals = await this.getTokenDecimals(tokenAddress, order.sourceChain)
			const price = await this.getTokenPrice(tokenAddress)
			const tokenUsdValue = (amount / BigInt(10 ** decimals)) * price
			inputUsdValue = inputUsdValue + tokenUsdValue
		}

		return { outputUsdValue, inputUsdValue }
	}

	async getTokenPrice(tokenAddress: string): Promise<bigint> {
		const usdValue = await fetchTokenUsdPriceOnchain(tokenAddress)
		return usdValue
	}

	async getFillerBalanceUSD(
		order: Order,
		chain: string,
	): Promise<{
		nativeTokenBalance: bigint
		daiBalance: bigint
		usdtBalance: bigint
		usdcBalance: bigint
		totalBalanceUsd: bigint
	}> {
		const fillerWalletAddress = privateKeyToAddress(this.privateKey)
		const destClient = this.clientManager.getPublicClient(chain)

		const nativeTokenBalance = await destClient.getBalance({ address: fillerWalletAddress })
		const nativeTokenPriceUsd = await this.getNativeTokenPriceUsd(order)
		const nativeTokenUsdValue = (nativeTokenBalance / BigInt(10 ** 18)) * nativeTokenPriceUsd

		// DAI balance
		const daiAddress = this.configService.getDaiAsset(chain)
		const daiBalance = await destClient.readContract({
			abi: ERC20_ABI,
			address: daiAddress,
			functionName: "balanceOf",
			args: [fillerWalletAddress],
		})
		const daiDecimals = await this.getTokenDecimals(daiAddress, chain)
		const daiBalanceUsd = daiBalance / BigInt(10 ** daiDecimals)

		// USDT Balance
		const usdtAddress = this.configService.getUsdtAsset(chain)
		const usdtBalance = await destClient.readContract({
			abi: ERC20_ABI,
			address: usdtAddress,
			functionName: "balanceOf",
			args: [fillerWalletAddress],
		})
		const usdtDecimals = await this.getTokenDecimals(usdtAddress, chain)
		const usdtBalanceUsd = usdtBalance / BigInt(10 ** usdtDecimals)

		// USDC Balance
		const usdcAddress = this.configService.getUsdcAsset(chain)
		const usdcBalance = await destClient.readContract({
			abi: ERC20_ABI,
			address: usdcAddress,
			functionName: "balanceOf",
			args: [fillerWalletAddress],
		})
		const usdcDecimals = await this.getTokenDecimals(usdcAddress, chain)
		const usdcBalanceUsd = usdcBalance / BigInt(10 ** usdcDecimals)

		const totalBalanceUsd = nativeTokenUsdValue + daiBalanceUsd + usdtBalanceUsd + usdcBalanceUsd

		return { nativeTokenBalance, daiBalance, usdtBalance, usdcBalance, totalBalanceUsd }
	}
}
