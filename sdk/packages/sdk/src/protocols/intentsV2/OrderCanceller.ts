import { encodeFunctionData, concatHex, parseEventLogs, pad } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import EVM_HOST from "@/abis/evmHost"
import {
	hexToString,
	getRequestCommitment,
	postRequestCommitment,
	constructRefundEscrowRequestBody,
	encodeWithdrawalRequest,
	adjustDecimals,
	parseStateMachineId,
	waitForChallengePeriod,
	retryPromise,
	sleep,
} from "@/utils"
import { STORAGE_KEYS } from "@/storage"
import type { OrderV2, HexString, IGetRequest, IPostRequest } from "@/types"
import type { IGetRequestMessage } from "@/chain"
import type { IProof } from "@/chain"
import type { IndexerClient } from "@/client"
import type { SubstrateChain } from "@/chain"
import { RequestStatus } from "@/types"
import type { IntentsV2Context } from "./types"
import type { CancelEvent } from "./types"
import { transformOrderForContract, fetchSourceProof, getFeeToken, convertGasToFeeToken } from "./utils"

/**
 * Handles cancellation of IntentGatewayV2 orders from either the source or
 * destination chain.
 *
 * **Source-chain cancellation** (`cancelOrderFromSource`):
 * For same-chain orders, encodes a direct `cancelOrder` call and waits for
 * the `EscrowRefunded` event. For cross-chain orders, fetches a destination
 * state proof, submits a GET request, waits for Hyperbridge finalization, and
 * submits the proof to unlock the escrowed funds.
 *
 * **Destination-chain cancellation** (`cancelOrderFromDest`):
 * Submits a `cancelOrder` call on the destination chain which dispatches an
 * ISMP POST request back to the source chain. Tracks the request through
 * Hyperbridge until the source-chain escrow is refunded.
 *
 * Both paths are resumable: intermediate state (destination proof, GET/POST
 * commitments, source proof) is persisted in `cancellationStorage` so the
 * generator can be re-entered after a crash without re-submitting transactions.
 */
export class OrderCanceller {
	/**
	 * @param ctx - Shared IntentsV2 context providing the source and destination
	 *   chain clients, config service, and cancellation storage.
	 */
	constructor(private readonly ctx: IntentsV2Context) {}

	/**
	 * Quotes the native token cost of cancelling an order from the given chain.
	 *
	 * For same-chain orders the cost is zero. For cross-chain orders, the
	 * quote covers the ISMP GET/POST dispatch fee.
	 *
	 * @param order - The order to quote a cancellation for.
	 * @param from - Which chain side initiates the cancellation (`"source"` or
	 *   `"dest"`). Defaults to `"source"`.
	 * @returns The native token amount required to submit the cancel transaction.
	 */
	async quoteCancelNative(order: OrderV2, from: "source" | "dest" = "source"): Promise<bigint> {
		if (from === "dest") {
			return this.quoteCancelNativeFromDest(order)
		}
		return this.quoteCancelNativeFromSource(order)
	}

	/**
	 * Quotes the native token cost of a source-initiated cross-chain cancellation.
	 *
	 * Constructs a mock ISMP GET request for the destination commitment slot and
	 * calls `quoteNative` on the source host to obtain the dispatch fee.
	 * Returns 0 for same-chain orders (no ISMP call needed).
	 *
	 * @param order - The order to quote.
	 * @returns The native token dispatch fee in wei.
	 */
	private async quoteCancelNativeFromSource(order: OrderV2): Promise<bigint> {
		if (order.source === order.destination) return 0n

		const height = order.deadline + 1n

		const destIntentGateway = this.ctx.dest.configService.getIntentGatewayV2Address(
			hexToString(order.destination as HexString),
		)
		const slotHash = await this.ctx.dest.client.readContract({
			abi: IntentGatewayV2ABI,
			address: destIntentGateway,
			functionName: "calculateCommitmentSlotHash",
			args: [order.id as HexString],
		})
		const key = concatHex([destIntentGateway as HexString, slotHash as HexString]) as HexString

		const context = encodeWithdrawalRequest(order, order.user as HexString)

		const getRequest: IGetRequest = {
			source: order.source.startsWith("0x") ? hexToString(order.source as HexString) : (order.source as string),
			dest: order.destination.startsWith("0x")
				? hexToString(order.destination as HexString)
				: (order.destination as string),
			from: this.ctx.source.configService.getIntentGatewayV2Address(hexToString(order.destination as HexString)),
			nonce: await this.ctx.source.getHostNonce(),
			height,
			keys: [key],
			timeoutTimestamp: 0n,
			context,
		}

		return await this.ctx.source.quoteNative(getRequest, 0n)
	}

	/**
	 * Async generator that cancels an order and streams status events until
	 * cancellation is complete.
	 *
	 * Delegates to `cancelOrderFromSource` or `cancelOrderFromDest` based on
	 * the `from` parameter.
	 *
	 * @param order - The order to cancel.
	 * @param indexerClient - Indexer client used to stream ISMP request status
	 *   updates and query state-machine heights.
	 * @param from - Which chain side initiates the cancellation. Defaults to
	 *   `"source"`.
	 * @yields {@link CancelEvent} objects describing each stage of the
	 *   cancellation lifecycle.
	 */
	async *cancelOrder(
		order: OrderV2,
		indexerClient: IndexerClient,
		from: "source" | "dest" = "source",
	): AsyncGenerator<CancelEvent> {
		if (from === "dest") {
			yield* this.cancelOrderFromDest(order, indexerClient)
			return
		}
		yield* this.cancelOrderFromSource(order, indexerClient)
	}

	/**
	 * Async generator that cancels an order by initiating the cancel from the
	 * source chain.
	 *
	 * **Same-chain path:** encodes a direct `cancelOrder` call, yields
	 * `AWAITING_CANCEL_TRANSACTION`, broadcasts the signed transaction, and
	 * yields `CANCELLATION_COMPLETE` after confirming the `EscrowRefunded` event.
	 *
	 * **Cross-chain path:**
	 * 1. Fetches (or resumes from cache) a destination finalization proof.
	 * 2. Yields `AWAITING_CANCEL_TRANSACTION` with the `cancelOrder` calldata
	 *    that includes the destination proof height.
	 * 3. Extracts the `GetRequestEvent` from the broadcast receipt and persists
	 *    the GET request.
	 * 4. Streams the GET request status through Hyperbridge, fetching and
	 *    submitting the source proof once `SOURCE_FINALIZED` is reached.
	 * 5. Cleans up persisted state when `HYPERBRIDGE_FINALIZED` is reached.
	 *
	 * @param order - The order to cancel.
	 * @param indexerClient - Used to stream GET request status and query heights.
	 * @yields {@link CancelEvent} at each lifecycle stage.
	 * @throws If the cancel transaction does not contain the expected on-chain event.
	 */
	private async *cancelOrderFromSource(order: OrderV2, indexerClient: IndexerClient): AsyncGenerator<CancelEvent> {
		const orderId = order.id!
		const isSameChain = order.source === order.destination
		const intentGatewayAddress = this.ctx.source.configService.getIntentGatewayV2Address(
			hexToString(order.source as HexString),
		)

		if (isSameChain) {
			const data = encodeFunctionData({
				abi: IntentGatewayV2ABI,
				functionName: "cancelOrder",
				args: [transformOrderForContract(order), { relayerFee: 0n, height: 0n }],
			}) as HexString

			const signedTransaction = yield {
				status: "AWAITING_CANCEL_TRANSACTION" as const,
				data,
				to: intentGatewayAddress,
				value: 0n,
			}

			const receipt = await this.ctx.source.broadcastTransaction(signedTransaction)
			const refundEvents = parseEventLogs({
				abi: IntentGatewayV2ABI,
				logs: receipt.logs,
				eventName: "EscrowRefunded",
			})
			if (refundEvents.length === 0) {
				throw new Error("EscrowRefunded event not found in cancel transaction receipt")
			}

			yield {
				status: "CANCELLATION_COMPLETE" as const,
				blockNumber: Number(receipt.blockNumber),
				transactionHash: receipt.transactionHash as HexString,
			}
			return
		}

		const hyperbridge = indexerClient.hyperbridge as SubstrateChain
		const sourceStateMachine = hexToString(order.source as HexString)
		const sourceConsensusStateId = this.ctx.source.configService.getConsensusStateId(sourceStateMachine)

		let destIProof: IProof | null = await this.ctx.cancellationStorage.getItem(STORAGE_KEYS.destProof(orderId))
		if (!destIProof) {
			destIProof = yield* this.fetchDestinationProof(order, indexerClient)
			await this.ctx.cancellationStorage.setItem(STORAGE_KEYS.destProof(orderId), destIProof)
		} else {
			yield { status: "DESTINATION_FINALIZED" as const, proof: destIProof }
		}

		let getRequest: IGetRequest | null = await this.ctx.cancellationStorage.getItem(
			STORAGE_KEYS.getRequest(orderId),
		)
		if (!getRequest) {
			const value = await this.quoteCancelNativeFromSource(order)
			const data = encodeFunctionData({
				abi: IntentGatewayV2ABI,
				functionName: "cancelOrder",
				args: [transformOrderForContract(order), { relayerFee: 0n, height: destIProof.height }],
			}) as HexString

			const signedTransaction = yield {
				status: "AWAITING_CANCEL_TRANSACTION" as const,
				data,
				to: intentGatewayAddress,
				value,
			}

			const receipt = await this.ctx.source.broadcastTransaction(signedTransaction)

			const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })
			const request = events.find((e) => e.eventName === "GetRequestEvent")
			if (!request) throw new Error("GetRequest missing")
			getRequest = request.args as unknown as IGetRequest

			await this.ctx.cancellationStorage.setItem(STORAGE_KEYS.getRequest(orderId), getRequest)
		}

		const commitment = getRequestCommitment({
			...getRequest,
			keys: [...getRequest.keys],
		})
		const sourceStatusStream = indexerClient.getRequestStatusStream(commitment)

		for await (const statusUpdate of sourceStatusStream) {
			switch (statusUpdate.status) {
				case RequestStatus.SOURCE_FINALIZED: {
					yield {
						status: "SOURCE_FINALIZED" as const,
						metadata: statusUpdate.metadata,
					}

					const sourceHeight = BigInt(statusUpdate.metadata.blockNumber)
					let sourceIProof: IProof | null = await this.ctx.cancellationStorage.getItem(
						STORAGE_KEYS.sourceProof(orderId),
					)
					if (!sourceIProof) {
						sourceIProof = await fetchSourceProof(
							commitment,
							this.ctx.source,
							sourceStateMachine,
							sourceConsensusStateId,
							sourceHeight,
						)
						await this.ctx.cancellationStorage.setItem(STORAGE_KEYS.sourceProof(orderId), sourceIProof)
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
					break
				}

				case RequestStatus.HYPERBRIDGE_DELIVERED:
					yield {
						status: "HYPERBRIDGE_DELIVERED" as const,
						metadata: statusUpdate.metadata,
					}
					break

				case RequestStatus.HYPERBRIDGE_FINALIZED:
					yield {
						status: "HYPERBRIDGE_FINALIZED" as const,
						metadata: statusUpdate.metadata,
					}
					await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.destProof(orderId))
					await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.getRequest(orderId))
					await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.sourceProof(orderId))
					return
			}
		}
	}

	/**
	 * Quotes the native token cost of a destination-initiated cross-chain cancellation.
	 *
	 * Estimates the relayer fee for delivering the refund POST request from the
	 * destination chain back to the source chain, converts it to the destination
	 * fee token, and calls `quoteNative` on the destination host.
	 * Returns 0 for same-chain orders.
	 *
	 * @param order - The order to quote.
	 * @returns The native token dispatch fee in wei.
	 */
	private async quoteCancelNativeFromDest(order: OrderV2): Promise<bigint> {
		if (order.source === order.destination) return 0n

		const destStateMachine = order.destination.startsWith("0x")
			? hexToString(order.destination as HexString)
			: (order.destination as string)
		const sourceStateMachine = order.source.startsWith("0x")
			? hexToString(order.source as HexString)
			: (order.source as string)

		const destIntentGateway = this.ctx.dest.configService.getIntentGatewayV2Address(destStateMachine)
		const sourceIntentGateway = this.ctx.source.configService.getIntentGatewayV2Address(sourceStateMachine)

		const relayerFee = await this.estimateRelayerFee(sourceStateMachine, destStateMachine)

		const body = constructRefundEscrowRequestBody(order, order.user as HexString)

		const postRequest: IPostRequest = {
			source: destStateMachine,
			dest: sourceStateMachine,
			from: destIntentGateway,
			to: sourceIntentGateway,
			nonce: await this.ctx.dest.getHostNonce(),
			body,
			timeoutTimestamp: 0n,
		}

		return await this.ctx.dest.quoteNative(postRequest, relayerFee)
	}

	/**
	 * Async generator that cancels an order by initiating from the destination
	 * chain and streaming status updates until the source-chain escrow is refunded.
	 *
	 * Throws if called with a same-chain order (use source-side cancellation instead).
	 *
	 * **Steps:**
	 * 1. Yields `AWAITING_CANCEL_TRANSACTION` so the caller can sign and submit
	 *    the cancel transaction on the destination chain.
	 * 2. Extracts the `PostRequestEvent` commitment and persists it for resumability.
	 * 3. Streams POST request status through Hyperbridge until `DESTINATION`
	 *    (i.e. the source chain processed the refund).
	 * 4. Yields `CANCELLATION_COMPLETE` and cleans up persisted state.
	 *
	 * @param order - The cross-chain order to cancel.
	 * @param indexerClient - Used to stream POST request status updates.
	 * @yields {@link CancelEvent} at each lifecycle stage.
	 * @throws If the order is same-chain, or if the cancel transaction does not
	 *   contain a `PostRequestEvent`.
	 */
	private async *cancelOrderFromDest(order: OrderV2, indexerClient: IndexerClient): AsyncGenerator<CancelEvent> {
		const orderId = order.id!

		if (order.source === order.destination) {
			throw new Error("Cannot cancel same-chain order from destination; use cancelOrder instead")
		}

		const destStateMachine = order.destination.startsWith("0x")
			? hexToString(order.destination as HexString)
			: (order.destination as string)
		const intentGatewayAddress = this.ctx.dest.configService.getIntentGatewayV2Address(destStateMachine)

		let commitment: HexString | null = await this.ctx.cancellationStorage.getItem(
			STORAGE_KEYS.postCommitment(orderId),
		)

		if (!commitment) {
			const value = await this.quoteCancelNativeFromDest(order)
			const data = encodeFunctionData({
				abi: IntentGatewayV2ABI,
				functionName: "cancelOrder",
				args: [transformOrderForContract(order), { relayerFee: 0n, height: 0n }],
			}) as HexString

			const signedTransaction = yield {
				status: "AWAITING_CANCEL_TRANSACTION" as const,
				data,
				to: intentGatewayAddress,
				value,
			}

			const receipt = await this.ctx.dest.broadcastTransaction(signedTransaction)

			const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })
			const postEvent = events.find((e) => e.eventName === "PostRequestEvent")
			if (!postEvent) throw new Error("PostRequestEvent not found in cancel transaction receipt")

			const postArgs = postEvent.args as unknown as IPostRequest
			commitment = postRequestCommitment(postArgs).commitment

			await this.ctx.cancellationStorage.setItem(STORAGE_KEYS.postCommitment(orderId), commitment)
		}

		const statusStream = indexerClient.postRequestStatusStream(commitment)

		for await (const statusUpdate of statusStream) {
			switch (statusUpdate.status) {
				case RequestStatus.SOURCE_FINALIZED:
					yield {
						status: "SOURCE_FINALIZED" as const,
						metadata: statusUpdate.metadata,
					}
					break

				case RequestStatus.HYPERBRIDGE_DELIVERED:
					yield {
						status: "HYPERBRIDGE_DELIVERED" as const,
						metadata: statusUpdate.metadata,
					}
					break

				case RequestStatus.HYPERBRIDGE_FINALIZED:
					yield {
						status: "HYPERBRIDGE_FINALIZED" as const,
						metadata: statusUpdate.metadata,
					}
					break

				case RequestStatus.DESTINATION:
					await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.postCommitment(orderId))
					yield {
						status: "CANCELLATION_COMPLETE" as const,
						blockNumber: statusUpdate.metadata.blockNumber,
						transactionHash: statusUpdate.metadata.transactionHash as HexString,
					}
					return
			}
		}
	}

	/**
	 * Polls for a finalized destination-chain state proof that demonstrates
	 * the order commitment slot is unset (i.e. the order was not filled before
	 * the deadline).
	 *
	 * Waits until the latest Hyperbridge-tracked state-machine height exceeds
	 * `order.deadline` (or the last failed probe height) before attempting to
	 * fetch the proof, then retries on failure.
	 *
	 * @param order - The order for which to fetch the destination proof.
	 * @param indexerClient - Used to query the latest known state-machine height.
	 * @yields `DESTINATION_FINALIZED` with the proof once it is successfully fetched.
	 * @returns The fetched {@link IProof} (also yielded).
	 */
	private async *fetchDestinationProof(
		order: OrderV2,
		indexerClient: IndexerClient,
	): AsyncGenerator<CancelEvent, IProof, void> {
		let latestHeight = 0n
		let lastFailedHeight: bigint | null = null

		while (true) {
			const height = await indexerClient.queryLatestStateMachineHeight({
				statemachineId: this.ctx.dest.config.stateMachineId,
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
				const intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayV2Address(
					this.ctx.dest.config.stateMachineId,
				)
				const orderId = order.id!
				const slotHash = (await this.ctx.dest.client.readContract({
					abi: IntentGatewayV2ABI,
					address: intentGatewayV2Address,
					functionName: "calculateCommitmentSlotHash",
					args: [orderId as HexString],
				})) as HexString

				const proofHex = await this.ctx.dest.queryStateProof(latestHeight, [slotHash], intentGatewayV2Address)

				const proof: IProof = {
					consensusStateId: this.ctx.dest.config.consensusStateId,
					height: latestHeight,
					proof: proofHex,
					stateMachine: this.ctx.dest.config.stateMachineId,
				}

				yield { status: "DESTINATION_FINALIZED" as const, proof }
				return proof
			} catch (e) {
				lastFailedHeight = latestHeight
				await sleep(10000)
			}
		}
	}

	/**
	 * Submits an unsigned GET request message to Hyperbridge and waits until
	 * the request receipt is confirmed on-chain.
	 *
	 * If the initial submission fails, the method waits 30 seconds and then
	 * retries querying for the receipt up to 10 times with 5-second back-off.
	 *
	 * @param hyperbridge - Hyperbridge Substrate chain client.
	 * @param commitment - The GET request commitment hash used to poll for the receipt.
	 * @param message - The fully constructed GET request message to submit.
	 */
	private async submitAndConfirmReceipt(
		hyperbridge: SubstrateChain,
		commitment: HexString,
		message: IGetRequestMessage,
	) {
		let storageValue = await hyperbridge.queryRequestReceipt(commitment)

		if (!storageValue) {
			try {
				await hyperbridge.submitUnsigned(message)
			} catch {
				// Submission failed, wait and retry
			}

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
	}

	/**
	 * Estimates the relayer fee for delivering a POST from dest to source.
	 * Converts estimated gas on the source chain into the dest chain's fee token.
	 */
	private async estimateRelayerFee(sourceChainId: string, destChainId: string): Promise<bigint> {
		const POST_REQUEST_GAS = 400_000n

		const feeInSourceFeeToken = await convertGasToFeeToken(this.ctx, POST_REQUEST_GAS, "source", sourceChainId)

		const sourceFeeToken = await getFeeToken(this.ctx, sourceChainId, this.ctx.source)
		const destFeeToken = await getFeeToken(this.ctx, destChainId, this.ctx.dest)
		const feeInDestFeeToken = adjustDecimals(feeInSourceFeeToken, sourceFeeToken.decimals, destFeeToken.decimals)

		return (feeInDestFeeToken * 1005n) / 1000n
	}
}
