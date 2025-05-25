import { FillerStrategy } from "@/strategies/base"
import {
	Order,
	ExecutionResult,
	HexString,
	FillOptions,
	estimateGasForPost,
	constructRedeemEscrowRequestBody,
	IPostRequest,
} from "hyperbridge-sdk"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { ChainClientManager, ChainConfigService, ContractInteractionService } from "@/services"

export class BasicFiller implements FillerStrategy {
	name = "BasicFiller"
	private privateKey: HexString
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: ChainConfigService

	constructor(privateKey: HexString) {
		this.privateKey = privateKey
		this.configService = new ChainConfigService()
		this.clientManager = new ChainClientManager(privateKey)
		this.contractService = new ContractInteractionService(this.clientManager, privateKey)
	}

	/**
	 * Determines if this strategy can fill the given order
	 * @param order The order to check
	 * @param config The filler configuration
	 * @returns True if the strategy can fill the order
	 */
	async canFill(order: Order): Promise<boolean> {
		try {
			const destClient = this.clientManager.getPublicClient(order.destChain)
			const currentBlock = await destClient.getBlockNumber()
			const deadline = BigInt(order.deadline)

			if (deadline < currentBlock) {
				console.debug(`Order expired at block ${deadline}, current block ${currentBlock}`)
				return false
			}

			const isAlreadyFilled = await this.contractService.checkIfOrderFilled(order)
			if (isAlreadyFilled) {
				console.debug(`Order is already filled`)
				return false
			}

			const hasEnoughTokens = await this.contractService.checkTokenBalances(order.outputs, order.destChain)
			if (!hasEnoughTokens) {
				console.debug(`Insufficient token balances for order`)
				return false
			}

			return true
		} catch (error) {
			console.error(`Error in canFill:`, error)
			return false
		}
	}

	/**
	 * Calculates the expected profit of filling this order
	 * @param order The order to calculate profit for
	 * @returns The expected profit in DAI (BigInt) which is the order fees minus the total cost
	 */
	async calculateProfitability(order: Order): Promise<bigint> {
		try {
			const { fillGas, postGas } = await this.contractService.estimateGasFillPost(order)
			const nativeTokenPriceUsd = await this.contractService.getNativeTokenPriceUsd(order)

			// 2% on top of postGas
			const relayerFeeEth = postGas + (postGas * BigInt(200)) / BigInt(10000)

			const protocolFeeUSD = await this.contractService.getProtocolFeeUSD(order, relayerFeeEth)

			// fillGas and relayerFeeEth are in wei (10^18)
			// nativeTokenPriceUsd has 18 decimals
			const totalGasWei = fillGas + relayerFeeEth

			// Converting gas cost from wei to USD using the formula:
			// gasCostUsd = (gasWei * priceUsd) / 10^18
			// Result has 18 decimals (matching nativeTokenPriceUsd)
			const gasCostUsd = (totalGasWei * nativeTokenPriceUsd) / BigInt(10 ** 18)

			// Both gasCostUsd and protocolFeeUSD have 18 decimals
			const totalCostUsd = gasCostUsd + protocolFeeUSD

			// Return profitability if positive, otherwise negative
			// order.fees also has 18 decimals
			return order.fees > totalCostUsd ? order.fees - totalCostUsd : BigInt(-1)
		} catch (error) {
			console.error(`Error calculating profitability:`, error)
			return BigInt(0)
		}
	}

	/**
	 * Executes the order fill
	 * @param order The order to fill
	 * @returns The execution result
	 */
	async executeOrder(order: Order): Promise<ExecutionResult> {
		const startTime = Date.now()

		try {
			const { destClient, walletClient } = this.clientManager.getClientsForOrder(order)

			const { postGas: postGasEstimate } = await this.contractService.estimateGasFillPost(order)
			const fillOptions: FillOptions = {
				relayerFee: postGasEstimate + (postGasEstimate * BigInt(200)) / BigInt(10000),
			}

			const ethValue = this.contractService.calculateRequiredEthValue(order.outputs)

			await this.contractService.approveTokensIfNeeded(order)

			const { request } = await destClient.simulateContract({
				abi: INTENT_GATEWAY_ABI,
				address: this.configService.getIntentGatewayAddress(order.destChain),
				functionName: "fillOrder",
				args: [this.contractService.transformOrderForContract(order), fillOptions as any],
				account: privateKeyToAccount(this.privateKey),
				value: ethValue,
			})

			const tx = await walletClient.writeContract(request)

			const endTime = Date.now()
			const processingTimeMs = endTime - startTime

			const receipt = await destClient.waitForTransactionReceipt({ hash: tx })

			return {
				success: true,
				txHash: receipt.transactionHash,
				gasUsed: receipt.gasUsed.toString(),
				gasPrice: receipt.effectiveGasPrice.toString(),
				confirmedAtBlock: Number(receipt.blockNumber),
				confirmedAt: new Date(),
				strategyUsed: this.name,
				processingTimeMs,
			}
		} catch (error) {
			console.error(`Error executing order:`, error)

			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}
}
