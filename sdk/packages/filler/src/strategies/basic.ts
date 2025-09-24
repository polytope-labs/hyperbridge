import { FillerStrategy } from "@/strategies/base"
import { Order, ExecutionResult, HexString, FillOptions, adjustFeeDecimals, bytes32ToBytes20 } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { privateKeyToAccount } from "viem/accounts"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { FillerConfigService } from "@/services/FillerConfigService"
import { compareDecimalValues } from "@/utils"
import { formatUnits } from "viem"
import { getLogger } from "@/services/Logger"

export class BasicFiller implements FillerStrategy {
	name = "BasicFiller"
	private privateKey: HexString
	private clientManager: ChainClientManager
	private contractService: ContractInteractionService
	private configService: FillerConfigService
	private logger = getLogger("basic-filler")

	constructor(privateKey: HexString, configService: FillerConfigService) {
		this.privateKey = privateKey
		this.configService = configService
		this.clientManager = new ChainClientManager(configService, privateKey)
		this.contractService = new ContractInteractionService(this.clientManager, privateKey, configService)
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
				this.logger.debug({ deadline: Number(deadline), currentBlock: Number(currentBlock) }, "Order expired")
				return false
			}

			const isAlreadyFilled = await this.contractService.checkIfOrderFilled(order)
			if (isAlreadyFilled) {
				this.logger.debug("Order is already filled")
				return false
			}

			// Validate order inputs and outputs
			const isValidOrder = await this.validateOrderInputsOutputs(order)
			if (!isValidOrder) {
				this.logger.debug("Order inputs and outputs validation failed")
				return false
			}

			const hasEnoughTokens = await this.contractService.checkTokenBalances(order.outputs, order.destChain)
			if (!hasEnoughTokens) {
				this.logger.debug("Insufficient token balances for order")
				return false
			}

			return true
		} catch (error) {
			this.logger.error({ err: error }, "Error in canFill")
			return false
		}
	}

	/**
	 * Calculates the USD value of the order's inputs, outputs, fees and compares
	 * what will the filler receive and what will the filler pay
	 * @param order The order to calculate the USD value for
	 * @returns The profit in USD (Number)
	 */
	async calculateProfitability(order: Order): Promise<number> {
		try {
			const { fillGas, relayerFeeInFeeToken } = await this.contractService.estimateGasFillPost(order)

			const protocolFeeInFeeToken = await this.contractService.quote(order)

			const { decimals: destFeeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(
				order.destChain,
			)
			const { decimals: sourceFeeTokenDecimals } = await this.contractService.getFeeTokenWithDecimals(
				order.sourceChain,
			)

			const totalGasEstimateInFeeToken =
				(await this.contractService.convertGasToFeeToken(fillGas, order.destChain, destFeeTokenDecimals)) +
				protocolFeeInFeeToken +
				relayerFeeInFeeToken

			const orderFeeInDestFeeToken = adjustFeeDecimals(order.fees, sourceFeeTokenDecimals, destFeeTokenDecimals)

			const profit =
				orderFeeInDestFeeToken > totalGasEstimateInFeeToken
					? orderFeeInDestFeeToken - totalGasEstimateInFeeToken
					: BigInt(0)

			this.logger.info(
				{
					orderFeesUSD: formatUnits(orderFeeInDestFeeToken, destFeeTokenDecimals),
					totalGasEstimateUSD: formatUnits(totalGasEstimateInFeeToken, destFeeTokenDecimals),
					profitable: profit > 0,
					profitUSD: formatUnits(profit, destFeeTokenDecimals),
				},
				"Profitability evaluation",
			)
			return parseFloat(formatUnits(profit, destFeeTokenDecimals))
		} catch (error) {
			this.logger.error({ err: error }, "Error calculating profitability")
			return 0
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

			const { relayerFeeInFeeToken, relayerFeeInNativeToken, fillGas } =
				await this.contractService.estimateGasFillPost(order)

			const fillOptions: FillOptions = {
				relayerFee: relayerFeeInFeeToken,
			}

			const ethValue = this.contractService.calculateRequiredEthValue(order.outputs)

			await this.contractService.approveTokensIfNeeded(order)

			const tx = await walletClient.writeContract({
				abi: INTENT_GATEWAY_ABI,
				address: this.configService.getIntentGatewayAddress(order.destChain),
				functionName: "fillOrder",
				args: [this.contractService.transformOrderForContract(order), fillOptions as any],
				account: privateKeyToAccount(this.privateKey),
				value: relayerFeeInFeeToken !== 0n ? ethValue + relayerFeeInNativeToken : ethValue,
				chain: walletClient.chain,
				gas: fillGas + (fillGas * 2500n) / 10000n,
			})

			const endTime = Date.now()
			const processingTimeMs = endTime - startTime

			const receipt = await destClient.waitForTransactionReceipt({ hash: tx })

			if (receipt.status !== "success") {
				this.logger.error({ txHash: receipt.transactionHash, status: receipt.status }, "Could not fill order")
				return {
					success: false,
					txHash: tx,
				}
			}

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
			this.logger.error({ err: error }, "Error executing order")

			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	/**
	 * Validates that order inputs and outputs are valid for filling
	 * @param order The order to validate
	 * @returns True if the order inputs and outputs are valid
	 */
	async validateOrderInputsOutputs(order: Order): Promise<boolean> {
		try {
			if (order.inputs.length !== order.outputs.length) {
				this.logger.debug(
					{ inputs: order.inputs.length, outputs: order.outputs.length },
					"Order length mismatch",
				)
				return false
			}

			const getTokenType = (tokenAddress: string, chain: string): string | null => {
				tokenAddress = bytes32ToBytes20(tokenAddress).toLowerCase()
				const assets = {
					DAI: this.configService.getDaiAsset(chain).toLowerCase(),
					USDT: this.configService.getUsdtAsset(chain).toLowerCase(),
					USDC: this.configService.getUsdcAsset(chain).toLowerCase(),
				}
				const result =
					Object.keys(assets).find((type) => assets[type as keyof typeof assets] === tokenAddress) || null

				return result
			}

			for (let i = 0; i < order.inputs.length; i++) {
				const input = order.inputs[i]
				const output = order.outputs[i]

				const inputType = getTokenType(input.token, order.sourceChain)
				const outputType = getTokenType(output.token, order.destChain)

				if (!inputType) {
					this.logger.debug({ index: i, token: input.token }, "Unsupported input token")
					return false
				}

				if (!outputType) {
					this.logger.debug({ index: i, token: output.token }, "Unsupported output token")
					return false
				}

				if (inputType !== outputType) {
					this.logger.debug({ index: i, inputType, outputType }, "Token mismatch")
					return false
				}

				const [inputDecimals, outputDecimals] = await Promise.all([
					this.contractService.getTokenDecimals(input.token, order.sourceChain),
					this.contractService.getTokenDecimals(output.token, order.destChain),
				])

				if (!compareDecimalValues(input.amount, inputDecimals, output.amount, outputDecimals)) {
					this.logger.debug(
						{
							index: i,
							inputAmount: input.amount.toString(),
							inputDecimals,
							outputAmount: output.amount.toString(),
							outputDecimals,
						},
						"Amount mismatch",
					)
					return false
				}
			}

			return true
		} catch (error) {
			this.logger.error({ err: error }, "Order validation failed")
			return false
		}
	}
}
