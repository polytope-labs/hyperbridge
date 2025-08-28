import { FillerStrategy } from "@/strategies/base"
import { Order, ExecutionResult, HexString, FillOptions } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { ChainConfigService, fetchTokenUsdPrice } from "@hyperbridge/sdk"

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
	 * Calculates the USD value of the order's inputs, outputs, fees and compares
	 * what will the filler receive and what will the filler pay
	 * @param order The order to calculate the USD value for
	 * @returns The profit in USD (BigInt)
	 */
	async calculateProfitability(order: Order): Promise<bigint> {
		try {
			const { fillGas, relayerFeeInFeeToken } = await this.contractService.estimateGasFillPost(order)

			const protocolFeeInFeeToken = (await this.contractService.quote(order)) + relayerFeeInFeeToken

			const { decimals } = await this.contractService.getFeeTokenWithDecimals(order.destChain)

			const totalGasEstimateInFeeToken =
				(await this.contractService.convertGasToFeeToken(fillGas, order.destChain, decimals)) +
				protocolFeeInFeeToken +
				relayerFeeInFeeToken

			const { outputUsdValue, inputUsdValue } = await this.contractService.getTokenUsdValue(order)

			const orderFeeInUsd = (order.fees * BigInt(10 ** decimals)) / BigInt(10 ** 18)
			const totalGasEstimateInUsd = (totalGasEstimateInFeeToken * BigInt(10 ** decimals)) / BigInt(10 ** 18)

			const toReceive = inputUsdValue + orderFeeInUsd
			const toPay = outputUsdValue + totalGasEstimateInUsd

			const profit = toReceive > toPay ? toReceive - toPay : BigInt(0)

			// Log for debugging
			console.log({
				orderFees: order.fees.toString(),
				totalGasEstimateInFeeToken: totalGasEstimateInFeeToken.toString(),
				profitable: profit > 0,
				profitUsd: profit.toString(),
			})
			return profit
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

			const { relayerFeeInFeeToken } = await this.contractService.estimateGasFillPost(order)
			const fillOptions: FillOptions = {
				relayerFee: relayerFeeInFeeToken,
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
