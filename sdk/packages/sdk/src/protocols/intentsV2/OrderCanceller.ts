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
import type { RequestStatusWithMetadata } from "@/types"
import type { IntentsV2Context } from "./types"
import type { CancelEvent } from "./types"
import { transformOrderForContract, fetchSourceProof, getFeeToken, convertGasToFeeToken } from "./utils"

export class OrderCanceller {
	constructor(private readonly ctx: IntentsV2Context) {}

	async quoteCancelNative(order: OrderV2, from: "source" | "dest" = "source"): Promise<bigint> {
		if (from === "dest") {
			return this.quoteCancelNativeFromDest(order)
		}
		return this.quoteCancelNativeFromSource(order)
	}

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

	private async *cancelOrderFromSource(order: OrderV2, indexerClient: IndexerClient): AsyncGenerator<CancelEvent> {
		const orderId = order.id!
		const isSameChain = order.source === order.destination

		if (isSameChain) {
			const intentGatewayAddress = this.ctx.source.configService.getIntentGatewayV2Address(
				hexToString(order.source as HexString),
			)

			const calldata = encodeFunctionData({
				abi: IntentGatewayV2ABI,
				functionName: "cancelOrder",
				args: [transformOrderForContract(order), { relayerFee: 0n, height: 0n }],
			}) as HexString

			const signedTransaction = yield {
				status: "AWAITING_CANCEL_TRANSACTION",
				data: { calldata, to: intentGatewayAddress },
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
				status: "CANCELLATION_COMPLETE",
				data: { metadata: { blockNumber: Number(receipt.blockNumber) } },
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
			yield { status: "DESTINATION_FINALIZED", data: { proof: destIProof } }
		}

		let getRequest: IGetRequest | null = await this.ctx.cancellationStorage.getItem(
			STORAGE_KEYS.getRequest(orderId),
		)
		if (!getRequest) {
			const transactionHash = yield {
				status: "AWAITING_GET_REQUEST",
				data: undefined,
			}
			const receipt = await this.ctx.source.client.getTransactionReceipt({
				hash: transactionHash,
			})

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
			if (statusUpdate.status === RequestStatus.SOURCE_FINALIZED) {
				yield {
					status: "SOURCE_FINALIZED",
					data: { metadata: statusUpdate.metadata },
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
				await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.destProof(orderId))
				await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.getRequest(orderId))
				await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.sourceProof(orderId))
				return
			}
		}
	}

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
			const calldata = encodeFunctionData({
				abi: IntentGatewayV2ABI,
				functionName: "cancelOrder",
				args: [transformOrderForContract(order), { relayerFee: 0n, height: 0n }],
			}) as HexString

			const signedTransaction = yield {
				status: "AWAITING_CANCEL_TRANSACTION",
				data: { calldata, to: intentGatewayAddress },
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
			if (statusUpdate.status === RequestStatus.SOURCE_FINALIZED) {
				yield {
					status: "SOURCE_FINALIZED",
					data: { metadata: statusUpdate.metadata },
				}
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
				continue
			}

			if (statusUpdate.status === RequestStatus.DESTINATION) {
				await this.ctx.cancellationStorage.removeItem(STORAGE_KEYS.postCommitment(orderId))

				yield {
					status: "CANCELLATION_COMPLETE",
					data: { metadata: statusUpdate.metadata },
				}
				return
			}
		}
	}

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

				yield { status: "DESTINATION_FINALIZED", data: { proof } }
				return proof
			} catch (e) {
				lastFailedHeight = latestHeight
				await sleep(10000)
			}
		}
	}

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
