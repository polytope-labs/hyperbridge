import {
	bytes32ToBytes20,
	bytes20ToBytes32,
	constructRedeemEscrowRequestBody,
	getStorageSlot,
	ADDRESS_ZERO,
	MOCK_ADDRESS,
	ERC20Method,
	adjustFeeDecimals,
	fetchPrice,
	parseStateMachineId,
	orderCommitment,
	sleep,
	getRequestCommitment,
	waitForChallengePeriod,
	retryPromise,
	maxBigInt,
	getGasPriceFromEtherscan,
	USE_ETHERSCAN_CHAINS,
} from "@/utils"
import { formatUnits, hexToString, maxUint256, pad, parseUnits, toHex } from "viem"
import {
	DispatchPost,
	IGetRequest,
	IHyperbridgeConfig,
	RequestStatus,
	type FillOptions,
	type HexString,
	type IPostRequest,
	type Order,
} from "@/types"
import IntentGatewayABI from "@/abis/IntentGateway"
import type { EvmChain } from "@/chains/evm"
import { Decimal } from "decimal.js"
import { getChain, IGetRequestMessage, IProof, requestCommitmentKey, SubstrateChain } from "@/chain"
import { IndexerClient } from "@/client"
import { Swap } from "@/utils/swap"

/**
 * IntentGateway handles cross-chain intent operations between EVM chains.
 * It provides functionality for estimating fill orders, finding optimal swap protocols,
 * and checking order statuses across different chains.
 */
export class IntentGateway {
	public readonly swap: Swap
	/**
	 * Creates a new IntentGateway instance for cross-chain operations.
	 * @param source - The source EVM chain
	 * @param dest - The destination EVM chain
	 */
	constructor(
		public readonly source: EvmChain,
		public readonly dest: EvmChain,
	) {
		this.swap = new Swap()
	}

	/**
	 * Estimates the total cost required to fill an order, including gas fees, relayer fees,
	 * protocol fees, and swap operations.
	 *
	 * @param order - The order to estimate fill costs for
	 * @returns An object containing the estimated cost in both fee token and native token, plus the post request calldata
	 */
	async estimateFillOrder(
		order: Order,
	): Promise<{ feeTokenAmount: bigint; nativeTokenAmount: bigint; postRequestCalldata: HexString }> {
		const postRequest: IPostRequest = {
			source: order.destChain,
			dest: order.sourceChain,
			body: constructRedeemEscrowRequestBody(order, MOCK_ADDRESS),
			timeoutTimestamp: 0n,
			nonce: await this.source.getHostNonce(),
			from: this.source.configService.getIntentGatewayAddress(order.destChain),
			to: this.source.configService.getIntentGatewayAddress(order.sourceChain),
		}

		const { decimals: sourceChainFeeTokenDecimals } = await this.source.getFeeTokenWithDecimals()

		const { address: destChainFeeTokenAddress, decimals: destChainFeeTokenDecimals } =
			await this.dest.getFeeTokenWithDecimals()

		const { gas: postGasEstimate, postRequestCalldata } = await this.source.estimateGas(postRequest)

		const postGasEstimateInSourceFeeToken = await this.convertGasToFeeToken(
			postGasEstimate,
			"source",
			order.sourceChain,
		)

		const minRelayerFee = 5n * 10n ** BigInt(sourceChainFeeTokenDecimals - 2)
		const postGasWithIncentive = postGasEstimateInSourceFeeToken + (postGasEstimateInSourceFeeToken * 1n) / 100n
		const relayerFeeInSourceFeeToken = maxBigInt(postGasWithIncentive, minRelayerFee)

		const relayerFeeInDestFeeToken = adjustFeeDecimals(
			relayerFeeInSourceFeeToken,
			sourceChainFeeTokenDecimals,
			destChainFeeTokenDecimals,
		)

		const fillOptions: FillOptions = {
			relayerFee: relayerFeeInDestFeeToken,
		}

		const totalEthValue = order.outputs
			.filter((output) => bytes32ToBytes20(output.token) === ADDRESS_ZERO)
			.reduce((sum, output) => sum + output.amount, 0n)

		const intentGatewayAddress = this.source.configService.getIntentGatewayAddress(order.destChain)
		const testValue = toHex(maxUint256 / 2n)

		const orderOverrides = await Promise.all(
			order.outputs.map(async (output) => {
				const tokenAddress = bytes32ToBytes20(output.token)

				if (tokenAddress === ADDRESS_ZERO) {
					return null
				}

				try {
					const stateDiffs = []

					const balanceData = ERC20Method.BALANCE_OF + bytes20ToBytes32(MOCK_ADDRESS).slice(2)
					const balanceSlot = await getStorageSlot(this.dest.client, tokenAddress, balanceData as HexString)
					stateDiffs.push({ slot: balanceSlot as HexString, value: testValue })

					try {
						const allowanceData =
							ERC20Method.ALLOWANCE +
							bytes20ToBytes32(MOCK_ADDRESS).slice(2) +
							bytes20ToBytes32(intentGatewayAddress).slice(2)
						const allowanceSlot = await getStorageSlot(
							this.dest.client,
							tokenAddress,
							allowanceData as HexString,
						)
						stateDiffs.push({ slot: allowanceSlot as HexString, value: testValue })
					} catch (e) {
						console.warn(`Could not find allowance slot for token ${tokenAddress}:`, e)
					}

					return { address: tokenAddress, stateDiff: stateDiffs }
				} catch (e) {
					console.warn(`Could not find balance slot for token ${tokenAddress}:`, e)
					return null
				}
			}),
		).then((results) => results.filter(Boolean))

		const stateOverrides = [
			// Mock address with ETH balance so that any chain estimation runs
			// even when the address doesn't hold any native token in that chain
			{
				address: MOCK_ADDRESS,
				balance: maxUint256,
			},
			...orderOverrides.map((override) => ({
				address: override!.address,
				stateDiff: override!.stateDiff,
			})),
		]

		let destChainFillGas = 0n
		try {
			let protocolFeeInNativeToken = await this.quoteNative(postRequest, relayerFeeInDestFeeToken).catch(() =>
				this.dest.quoteNative(postRequest, relayerFeeInDestFeeToken).catch(() => 0n),
			)
			protocolFeeInNativeToken = protocolFeeInNativeToken + (protocolFeeInNativeToken * 50n) / 10000n

			destChainFillGas = await this.dest.client.estimateContractGas({
				abi: IntentGatewayABI.ABI,
				address: intentGatewayAddress,
				functionName: "fillOrder",
				args: [transformOrderForContract(order), fillOptions as any],
				account: MOCK_ADDRESS,
				value: totalEthValue + protocolFeeInNativeToken,
				stateOverride: stateOverrides as any,
			})
		} catch {
			console.warn(
				`Could not estimate gas for fill order with native token as fees for chain ${order.destChain}, now trying with fee token as fees`,
			)

			const destFeeTokenBalanceData = ERC20Method.BALANCE_OF + bytes20ToBytes32(MOCK_ADDRESS).slice(2)
			const destFeeTokenBalanceSlot = await getStorageSlot(
				this.dest.client,
				destChainFeeTokenAddress,
				destFeeTokenBalanceData as HexString,
			)
			const destFeeTokenAllowanceData =
				ERC20Method.ALLOWANCE +
				bytes20ToBytes32(MOCK_ADDRESS).slice(2) +
				bytes20ToBytes32(intentGatewayAddress).slice(2)
			const destFeeTokenAllowanceSlot = await getStorageSlot(
				this.dest.client,
				destChainFeeTokenAddress,
				destFeeTokenAllowanceData as HexString,
			)
			const feeTokenStateDiffs = [
				{ slot: destFeeTokenBalanceSlot, value: testValue },
				{ slot: destFeeTokenAllowanceSlot, value: testValue },
			]

			stateOverrides.push({
				address: destChainFeeTokenAddress,
				stateDiff: feeTokenStateDiffs as any,
			})

			destChainFillGas = await this.dest.client.estimateContractGas({
				abi: IntentGatewayABI.ABI,
				address: intentGatewayAddress,
				functionName: "fillOrder",
				args: [transformOrderForContract(order), fillOptions as any],
				account: MOCK_ADDRESS,
				value: totalEthValue,
				stateOverride: stateOverrides as any,
			})
		}

		const fillGasInDestFeeToken = await this.convertGasToFeeToken(destChainFillGas, "dest", order.destChain)
		const fillGasInSourceFeeToken = adjustFeeDecimals(
			fillGasInDestFeeToken,
			destChainFeeTokenDecimals,
			sourceChainFeeTokenDecimals,
		)

		const protocolFeeInSourceFeeToken = adjustFeeDecimals(
			await this.dest.quote(postRequest),
			destChainFeeTokenDecimals,
			sourceChainFeeTokenDecimals,
		)

		let totalEstimateInSourceFeeToken =
			fillGasInSourceFeeToken + protocolFeeInSourceFeeToken + relayerFeeInSourceFeeToken

		let totalNativeTokenAmount = await this.convertFeeTokenToNative(
			totalEstimateInSourceFeeToken,
			"source",
			order.sourceChain,
		)

		if ([order.destChain, order.sourceChain].includes("EVM-1")) {
			totalEstimateInSourceFeeToken =
				totalEstimateInSourceFeeToken + (totalEstimateInSourceFeeToken * 3000n) / 10000n
			totalNativeTokenAmount = totalNativeTokenAmount + (totalNativeTokenAmount * 3200n) / 10000n
		} else {
			totalEstimateInSourceFeeToken =
				totalEstimateInSourceFeeToken + (totalEstimateInSourceFeeToken * 250n) / 10000n
			totalNativeTokenAmount = totalNativeTokenAmount + (totalNativeTokenAmount * 350n) / 10000n
		}
		return {
			feeTokenAmount: totalEstimateInSourceFeeToken,
			nativeTokenAmount: totalNativeTokenAmount,
			postRequestCalldata,
		}
	}

	/**
	 * Converts fee token amounts back to the equivalent amount in native token.
	 * Uses USD pricing to convert between fee token amounts and native token costs.
	 *
	 * @param feeTokenAmount - The amount in fee token (DAI)
	 * @param getQuoteIn - Whether to use "source" or "dest" chain for the conversion
	 * @param evmChainID - The EVM chain ID in format "EVM-{id}"
	 * @returns The fee token amount converted to native token amount
	 * @private
	 */
	private async convertFeeTokenToNative(
		feeTokenAmount: bigint,
		getQuoteIn: "source" | "dest",
		evmChainID: string,
	): Promise<bigint> {
		const client = this[getQuoteIn].client
		const wethAsset = this[getQuoteIn].configService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = await this[getQuoteIn].getFeeTokenWithDecimals()

		try {
			const { amountOut } = await this.swap.findBestProtocolWithAmountIn(
				this[getQuoteIn].client,
				feeToken.address,
				wethAsset,
				feeTokenAmount,
				evmChainID,
				{ selectedProtocol: "v2" },
			)

			if (amountOut === 0n) {
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = Number.parseInt(evmChainID.split("-")[1])
			const feeTokenAmountDecimal = new Decimal(formatUnits(feeTokenAmount, feeToken.decimals))
			const nativeTokenPriceUsd = new Decimal(await fetchPrice(nativeCurrency?.symbol!, chainId))
			const totalCostInNativeToken = feeTokenAmountDecimal.dividedBy(nativeTokenPriceUsd)
			return parseUnits(totalCostInNativeToken.toFixed(nativeCurrency?.decimals!), nativeCurrency?.decimals!)
		}
	}

	/**
	 * Converts gas costs to the equivalent amount in the fee token (DAI).
	 * Uses USD pricing to convert between native token gas costs and fee token amounts.
	 *
	 * @param gasEstimate - The estimated gas units
	 * @param gasEstimateIn - Whether to use "source" or "dest" chain for the conversion
	 * @param evmChainID - The EVM chain ID in format "EVM-{id}"
	 * @returns The gas cost converted to fee token amount
	 * @private
	 */
	private async convertGasToFeeToken(
		gasEstimate: bigint,
		gasEstimateIn: "source" | "dest",
		evmChainID: string,
	): Promise<bigint> {
		const client = this[gasEstimateIn].client
		const useEtherscan = USE_ETHERSCAN_CHAINS.has(evmChainID)
		const etherscanApiKey = useEtherscan ? this[gasEstimateIn].configService.getEtherscanApiKey() : undefined
		const gasPrice =
			useEtherscan && etherscanApiKey
				? await retryPromise(() => getGasPriceFromEtherscan(evmChainID, etherscanApiKey), {
						maxRetries: 3,
						backoffMs: 250,
					}).catch(async () => {
						console.warn({ evmChainID }, "Error getting gas price from etherscan, using client's gas price")
						return await client.getGasPrice()
					})
				: await client.getGasPrice()
		const gasCostInWei = gasEstimate * gasPrice
		const wethAddr = this[gasEstimateIn].configService.getWrappedNativeAssetWithDecimals(evmChainID).asset
		const feeToken = await this[gasEstimateIn].getFeeTokenWithDecimals()

		try {
			const { amountOut } = await this.swap.findBestProtocolWithAmountIn(
				this[gasEstimateIn].client,
				wethAddr,
				feeToken.address,
				gasCostInWei,
				evmChainID,
				{ selectedProtocol: "v2" },
			)
			if (amountOut === 0n) {
				console.log("Amount out not found")
				throw new Error()
			}
			return amountOut
		} catch {
			// Testnet block
			const nativeCurrency = client.chain?.nativeCurrency
			const chainId = Number.parseInt(evmChainID.split("-")[1])
			const gasCostInToken = new Decimal(formatUnits(gasCostInWei, nativeCurrency?.decimals!))
			const tokenPriceUsd = await fetchPrice(nativeCurrency?.symbol!, chainId)
			const gasCostUsd = gasCostInToken.times(tokenPriceUsd)
			const feeTokenPriceUsd = new Decimal(1) // stable coin
			const gasCostInFeeToken = gasCostUsd.dividedBy(feeTokenPriceUsd)
			return parseUnits(gasCostInFeeToken.toFixed(feeToken.decimals), feeToken.decimals)
		}
	}

	/**
	 * Gets a quote for the native token cost of dispatching a post request.
	 *
	 * @param postRequest - The post request to quote
	 * @param fee - The fee amount in fee token
	 * @returns The native token amount required
	 */
	async quoteNative(postRequest: IPostRequest, fee: bigint): Promise<bigint> {
		const dispatchPost: DispatchPost = {
			dest: toHex(postRequest.dest),
			to: postRequest.to,
			body: postRequest.body,
			timeout: postRequest.timeoutTimestamp,
			fee: fee,
			payer: postRequest.from,
		}

		const quoteNative = await this.dest.client.readContract({
			address: this.dest.configService.getIntentGatewayAddress(postRequest.dest),
			abi: IntentGatewayABI.ABI,
			functionName: "quoteNative",
			args: [dispatchPost] as any,
		})

		return quoteNative
	}

	/**
	 * Checks if an order has been filled by verifying the commitment status on-chain.
	 * Reads the storage slot corresponding to the order's commitment hash.
	 *
	 * @param order - The order to check
	 * @returns True if the order has been filled, false otherwise
	 */
	async isOrderFilled(order: Order): Promise<boolean> {
		const intentGatewayAddress = this.source.configService.getIntentGatewayAddress(order.destChain)

		const filledSlot = await this.dest.client.readContract({
			abi: IntentGatewayABI.ABI,
			address: intentGatewayAddress,
			functionName: "calculateCommitmentSlotHash",
			args: [order.id as HexString],
		})

		const filledStatus = await this.dest.client.getStorageAt({
			address: intentGatewayAddress,
			slot: filledSlot,
		})
		return filledStatus !== "0x0000000000000000000000000000000000000000000000000000000000000000"
	}

	async submitAndConfirmReceipt(hyperbridge: SubstrateChain, commitment: HexString, message: IGetRequestMessage) {
		let storageValue = await hyperbridge.queryRequestReceipt(commitment)

		if (!storageValue) {
			console.log("No receipt found. Attempting to submit...")
			try {
				await hyperbridge.submitUnsigned(message)
			} catch {
				console.warn("Submission failed. Awaiting network confirmation...")
			}

			console.log("Waiting for network state update...")
			await sleep(30000)

			storageValue = await retryPromise(
				async () => {
					const value = await hyperbridge.queryRequestReceipt(commitment)
					if (!value) throw new Error("Receipt not found")
					return value
				},
				{ maxRetries: 10, backoffMs: 5000, logMessage: "Checking for receipt" },
			)
		}

		console.log("Hyperbridge Receipt confirmed.")
	}

	async *cancelOrder(
		order: Order,
		hyperbridgeConfig: IHyperbridgeConfig,
		indexerClient: IndexerClient,
		storedData?: StoredCancellationData,
	) {
		const hyperbridge = (await getChain({ ...hyperbridgeConfig, hasher: "Keccak" })) as SubstrateChain
		const sourceStateMachine = hexToString(order.sourceChain as HexString)
		const destStateMachine = hexToString(order.destChain as HexString)

		const sourceConsensusStateId = this.source.configService.getConsensusStateId(sourceStateMachine)
		const destConsensusStateId = this.dest.configService.getConsensusStateId(destStateMachine)

		let destIProof: IProof

		if (storedData?.destIProof) {
			destIProof = storedData.destIProof
			yield { status: "DESTINATION_FINALIZED", data: { proof: destIProof } }
		} else {
			let latestHeight = 0n
			let lastFailedHeight: bigint | null = null
			let proofHex: HexString | null = null

			while (!proofHex) {
				const height = await retryPromise(
					() =>
						indexerClient.queryLatestStateMachineHeight({
							statemachineId: destStateMachine,
							chain: hyperbridgeConfig.stateMachineId,
						}),
					{ maxRetries: 5, backoffMs: 500, logMessage: "Failed to fetch latest state machine height" },
				)

				if (!height) {
					throw new Error("No state machine updates found for destination chain")
				}

				latestHeight = height

				const shouldFetchProof =
					lastFailedHeight === null ? latestHeight > order.deadline : latestHeight > lastFailedHeight

				if (!shouldFetchProof) {
					yield {
						status: "AWAITING_DESTINATION_FINALIZED",
						data: {
							currentHeight: latestHeight,
							deadline: order.deadline,
							...(lastFailedHeight && { lastFailedHeight }),
						},
					}
					await sleep(10000)
					continue
				}

				try {
					const intentGatewayAddress = this.dest.configService.getIntentGatewayAddress(destStateMachine)
					const orderId = orderCommitment(order)
					const slotHash = await this.dest.client.readContract({
						abi: IntentGatewayABI.ABI,
						address: intentGatewayAddress,
						functionName: "calculateCommitmentSlotHash",
						args: [orderId],
					})
					proofHex = await this.dest.queryStateProof(latestHeight, [slotHash], intentGatewayAddress)
				} catch (error) {
					lastFailedHeight = latestHeight
					yield {
						status: "PROOF_FETCH_FAILED",
						data: {
							failedHeight: latestHeight,
							error: error instanceof Error ? error.message : String(error),
							deadline: order.deadline,
						},
					}
					await sleep(10000)
				}
			}

			destIProof = {
				consensusStateId: destConsensusStateId,
				height: latestHeight,
				proof: proofHex,
				stateMachine: destStateMachine,
			}

			yield { status: "DESTINATION_FINALIZED", data: { proof: destIProof } }
		}

		const getRequest = storedData?.getRequest ?? ((yield { status: "AWAITING_GET_REQUEST" }) as IGetRequest)
		if (!getRequest) throw new Error("[Cancel Order]: Get Request not provided")

		const commitment = getRequestCommitment({ ...getRequest, keys: [...getRequest.keys] })

		const sourceStatusStream = indexerClient.getRequestStatusStream(commitment)
		for await (const statusUpdate of sourceStatusStream) {
			yield statusUpdate

			if (statusUpdate.status !== RequestStatus.SOURCE_FINALIZED) {
				continue
			}

			let sourceHeight = BigInt(statusUpdate.metadata.blockNumber)
			let proof: HexString | undefined
			// Check if request was delivered while waiting for proof
			const checkIfAlreadyDelivered = async () => {
				const currentStatus = await indexerClient.queryGetRequestWithStatus(commitment)
				return (
					currentStatus?.statuses.some((status) => status.status === RequestStatus.HYPERBRIDGE_DELIVERED) ??
					false
				)
			}

			const { slot1, slot2 } = requestCommitmentKey(commitment)

			while (true) {
				try {
					proof = await this.source.queryStateProof(sourceHeight, [slot1, slot2])
					break
				} catch {
					const failedHeight = sourceHeight
					while (sourceHeight <= failedHeight) {
						if (await checkIfAlreadyDelivered()) {
							break
						}

						const nextHeight = await retryPromise(
							() =>
								indexerClient.queryLatestStateMachineHeight({
									statemachineId: sourceStateMachine,
									chain: hyperbridgeConfig.stateMachineId,
								}),
							{
								maxRetries: 5,
								backoffMs: 5000,
								logMessage: "Failed to fetch latest state machine height (post-source-proof failure)",
							},
						)

						if (!nextHeight) {
							throw new Error(
								`No state machine updates found for ${sourceStateMachine} on chain ${hyperbridgeConfig.stateMachineId}`,
							)
						}

						if (nextHeight <= failedHeight) {
							await sleep(10000)
							continue
						}

						sourceHeight = nextHeight
					}

					if (await checkIfAlreadyDelivered()) {
						break
					}
				}
			}

			if (proof) {
				const sourceIProof: IProof = {
					height: sourceHeight,
					stateMachine: sourceStateMachine,
					consensusStateId: sourceConsensusStateId,
					proof,
				}

				yield { status: "SOURCE_PROOF_RECEIVED", data: sourceIProof }

				const getRequestMessage: IGetRequestMessage = {
					kind: "GetRequest",
					requests: [getRequest],
					source: sourceIProof,
					response: destIProof,
					signer: pad("0x"),
				}

				await waitForChallengePeriod(hyperbridge, {
					height: sourceHeight,
					id: {
						stateId: parseStateMachineId(sourceStateMachine).stateId,
						consensusStateId: sourceConsensusStateId,
					},
				})

				await this.submitAndConfirmReceipt(hyperbridge, commitment, getRequestMessage)
			}
		}
	}
}

/**
 * Transforms an Order object into the format expected by the smart contract.
 * Converts chain IDs to hex format and restructures input/output arrays.
 *
 * @param order - The order to transform
 * @returns The order in contract-compatible format
 */
function transformOrderForContract(order: Order) {
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

interface StoredCancellationData {
	destIProof?: IProof
	getRequest?: IGetRequest
	sourceIProof?: IProof
}
