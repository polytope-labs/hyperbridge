import { Decimal } from "decimal.js"
import {
	concatHex,
	encodeAbiParameters,
	formatUnits,
	hexToString,
	keccak256,
	maxUint256,
	pad,
	parseEventLogs,
	parseUnits,
	toHex,
} from "viem"
import EVM_HOST from "@/abis/evmHost"
import IntentGatewayABI from "@/abis/IntentGateway"
import { type IGetRequestMessage, type IProof, requestCommitmentKey, type SubstrateChain } from "@/chain"
import type { EvmChain } from "@/chains/evm"
import type { IndexerClient } from "@/client"
import { createCancellationStorage, STORAGE_KEYS } from "@/storage"
import {
	type DispatchPost,
	type FillOptions,
	type HexString,
	type IGetRequest,
	type IPostRequest,
	type Order,
	RequestStatus,
	type RequestStatusWithMetadata,
} from "@/types"
import {
	ADDRESS_ZERO,
	adjustFeeDecimals,
	bytes20ToBytes32,
	bytes32ToBytes20,
	constructRedeemEscrowRequestBody,
	ERC20Method,
	fetchPrice,
	getGasPriceFromEtherscan,
	getRequestCommitment,
	getStorageSlot,
	MOCK_ADDRESS,
	maxBigInt,
	orderCommitment,
	parseStateMachineId,
	retryPromise,
	sleep,
	USE_ETHERSCAN_CHAINS,
	waitForChallengePeriod,
} from "@/utils"
import { Swap } from "@/utils/swap"

/**
 * IntentGateway handles cross-chain intent operations between EVM chains.
 * It provides functionality for estimating fill orders, finding optimal swap protocols,
 * and checking order statuses across different chains.
 */
export class IntentGateway {
	public readonly swap: Swap
	private readonly storage = createCancellationStorage()
	/**
	 * Optional custom IntentGateway address for the destination chain.
	 * If set, this address will be used when fetching destination proofs in `cancelOrder`.
	 * If not set, uses the default address from the chain configuration.
	 * This allows using different IntentGateway contract versions (e.g., old vs new contracts).
	 */
	public destIntentGatewayAddress?: HexString

	/**
	 * Creates a new IntentGateway instance for cross-chain operations.
	 * @param source - The source EVM chain
	 * @param dest - The destination EVM chain
	 * @param destIntentGatewayAddress - Optional custom IntentGateway address for the destination chain.
	 *   If provided, this address will be used when fetching destination proofs in `cancelOrder`.
	 *   If not provided, uses the default address from the chain configuration.
	 */
	constructor(
		public readonly source: EvmChain,
		public readonly dest: EvmChain,
		destIntentGatewayAddress?: HexString,
	) {
		this.swap = new Swap()
		this.destIntentGatewayAddress = destIntentGatewayAddress
	}

	/**
	 * Estimates the total cost required to fill an order, including gas fees, relayer fees,
	 * protocol fees, and swap operations.
	 *
	 * @param order - The order to estimate fill costs for
	 * @returns An object containing the estimated cost in both fee token and native token, plus the post request calldata
	 */
	async estimateFillOrder(order: Order): Promise<{
		order: Order
		feeTokenAmount: bigint
		nativeTokenAmount: bigint
		postRequestCalldata: HexString
	}> {
		// Order with commitment and stringified chains
		const orderWithCommitment = transformOrder(order)

		const postRequest: IPostRequest = {
			source: orderWithCommitment.destChain,
			dest: orderWithCommitment.sourceChain,
			body: constructRedeemEscrowRequestBody(orderWithCommitment, MOCK_ADDRESS),
			timeoutTimestamp: 0n,
			nonce: await this.source.getHostNonce(),
			from: this.source.configService.getIntentGatewayAddress(orderWithCommitment.destChain),
			to: this.source.configService.getIntentGatewayAddress(orderWithCommitment.sourceChain),
		}

		const { decimals: sourceChainFeeTokenDecimals } = await this.source.getFeeTokenWithDecimals()

		const { address: destChainFeeTokenAddress, decimals: destChainFeeTokenDecimals } =
			await this.dest.getFeeTokenWithDecimals()

		const { gas: postGasEstimate, postRequestCalldata } = await this.source.estimateGas(postRequest)

		const postGasEstimateInSourceFeeToken = await this.convertGasToFeeToken(
			postGasEstimate,
			"source",
			orderWithCommitment.sourceChain,
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

		const totalEthValue = orderWithCommitment.outputs
			.filter((output) => bytes32ToBytes20(output.token) === ADDRESS_ZERO)
			.reduce((sum, output) => sum + output.amount, 0n)

		const intentGatewayAddress = this.source.configService.getIntentGatewayAddress(orderWithCommitment.destChain)
		const testValue = toHex(maxUint256 / 2n)

		const orderOverrides = await Promise.all(
			orderWithCommitment.outputs.map(async (output) => {
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
						stateDiffs.push({
							slot: allowanceSlot as HexString,
							value: testValue,
						})
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
				args: [order as any, fillOptions as any],
				account: MOCK_ADDRESS,
				value: totalEthValue + protocolFeeInNativeToken,
				stateOverride: stateOverrides as any,
			})
		} catch {
			console.warn(
				`Could not estimate gas for fill order with native token as fees for chain ${orderWithCommitment.destChain}, now trying with fee token as fees`,
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
				args: [order as any, fillOptions as any],
				account: MOCK_ADDRESS,
				value: totalEthValue,
				stateOverride: stateOverrides as any,
			})
		}

		const fillGasInDestFeeToken = await this.convertGasToFeeToken(
			destChainFillGas,
			"dest",
			orderWithCommitment.destChain,
		)
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
			orderWithCommitment.sourceChain,
		)

		if ([orderWithCommitment.destChain, orderWithCommitment.sourceChain].includes("EVM-1")) {
			totalEstimateInSourceFeeToken =
				totalEstimateInSourceFeeToken + (totalEstimateInSourceFeeToken * 3000n) / 10000n
			totalNativeTokenAmount = totalNativeTokenAmount + (totalNativeTokenAmount * 3200n) / 10000n
		} else {
			totalEstimateInSourceFeeToken =
				totalEstimateInSourceFeeToken + (totalEstimateInSourceFeeToken * 250n) / 10000n
			totalNativeTokenAmount = totalNativeTokenAmount + (totalNativeTokenAmount * 350n) / 10000n
		}
		return {
			order: {
				...order,
				fees: totalEstimateInSourceFeeToken,
			},
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
		order = transformOrder(order)
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

	/**
	 * Checks if an order has been refunded by verifying the escrowed token amounts on-chain.
	 * Reads the storage slots for the `_orders` mapping on the source chain (where the escrow is held).
	 * An order is considered refunded when all input token amounts in the `_orders` mapping are 0.
	 *
	 * @param order - The order to check
	 * @returns True if the order has been refunded (all token amounts are 0), false otherwise
	 */
	async isOrderRefunded(order: Order): Promise<boolean> {
		order = transformOrder(order)
		const intentGatewayAddress =
			this.destIntentGatewayAddress ?? this.source.configService.getIntentGatewayAddress(order.sourceChain)

		const commitment = order.id as HexString
		const ORDERS_MAPPING_SLOT = 4n

		const firstLevelSlot = keccak256(
			encodeAbiParameters([{ type: "bytes32" }, { type: "uint256" }], [commitment, ORDERS_MAPPING_SLOT]),
		)

		for (const input of order.inputs) {
			const tokenAddress = bytes32ToBytes20(input.token)

			const storageSlot = keccak256(
				encodeAbiParameters(
					[{ type: "address" }, { type: "bytes32" }],
					[tokenAddress as `0x${string}`, firstLevelSlot as `0x${string}`],
				),
			)

			const escrowedAmount = await this.source.client.getStorageAt({
				address: intentGatewayAddress,
				slot: storageSlot,
			})

			if (escrowedAmount !== "0x0000000000000000000000000000000000000000000000000000000000000000") {
				return false
			}
		}

		return true
	}

	private async submitAndConfirmReceipt(
		hyperbridge: SubstrateChain,
		commitment: HexString,
		message: IGetRequestMessage,
	) {
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

	/**
	 * Returns the native token amount required to dispatch a cancellation GET request for the given order.
	 * Internally constructs the IGetRequest and calls quoteNative.
	 */
	async quoteCancelNative(order: Order): Promise<bigint> {
		const orderWithCommitment = transformOrder(order)

		const height = (orderWithCommitment.deadline as bigint) + 1n

		const destIntentGateway = this.dest.configService.getIntentGatewayAddress(orderWithCommitment.destChain)
		const slotHash = await this.dest.client.readContract({
			abi: IntentGatewayABI.ABI,
			address: destIntentGateway,
			functionName: "calculateCommitmentSlotHash",
			args: [orderWithCommitment.id as HexString],
		})
		const key = concatHex([destIntentGateway as HexString, slotHash as HexString]) as HexString

		const context = encodeAbiParameters(
			[
				{
					name: "requestBody",
					type: "tuple",
					components: [
						{ name: "commitment", type: "bytes32" },
						{ name: "beneficiary", type: "bytes32" },
						{
							name: "tokens",
							type: "tuple[]",
							components: [
								{ name: "token", type: "bytes32" },
								{ name: "amount", type: "uint256" },
							],
						},
					],
				},
			],
			[
				{
					commitment: orderWithCommitment.id as HexString,
					beneficiary: orderWithCommitment.user as HexString,
					tokens: orderWithCommitment.inputs,
				},
			],
		) as HexString

		const getRequest: IGetRequest = {
			source: orderWithCommitment.sourceChain,
			dest: orderWithCommitment.destChain,
			from: this.source.configService.getIntentGatewayAddress(orderWithCommitment.destChain),
			nonce: await this.source.getHostNonce(),
			height,
			keys: [key],
			timeoutTimestamp: 0n,
			context,
		}

		return await this.source.quoteNative(getRequest, 0n)
	}

	/**
	 * Cancels an order through the cross-chain protocol by generating and submitting proofs.
	 * This is an async generator function that yields status updates throughout the cancellation process.
	 *
	 * The cancellation process involves:
	 * 1. Fetching proof from the destination chain that the order exists
	 * 2. Creating a GET request to retrieve the order state
	 * 3. Waiting for the source chain to finalize the request
	 * 4. Fetching proof from the source chain
	 * 5. Waiting for the challenge period to complete
	 * 6. Submitting the request message to Hyperbridge
	 * 7. Monitoring until Hyperbridge finalizes the cancellation
	 *
	 * @param order - The order to cancel, containing source/dest chains, deadline, and other order details
	 * @param indexerClient - Client for querying the indexer and interacting with Hyperbridge
	 *
	 * @yields {CancelEvent} Status updates during the cancellation process:
	 *   - DESTINATION_FINALIZED: Destination proof has been obtained
	 *   - AWAITING_GET_REQUEST: Waiting for GET request to be provided
	 *   - SOURCE_FINALIZED: Source chain has finalized the request
	 *   - HYPERBRIDGE_DELIVERED: Hyperbridge has delivered the request
	 *   - HYPERBRIDGE_FINALIZED: Cancellation is complete
	 *
	 * @throws {Error} If GET request is not provided when needed
	 *
	 * @example
	 * ```typescript
	 * // Using default IntentGateway address
	 * const intentGateway = new IntentGateway(sourceChain, destChain);
	 * const cancelStream = intentGateway.cancelOrder(order, indexerClient);
	 *
	 * // Using custom IntentGateway address (e.g., for old contract version)
	 * const intentGateway = new IntentGateway(
	 *   sourceChain,
	 *   destChain,
	 *   "0xd54165e45926720b062C192a5bacEC64d5bB08DA"
	 * );
	 * const cancelStream = intentGateway.cancelOrder(order, indexerClient);
	 *
	 * // Or set it after instantiation
	 * const intentGateway = new IntentGateway(sourceChain, destChain);
	 * intentGateway.destIntentGatewayAddress = "0xd54165e45926720b062C192a5bacEC64d5bB08DA";
	 * const cancelStream = intentGateway.cancelOrder(order, indexerClient);
	 *
	 * for await (const event of cancelStream) {
	 *   switch (event.status) {
	 *     case 'SOURCE_FINALIZED':
	 *       console.log('Source finalized at block:', event.data.metadata.blockNumber);
	 *       break;
	 *     case 'HYPERBRIDGE_FINALIZED':
	 *       console.log('Cancellation complete');
	 *       break;
	 *   }
	 * }
	 * ```
	 */
	async *cancelOrder(order: Order, indexerClient: IndexerClient): AsyncGenerator<CancelEvent> {
		const orderId = orderCommitment(order)

		const hyperbridge = indexerClient.hyperbridge as SubstrateChain
		const sourceStateMachine = hexToString(order.sourceChain as HexString)
		const sourceConsensusStateId = this.source.configService.getConsensusStateId(sourceStateMachine)

		let destIProof: IProof | null = await this.storage.getItem(STORAGE_KEYS.destProof(orderId))
		if (!destIProof) {
			destIProof = yield* this.fetchDestinationProof(order, indexerClient)
			await this.storage.setItem(STORAGE_KEYS.destProof(orderId), destIProof)
		} else {
			yield { status: "DESTINATION_FINALIZED", data: { proof: destIProof } }
		}

		let getRequest: IGetRequest | null = await this.storage.getItem(STORAGE_KEYS.getRequest(orderId))
		if (!getRequest) {
			const transactionHash = yield {
				status: "AWAITING_GET_REQUEST",
				data: undefined,
			}
			const receipt = await this.source.client.getTransactionReceipt({
				hash: transactionHash,
			})

			const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })
			const request = events.find((e) => e.eventName === "GetRequestEvent")
			if (!request) throw new Error("GetRequest missing")
			getRequest = request.args as unknown as IGetRequest

			await this.storage.setItem(STORAGE_KEYS.getRequest(orderId), getRequest)
		}

		const commitment = getRequestCommitment({
			...getRequest,
			keys: [...getRequest.keys],
		})
		const sourceStatusStream = indexerClient.getRequestStatusStream(commitment)

		for await (const statusUpdate of sourceStatusStream) {
			if (statusUpdate.status === RequestStatus.SOURCE_FINALIZED) {
				yield {
					status: "SOURCE_FINALIZED",
					data: { metadata: statusUpdate.metadata },
				}

				const sourceHeight = BigInt(statusUpdate.metadata.blockNumber)
				let sourceIProof: IProof | null = await this.storage.getItem(STORAGE_KEYS.sourceProof(orderId))
				if (!sourceIProof) {
					sourceIProof = await fetchSourceProof(
						commitment,
						this.source,
						sourceStateMachine,
						sourceConsensusStateId,
						sourceHeight,
					)
					await this.storage.setItem(STORAGE_KEYS.sourceProof(orderId), sourceIProof)
				}

				await waitForChallengePeriod(hyperbridge, {
					height: sourceIProof.height,
					id: {
						stateId: parseStateMachineId(sourceStateMachine).stateId,
						consensusStateId: sourceConsensusStateId,
					},
				})

				const getRequestMessage: IGetRequestMessage = {
					kind: "GetRequest",
					requests: [getRequest],
					source: sourceIProof,
					response: destIProof,
					signer: pad("0x"),
				}

				await this.submitAndConfirmReceipt(hyperbridge, commitment, getRequestMessage)
				continue
			}

			if (statusUpdate.status === RequestStatus.HYPERBRIDGE_DELIVERED) {
				yield {
					status: "HYPERBRIDGE_DELIVERED",
					data: statusUpdate as RequestStatusWithMetadata,
				}
				continue
			}

			if (statusUpdate.status === RequestStatus.HYPERBRIDGE_FINALIZED) {
				yield {
					status: "HYPERBRIDGE_FINALIZED",
					data: statusUpdate as RequestStatusWithMetadata,
				}
				await this.storage.removeItem(STORAGE_KEYS.destProof(orderId))
				await this.storage.removeItem(STORAGE_KEYS.getRequest(orderId))
				await this.storage.removeItem(STORAGE_KEYS.sourceProof(orderId))
				return
			}
		}
	}

	/**
	 * Fetches proof for the destination chain.
	 * @param order - The order to fetch proof for
	 * @param indexerClient - Client for querying the indexer
	 */
	private async *fetchDestinationProof(
		order: Order,
		indexerClient: IndexerClient,
	): AsyncGenerator<CancelEvent, IProof, void> {
		let latestHeight = 0n
		let lastFailedHeight: bigint | null = null

		while (true) {
			const height = await indexerClient.queryLatestStateMachineHeight({
				statemachineId: this.dest.config.stateMachineId,
				chain: indexerClient.hyperbridge.config.stateMachineId,
			})

			latestHeight = height ?? 0n
			const shouldFetch =
				lastFailedHeight === null ? latestHeight > order.deadline : latestHeight > lastFailedHeight

			if (!shouldFetch) {
				await sleep(10000)
				continue
			}

			try {
				const intentGatewayAddress =
					this.destIntentGatewayAddress ??
					this.dest.configService.getIntentGatewayAddress(this.dest.config.stateMachineId)
				const orderId = orderCommitment(order)
				const slotHash = await this.dest.client.readContract({
					abi: IntentGatewayABI.ABI,
					address: intentGatewayAddress,
					functionName: "calculateCommitmentSlotHash",
					args: [orderId],
				})

				const proofHex = await this.dest.queryStateProof(latestHeight, [slotHash], intentGatewayAddress)

				const proof: IProof = {
					consensusStateId: this.dest.config.consensusStateId,
					height: latestHeight,
					proof: proofHex,
					stateMachine: this.dest.config.stateMachineId,
				}

				yield { status: "DESTINATION_FINALIZED", data: { proof } }
				return proof
			} catch (e) {
				lastFailedHeight = latestHeight
				await sleep(10000)
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
function transformOrder(order: Order) {
	return {
		...order,
		id: orderCommitment(order),
		sourceChain: hexToString(order.sourceChain as HexString),
		destChain: hexToString(order.destChain as HexString),
	}
}

/**
 * Fetches proof for the source chain.
 */
async function fetchSourceProof(
	commitment: HexString,
	source: EvmChain,
	sourceStateMachine: string,
	sourceConsensusStateId: string,
	sourceHeight: bigint,
): Promise<IProof> {
	const { slot1, slot2 } = requestCommitmentKey(commitment)
	const proofHex = await source.queryStateProof(sourceHeight, [slot1, slot2])

	return {
		height: sourceHeight,
		stateMachine: sourceStateMachine,
		consensusStateId: sourceConsensusStateId,
		proof: proofHex,
	}
}

interface CancelEventMap {
	DESTINATION_FINALIZED: { proof: IProof }
	AWAITING_GET_REQUEST: undefined
	SOURCE_FINALIZED: { metadata: { blockNumber: number } }
	HYPERBRIDGE_DELIVERED: RequestStatusWithMetadata
	HYPERBRIDGE_FINALIZED: RequestStatusWithMetadata
	SOURCE_PROOF_RECEIVED: IProof
}

type CancelEvent = {
	[K in keyof CancelEventMap]: { status: K; data: CancelEventMap[K] }
}[keyof CancelEventMap]
