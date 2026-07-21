import { encodeFunctionData, concatHex, parseEventLogs, pad } from "viem"
import { LogLevels, createConsola } from "consola"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import EVM_HOST from "@/abis/evmHost"
import {
	getRequestCommitment,
	postRequestCommitment,
	constructRefundEscrowRequestBody,
	encodeWithdrawalRequest,
	adjustDecimals,
	normalizeStateMachineId,
	parseStateMachineId,
	waitForChallengePeriod,
	retryPromise,
	sleep,
} from "@/utils"
import { LEGACY_STORAGE_KEYS, STORAGE_KEYS } from "@/storage"
import { MissingConsensusUpdateTimeError } from "@/utils/exceptions"
import type { Order, HexString, IGetRequest, IPostRequest, CancelOrderOptions, CancelQuote } from "@/types"
import type { IGetRequestMessage } from "@/chain"
import type { IProof } from "@/chain"
import type { IsmpClient } from "@/client"
import type { SubstrateChain } from "@/chain"
import { RequestStatus } from "@/types"
import type { IntentGatewayContext } from "./types"
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
	private static readonly DEFAULT_MAX_RECOVERY_RESTARTS = 1
	private static readonly PROOF_FRESHNESS_MAX_RETRIES = 3
	private static readonly PROOF_FRESHNESS_BACKOFF_MS = 500

	private readonly logger = createConsola({
		level: LogLevels.info,
		formatOptions: { columns: 80, colors: true, compact: true, date: false },
	}).withTag("[OrderCanceller]")

	/**
	 * @param ctx - Shared IntentsV2 context providing the source and destination
	 *   chain clients, config service, and cancellation storage.
	 */
	constructor(private readonly ctx: IntentGatewayContext) {}

	/**
	 * Returns both the native token cost and the relayer fee for cancelling an
	 * order. Frontends can use `relayerFee` to approve the ERC-20 spend before
	 * submitting the cancel transaction.
	 *
	 * @param order - The order to quote.
	 * @param options - Choose the initiation side. Defaults to source-side cancellation.
	 * @returns `{ nativeValue }` — native token amount (wei) to send as `value`;
	 *   `{ relayerFee }` — relayer incentive denominated in the chain's fee token.
	 */
	async quoteCancelOrder(order: Order, options: CancelOrderOptions = {}): Promise<CancelQuote> {
		if (options.from === "destination") {
			return this.quoteCancelFromDest(order)
		}
		return this.quoteCancelFromSource(order)
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
	private async quoteCancelFromSource(order: Order): Promise<CancelQuote> {
		const sourceStateMachine = normalizeStateMachineId(order.source)
		const destStateMachine = normalizeStateMachineId(order.destination)
		if (sourceStateMachine === destStateMachine) {
			return { nativeValue: 0n, relayerFee: 0n }
		}

		const height = order.deadline + 1n

		const destIntentGateway = this.ctx.dest.configService.getIntentGatewayAddress(
			destStateMachine,
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
			source: sourceStateMachine,
			dest: destStateMachine,
			from: this.ctx.source.configService.getIntentGatewayAddress(destStateMachine),
			nonce: await this.ctx.source.getHostNonce(),
			height,
			keys: [key],
			timeoutTimestamp: 0n,
			context,
		}

		const feeInSourceFeeToken = await convertGasToFeeToken(this.ctx, 400_000n, "source", sourceStateMachine)
		const relayerFee = (feeInSourceFeeToken * 1005n) / 1000n

		const nativeValue = await this.ctx.source.quoteNative(getRequest, relayerFee)
		return { nativeValue, relayerFee }
	}

	/**
	 * Async generator that cancels an order and streams status events until
	 * cancellation is complete.
	 *
	 * Delegates to `cancelOrderFromSource` or `cancelOrderFromDest` based on
	 * the `from` parameter. If Hyperbridge has pruned consensus data needed for
	 * source-side GET recovery, clears its stale checkpoint and restarts the
	 * source-side stream internally.
	 *
	 * @param order - The order to cancel.
	 * @param indexerClient - Indexer client used to stream ISMP request status
	 *   updates and query state-machine heights.
	 * @param options - Choose the initiation side. Defaults to source-side cancellation.
	 * @yields {@link CancelEvent} objects describing each stage of the
	 *   cancellation lifecycle.
	 */
	async *cancelOrder(
		order: Order,
		indexerClient: IsmpClient,
		options: CancelOrderOptions = {},
	): AsyncGenerator<CancelEvent> {
		const sourceStateMachine = normalizeStateMachineId(order.source)
		const destStateMachine = normalizeStateMachineId(order.destination)
		const isSameChain = sourceStateMachine === destStateMachine
		if (options.from === "destination" && !isSameChain) {
			yield* this.cancelOrderFromDest(order, indexerClient)
			return
		}

		const maxRecoveryRestarts = options.maxRecoveryRestarts ?? OrderCanceller.DEFAULT_MAX_RECOVERY_RESTARTS
		if (!Number.isInteger(maxRecoveryRestarts) || maxRecoveryRestarts < 0) {
			throw new Error("maxRecoveryRestarts must be a non-negative integer")
		}

		let recoveryRestarts = 0
		while (true) {
			try {
				yield* this.cancelOrderFromSource(order, indexerClient)
				return
			} catch (error) {
				if (!MissingConsensusUpdateTimeError.isError(error)) throw error
				if (recoveryRestarts >= maxRecoveryRestarts) {
					throw new Error(
						`Cancellation recovery stopped after ${recoveryRestarts} restart(s): Hyperbridge no longer retains a required consensus update.`,
					)
				}

				recoveryRestarts += 1
				this.logger.warn(
					`Restarting cancellation recovery (${recoveryRestarts}/${maxRecoveryRestarts}) after a required consensus update was pruned`,
				)
				await this.clearGetRecoveryCache(order)
				yield {
					status: "RECOVERY_RESTARTED",
					attempt: recoveryRestarts,
					maxAttempts: maxRecoveryRestarts,
					reason: "Hyperbridge pruned a consensus update required for cancellation recovery",
				}
			}
		}
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
	private async *cancelOrderFromSource(order: Order, indexerClient: IsmpClient): AsyncGenerator<CancelEvent> {
		const sourceStateMachine = normalizeStateMachineId(order.source)
		const destStateMachine = normalizeStateMachineId(order.destination)
		const isSameChain = sourceStateMachine === destStateMachine
		const intentGatewayAddress = this.ctx.source.configService.getIntentGatewayAddress(sourceStateMachine)

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

			const receipt =
				signedTransaction.length === 66
					? await this.ctx.source.getTransactionReceipt(signedTransaction)
					: await this.ctx.source.broadcastTransaction(signedTransaction)
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

		const storageKeys = this.recoveryStorageKeys(order)
		const legacyStorageKeys = this.legacyRecoveryStorageKeys(order)
		const hyperbridge = indexerClient.hyperbridge as SubstrateChain
		const sourceConsensusStateId = this.ctx.source.configService.getConsensusStateId(sourceStateMachine)

		let destIProof: IProof | null = await this.getRecoveryItem(
			storageKeys.destProof,
			legacyStorageKeys.map((keys) => keys.destProof),
		)
		if (!destIProof) {
			destIProof = yield* this.fetchDestinationProof(order, indexerClient)
			await this.ctx.cancellationStorage.setItem(storageKeys.destProof, destIProof)
		} else {
			let refreshed = false
			try {
				await this.assertProofFresh(hyperbridge, destIProof)
			} catch (error) {
				if (!MissingConsensusUpdateTimeError.isError(error)) throw error
				await this.removeRecoveryItems(
					storageKeys.destProof,
					...legacyStorageKeys.map((keys) => keys.destProof),
				)
				destIProof = yield* this.fetchDestinationProof(order, indexerClient)
				await this.ctx.cancellationStorage.setItem(storageKeys.destProof, destIProof)
				refreshed = true
			}

			if (!refreshed) yield { status: "DESTINATION_FINALIZED" as const, proof: destIProof }
		}

		// A proof fetched moments ago can still have crossed the retention boundary;
		// validate every proof immediately before deriving cancel calldata from it.
		await this.assertProofFresh(hyperbridge, destIProof)

		let getRequest: IGetRequest | null = await this.getRecoveryItem(
			storageKeys.getRequest,
			legacyStorageKeys.map((keys) => keys.getRequest),
		)
		if (!getRequest) {
			const quote = await this.quoteCancelFromSource(order)
			const data = encodeFunctionData({
				abi: IntentGatewayV2ABI,
				functionName: "cancelOrder",
				args: [transformOrderForContract(order), { relayerFee: quote.relayerFee, height: destIProof.height }],
			}) as HexString

			const signedTransaction = yield {
				status: "AWAITING_CANCEL_TRANSACTION" as const,
				data,
				to: intentGatewayAddress,
				value: quote.nativeValue,
			}

			const receipt =
				signedTransaction.length === 66
					? await this.ctx.source.getTransactionReceipt(signedTransaction)
					: await this.ctx.source.broadcastTransaction(signedTransaction)

			const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })
			const request = events.find((e) => e.eventName === "GetRequestEvent")
			if (!request) throw new Error("GetRequest missing")
			getRequest = request.args as unknown as IGetRequest

			await this.ctx.cancellationStorage.setItem(storageKeys.getRequest, getRequest)

			yield {
				status: "CANCEL_STARTED" as const,
				receipt,
			}
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
					let sourceIProof: IProof | null = await this.getRecoveryItem(
						storageKeys.sourceProof,
						legacyStorageKeys.map((keys) => keys.sourceProof),
					)
					if (!sourceIProof) {
						sourceIProof = await fetchSourceProof(
							commitment,
							this.ctx.source,
							sourceStateMachine,
							sourceConsensusStateId,
							sourceHeight,
						)
						await this.ctx.cancellationStorage.setItem(storageKeys.sourceProof, sourceIProof)
					}

					await this.assertProofFresh(hyperbridge, sourceIProof)

					await waitForChallengePeriod(hyperbridge, {
						height: sourceIProof.height,
						id: {
							stateId: parseStateMachineId(sourceStateMachine).stateId,
							consensusStateId: sourceConsensusStateId,
						},
					})

					// Challenge waiting can outlive a retention window. Validate both
					// inputs again immediately before submitting the proof-bearing message.
					await Promise.all([
						this.assertProofFresh(hyperbridge, sourceIProof),
						this.assertProofFresh(hyperbridge, destIProof),
					])

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
					await this.removeRecoveryItems(
						storageKeys.destProof,
						storageKeys.getRequest,
						storageKeys.sourceProof,
						...legacyStorageKeys.flatMap((keys) => [keys.destProof, keys.getRequest, keys.sourceProof]),
					)
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
	private async quoteCancelFromDest(order: Order): Promise<CancelQuote> {
		const sourceStateMachine = normalizeStateMachineId(order.source)
		const destStateMachine = normalizeStateMachineId(order.destination)
		if (sourceStateMachine === destStateMachine) {
			return { nativeValue: 0n, relayerFee: 0n }
		}

		const destIntentGateway = this.ctx.dest.configService.getIntentGatewayAddress(destStateMachine)
		const sourceIntentGateway = this.ctx.source.configService.getIntentGatewayAddress(sourceStateMachine)

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

		const nativeValue = await this.ctx.dest.quoteNative(postRequest, relayerFee)
		return { nativeValue, relayerFee }
	}

	/**
	 * Async generator that cancels an order by initiating from the destination
	 * chain and streaming status updates until the source-chain escrow is refunded.
	 *
	 * Same-chain requests are handled by the top-level router and fall back to
	 * the direct source-side cancellation path.
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
	 * @throws If the cancel transaction does not contain a `PostRequestEvent`.
	 */
	private async *cancelOrderFromDest(order: Order, indexerClient: IsmpClient): AsyncGenerator<CancelEvent> {
		const storageKeys = this.recoveryStorageKeys(order)

		const destStateMachine = normalizeStateMachineId(order.destination)
		const intentGatewayAddress = this.ctx.dest.configService.getIntentGatewayAddress(destStateMachine)

		const legacyStorageKeys = this.legacyRecoveryStorageKeys(order)
		let commitment: HexString | null = await this.getRecoveryItem(
			storageKeys.postCommitment,
			legacyStorageKeys.map((keys) => keys.postCommitment),
		)

		if (!commitment) {
			const quote = await this.quoteCancelFromDest(order)
			const data = encodeFunctionData({
				abi: IntentGatewayV2ABI,
				functionName: "cancelOrder",
				args: [transformOrderForContract(order), { relayerFee: quote.relayerFee, height: 0n }],
			}) as HexString

			const signedTransaction = yield {
				status: "AWAITING_CANCEL_TRANSACTION" as const,
				data,
				to: intentGatewayAddress,
				value: quote.nativeValue,
			}

			const receipt =
				signedTransaction.length === 66
					? await this.ctx.dest.getTransactionReceipt(signedTransaction)
					: await this.ctx.dest.broadcastTransaction(signedTransaction)

			yield {
				status: "CANCEL_STARTED" as const,
				receipt,
			}

			const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })
			const postEvent = events.find((e) => e.eventName === "PostRequestEvent")
			if (!postEvent) throw new Error("PostRequestEvent not found in cancel transaction receipt")

			const postArgs = postEvent.args as unknown as IPostRequest
			commitment = postRequestCommitment(postArgs).commitment

			await this.ctx.cancellationStorage.setItem(storageKeys.postCommitment, commitment)
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

				case RequestStatus.DESTINATION: {
					const deliveryTxHash = statusUpdate.metadata.transactionHash as HexString
					const deliveryReceipt = await this.ctx.source.getTransactionReceipt(deliveryTxHash)
					const refundEvents = parseEventLogs({
						abi: IntentGatewayV2ABI,
						logs: deliveryReceipt.logs,
						eventName: "EscrowRefunded",
					})
					if (refundEvents.length === 0) {
						throw new Error("EscrowRefunded event not found in source-chain delivery receipt")
					}
					await this.removeRecoveryItems(
						storageKeys.postCommitment,
						...legacyStorageKeys.map((keys) => keys.postCommitment),
					)
					yield {
						status: "CANCELLATION_COMPLETE" as const,
						blockNumber: statusUpdate.metadata.blockNumber,
						transactionHash: deliveryTxHash,
					}
					return
				}
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
		order: Order,
		indexerClient: IsmpClient,
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
				const intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayAddress(
					this.ctx.dest.config.stateMachineId,
				)
				const orderId = this.orderId(order)
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
				if (MissingConsensusUpdateTimeError.isError(e)) throw e
				lastFailedHeight = latestHeight
				await sleep(10000)
			}
		}
	}

	/**
	 * Submits an unsigned GET request message to Hyperbridge and waits until
	 * the GET response receipt is confirmed on-chain.
	 *
	 * GET handling on Hyperbridge creates a response receipt keyed by the
	 * request commitment. That receipt is the durable delivery signal, so a
	 * duplicate unsigned submission is considered successful only if the
	 * response receipt can be observed.
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
		this.logger.info(`Checking GET response receipt before Hyperbridge delivery (${commitment})`)
		if (await this.queryDeliveredReceipt(hyperbridge, commitment)) {
			this.logger.info(`GET ${commitment} already delivered to Hyperbridge; skipping unsigned submission`)
			return
		}

		try {
			this.logger.info(`Submitting unsigned GET ${commitment} to Hyperbridge`)
			await hyperbridge.submitUnsigned(message)
			this.logger.info(`Unsigned GET ${commitment} submitted; waiting for Hyperbridge response receipt`)
			await sleep(30000)
		} catch (error) {
			if (MissingConsensusUpdateTimeError.isError(error)) throw error
			this.logger.warn(
				`Unsigned GET submit failed for ${commitment}; polling response receipt before failing: ${String(error)}`,
			)
		}

		try {
			await this.pollDeliveredReceipt(hyperbridge, commitment)
			this.logger.info(`Confirmed Hyperbridge GET delivery for ${commitment}`)
		} catch (error) {
			if (MissingConsensusUpdateTimeError.isError(error)) throw error
			const message = `Failed to deliver GET request to Hyperbridge; no response receipt found for ${commitment}: ${String(error)}`
			this.logger.error(message)
			throw new Error(message)
		}
	}

	/** Verify that Hyperbridge still retains the consensus update for a proof. */
	private async assertProofFresh(hyperbridge: SubstrateChain, proof: IProof): Promise<void> {
		await retryPromise(
			() =>
				hyperbridge.stateMachineUpdateTime({
					height: proof.height,
					id: {
						stateId: parseStateMachineId(proof.stateMachine).stateId,
						consensusStateId: proof.consensusStateId,
					},
				}),
			{
				maxRetries: OrderCanceller.PROOF_FRESHNESS_MAX_RETRIES,
				backoffMs: OrderCanceller.PROOF_FRESHNESS_BACKOFF_MS,
				logger: this.logger,
				logMessage: `Checking consensus update time for proof at height ${proof.height}`,
				shouldRetry: (error) => !MissingConsensusUpdateTimeError.isError(error),
			},
		)
	}

	/**
	 * Drops the source-initiated GET recovery checkpoint after Hyperbridge prunes
	 * a consensus update required by that recovery attempt.
	 */
	private async clearGetRecoveryCache(order: Order): Promise<void> {
		const keys = this.recoveryStorageKeys(order)
		const legacyStorageKeys = this.legacyRecoveryStorageKeys(order)
		await this.removeRecoveryItems(
			keys.destProof,
			keys.sourceProof,
			keys.getRequest,
			...legacyStorageKeys.flatMap((legacyKeys) => [
				legacyKeys.destProof,
				legacyKeys.sourceProof,
				legacyKeys.getRequest,
			]),
		)
	}

	/**
	 * Returns the pair-scoped storage keys used to resume cancellation recovery
	 * for an order.
	 *
	 * @param order - The order whose source/destination recovery state is stored.
	 */
	private recoveryStorageKeys(order: Order) {
		const orderId = this.orderId(order)
		return {
			destProof: STORAGE_KEYS.destProof(orderId, order.source, order.destination),
			getRequest: STORAGE_KEYS.getRequest(orderId, order.source, order.destination),
			sourceProof: STORAGE_KEYS.sourceProof(orderId, order.source, order.destination),
			postCommitment: STORAGE_KEYS.postCommitment(orderId, order.source, order.destination),
		}
	}

	/** Older SDK key shapes, checked once and migrated to the normalized pair-scoped key. */
	private legacyRecoveryStorageKeys(order: Order) {
		const orderId = this.orderId(order)
		return [
			{
				destProof: LEGACY_STORAGE_KEYS.destProof(orderId, order.source, order.destination),
				getRequest: LEGACY_STORAGE_KEYS.getRequest(orderId, order.source, order.destination),
				sourceProof: LEGACY_STORAGE_KEYS.sourceProof(orderId, order.source, order.destination),
				postCommitment: LEGACY_STORAGE_KEYS.postCommitment(orderId, order.source, order.destination),
			},
			{
				destProof: LEGACY_STORAGE_KEYS.destProof(orderId),
				getRequest: LEGACY_STORAGE_KEYS.getRequest(orderId),
				sourceProof: LEGACY_STORAGE_KEYS.sourceProof(orderId),
				postCommitment: LEGACY_STORAGE_KEYS.postCommitment(orderId),
			},
		]
	}

	private async getRecoveryItem<T>(currentKey: string, legacyKeys: string[]): Promise<T | null> {
		const current = await this.ctx.cancellationStorage.getItem<T>(currentKey)
		if (current) return current

		for (const legacyKey of new Set(legacyKeys)) {
			if (legacyKey === currentKey) continue
			const legacy = await this.ctx.cancellationStorage.getItem<T>(legacyKey)
			if (!legacy) continue

			await this.ctx.cancellationStorage.setItem(currentKey, legacy)
			await this.ctx.cancellationStorage.removeItem(legacyKey)
			return legacy
		}

		return null
	}

	private async removeRecoveryItems(...keys: string[]): Promise<void> {
		await Promise.all(
			[...new Set(keys)].map((key) => this.ctx.cancellationStorage.removeItem(key)),
		)
	}

	/**
	 * Returns the order identifier required to persist or clear recovery state.
	 *
	 * @param order - The order being cancelled.
	 * @throws If the order has no identifier.
	 */
	private orderId(order: Order): string {
		if (!order.id) throw new Error("An order id is required to recover cancellation")
		return order.id
	}

	/**
	 * Reads the Hyperbridge response receipt for a GET request commitment.
	 *
	 * @param hyperbridge - Hyperbridge chain client used to query the receipt.
	 * @param commitment - Commitment of the GET request.
	 * @returns The receipt commitment when delivered, otherwise `undefined`.
	 */
	private async queryDeliveredReceipt(
		hyperbridge: SubstrateChain,
		commitment: HexString,
	): Promise<HexString | undefined> {
		return hyperbridge.queryResponseReceipt(commitment)
	}

	/**
	 * Polls Hyperbridge until it records a response receipt for the GET request.
	 *
	 * @param hyperbridge - Hyperbridge chain client used to query the receipt.
	 * @param commitment - Commitment of the GET request.
	 * @returns The delivered response receipt commitment.
	 * @throws If no receipt is observed within the configured retry limit.
	 */
	private async pollDeliveredReceipt(hyperbridge: SubstrateChain, commitment: HexString): Promise<HexString> {
		this.logger.info(`Polling Hyperbridge GET response receipt for ${commitment}`)
		return retryPromise(
			async () => {
				const value = await this.queryDeliveredReceipt(hyperbridge, commitment)
				if (!value) throw new Error(`GET response receipt not found for ${commitment}`)
				return value
			},
			{ maxRetries: 10, backoffMs: 5000, logMessage: `Checking GET response receipt ${commitment}` },
		)
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
