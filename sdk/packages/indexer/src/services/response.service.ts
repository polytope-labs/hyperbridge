import { solidityKeccak256 } from "ethers/lib/utils"
import { RequestV2, ResponseV2, ResponseStatusMetadata, PendingStatusMetadata, Status } from "@/configs/src/types"
import { ethers } from "ethers"
import { timestampToDate } from "@/utils/date.helpers"

const ENTITY_TYPE = "ResponseV2"

export interface ICreateResponseArgs {
	chain: string
	commitment: string
	response_message?: string | undefined
	responseTimeoutTimestamp?: bigint | undefined
	request?: RequestV2 | undefined
	status: Status
	blockNumber: string
	blockHash: string
	transactionHash: string
	blockTimestamp: bigint
}

export interface IUpdateResponseStatusArgs {
	commitment: string
	status: Status
	blockNumber: string
	blockHash: string
	transactionHash: string
	timeoutHash?: string
	blockTimestamp: bigint
	chain: string
}

const RESPONSE_STATUS_WEIGHTS = {
	[Status.SOURCE]: 1,
	[Status.HYPERBRIDGE_DELIVERED]: 2,
	[Status.DESTINATION]: 3,
	[Status.HYPERBRIDGE_TIMED_OUT]: 4,
	[Status.TIMED_OUT]: 5,
}

export class ResponseService {
	/**
	 * Finds a response enitity and creates a new one if it doesn't exist
	 */
	static async findOrCreate(args: ICreateResponseArgs): Promise<ResponseV2> {
		const {
			chain,
			commitment,
			request,
			response_message,
			responseTimeoutTimestamp,
			status,
			blockNumber,
			blockHash,
			blockTimestamp,
			transactionHash,
		} = args
		let response = await ResponseV2.get(commitment)

		logger.info(
			`Creating PostResponse Event: ${JSON.stringify({
				commitment,
				transactionHash,
				status,
			})}`,
		)

		if (typeof response === "undefined") {
			response = ResponseV2.create({
				id: commitment,
				commitment,
				chain,
				response_message,
				requestId: request?.id,
				responseTimeoutTimestamp,
				createdAt: timestampToDate(blockTimestamp),
			})

			await response.save()

			logger.info(
				`Created new response with details ${JSON.stringify({
					commitment,
					transactionHash,
					status,
				})}`,
			)

			let responseStatusMetadata = ResponseStatusMetadata.create({
				id: `${commitment}.${status}`,
				responseId: commitment,
				status,
				chain,
				timestamp: blockTimestamp,
				blockNumber,
				blockHash,
				transactionHash,
				createdAt: timestampToDate(blockTimestamp),
			})

			await responseStatusMetadata.save()

			await this.flushPendingStatuses(commitment)
		}

		return response
	}

	/**
	 * Update the status of a response
	 * Also adds a new entry to the response status metadata
	 * If the response doesn't exist, stores in PendingStatusMetadata until the entity is created
	 */
	static async updateStatus(args: IUpdateResponseStatusArgs): Promise<void> {
		const { commitment, blockNumber, blockHash, blockTimestamp, status, transactionHash, chain } = args

		let response = await ResponseV2.get(commitment)

		if (!response) {
			logger.warn(
				`ResponseV2 not found for commitment ${commitment}, storing in PendingStatusMetadata for status ${status}`,
			)

			let pending = PendingStatusMetadata.create({
				id: `${commitment}.${ENTITY_TYPE}.${status}`,
				commitment,
				entityType: ENTITY_TYPE,
				status,
				chain,
				timestamp: blockTimestamp,
				blockNumber,
				blockHash,
				transactionHash,
				createdAt: timestampToDate(blockTimestamp),
			})

			await pending.save()
			return
		}

		let responseStatusMetadata = ResponseStatusMetadata.create({
			id: `${commitment}.${status}`,
			responseId: commitment,
			status,
			chain,
			timestamp: blockTimestamp,
			blockNumber,
			blockHash,
			transactionHash,
			createdAt: timestampToDate(blockTimestamp),
		})

		await responseStatusMetadata.save()
	}

	/**
	 * Flush any pending status metadata entries for a response that was just created
	 */
	static async flushPendingStatuses(commitment: string): Promise<void> {
		const pendingStatuses = await PendingStatusMetadata.getByCommitment(commitment, {
			limit: 10,
		})

		const matching = pendingStatuses.filter((p) => p.entityType === ENTITY_TYPE)

		for (const pending of matching) {
			let statusMetadata = ResponseStatusMetadata.create({
				id: `${commitment}.${pending.status}`,
				responseId: commitment,
				status: pending.status as Status,
				chain: pending.chain,
				timestamp: pending.timestamp,
				blockNumber: pending.blockNumber,
				blockHash: pending.blockHash,
				transactionHash: pending.transactionHash,
				createdAt: pending.createdAt,
			})

			await statusMetadata.save()
			await PendingStatusMetadata.remove(pending.id)

			logger.info(
				`Flushed pending status ${pending.status} for ResponseV2 ${commitment}`,
			)
		}
	}

	/**
	 * Compute the response commitment and return the hash
	 */
	static computeResponseCommitment(
		source: string,
		dest: string,
		nonce: bigint,
		timeoutTimestamp: bigint,
		from: string,
		to: string,
		body: string,
		response: string,
		responseTimeoutTimestamp: bigint,
	): string {
		logger.info(
			`Computing response commitment with details ${JSON.stringify({
				source,
				dest,
				nonce: nonce.toString(),
				timeoutTimestamp: timeoutTimestamp.toString(),
				responseTimeoutTimestamp: responseTimeoutTimestamp.toString(),
				response,
				from,
				to,
				body,
			})}`,
		)

		// Convert source, dest, from, to, body to bytes
		const sourceByte = ethers.utils.toUtf8Bytes(source)
		const destByte = ethers.utils.toUtf8Bytes(dest)

		let hash = solidityKeccak256(
			["bytes", "bytes", "uint64", "uint64", "bytes", "bytes", "bytes", "bytes", "uint64"],
			[sourceByte, destByte, nonce, timeoutTimestamp, from, to, body, response, responseTimeoutTimestamp],
		)
		return hash
	}

	/**
	 * Find responses by chain
	 */
	static async findByChain(chain: string) {
		return ResponseV2.getByChain(chain, {
			orderBy: "id",
			limit: -1,
		})
	}

	/**
	 * Find a response by commitment
	 */
	static async findByCommitment(commitment: string) {
		// Since commitment is the ID, we can just use get()
		return ResponseV2.get(commitment)
	}

	/**
	 * Find responses by request ID
	 */
	static async findByRequestId(requestId: string) {
		return ResponseV2.getByRequestId(requestId, {
			orderBy: "id",
			limit: -1,
		})
	}
}
