import { FillerStrategy } from "@/strategies/base"
import { Order, ExecutionResult, HexString, FillOptions, adjustFeeDecimals, bytes32ToBytes20 } from "@hyperbridge/sdk"
import { INTENT_GATEWAY_ABI } from "@/config/abis/IntentGateway"
import { privateKeyToAccount, privateKeyToAddress } from "viem/accounts"
import { ChainClientManager, ContractInteractionService } from "@/services"
import { ChainConfigService } from "@hyperbridge/sdk"
import { compareDecimalValues } from "@/utils"

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

			// Validate order inputs and outputs
			const isValidOrder = await this.validateOrderInputsOutputs(order)
			if (!isValidOrder) {
				console.debug(`Order inputs and outputs validation failed`)
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

			console.log({
				orderFees: order.fees.toString(),
				totalGasEstimateInFeeToken: totalGasEstimateInFeeToken.toString(),
				profitable: profit > 0,
				profitInFeeToken: profit.toString(),
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

			const { relayerFeeInFeeToken, relayerFeeInNativeToken } =
				await this.contractService.estimateGasFillPost(order)
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
				value: relayerFeeInFeeToken !== 0n ? ethValue + relayerFeeInNativeToken : ethValue,
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

	/**
	 * Validates that order inputs and outputs are valid for filling
	 * @param order The order to validate
	 * @returns True if the order inputs and outputs are valid
	 */
	async validateOrderInputsOutputs(order: Order): Promise<boolean> {
		try {
			if (order.inputs.length !== order.outputs.length) {
				console.debug(`Order length mismatch: ${order.inputs.length} inputs vs ${order.outputs.length} outputs`)
				return false
			}

			const getTokenType = (tokenAddress: string, chain: string): string | null => {
				tokenAddress = bytes32ToBytes20(tokenAddress)
				const assets = {
					DAI: this.configService.getDaiAsset(chain),
					USDT: this.configService.getUsdtAsset(chain),
					USDC: this.configService.getUsdcAsset(chain),
				}
				return Object.keys(assets).find((type) => assets[type as keyof typeof assets] === tokenAddress) || null
			}

			for (let i = 0; i < order.inputs.length; i++) {
				const input = order.inputs[i]
				const output = order.outputs[i]

				const inputType = getTokenType(input.token, order.sourceChain)
				const outputType = getTokenType(output.token, order.destChain)

				if (!inputType) {
					console.debug(`Unsupported input token at index ${i}: ${input.token}`)
					return false
				}

				if (!outputType) {
					console.debug(`Unsupported output token at index ${i}: ${output.token}`)
					return false
				}

				if (inputType !== outputType) {
					console.debug(`Token mismatch at index ${i}: ${inputType} → ${outputType}`)
					return false
				}

				const [inputDecimals, outputDecimals] = await Promise.all([
					this.contractService.getTokenDecimals(input.token, order.sourceChain),
					this.contractService.getTokenDecimals(output.token, order.destChain),
				])

				if (!compareDecimalValues(input.amount, inputDecimals, output.amount, outputDecimals)) {
					console.debug(
						`Amount mismatch at index ${i}: ${input.amount} (${inputDecimals}d) ≠ ${output.amount} (${outputDecimals}d)`,
					)
					return false
				}
			}

			return true
		} catch (error) {
			console.error(`Order validation failed:`, error)
			return false
		}
	}
}
