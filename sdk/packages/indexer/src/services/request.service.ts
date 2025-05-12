import { solidityKeccak256 } from "ethers/lib/utils"
import { Status } from "@/configs/src/types/enums"
import { Request, RequestStatusMetadata } from "@/configs/src/types/models"
import { ethers } from "ethers"
import { timestampToDate } from "@/utils/date.helpers"

export interface ICreateRequestArgs {
	chain: string
	commitment: string
	body?: string | undefined
	dest?: string | undefined
	fee?: bigint | undefined
	from?: string | undefined
	nonce?: bigint | undefined
	source?: string | undefined
	timeoutTimestamp?: bigint | undefined
	to?: string | undefined
	status: Status
	blockNumber: string
	blockHash: string
	transactionHash: string
	blockTimestamp: bigint
	createdAt: Date
}

export interface IUpdateRequestStatusArgs {
	commitment: string
	status: Status
	blockNumber: string
	blockHash: string
	transactionHash: string
	timeoutHash?: string
	blockTimestamp: bigint
	chain: string
}

const REQUEST_STATUS_WEIGHTS = {
	[Status.SOURCE]: 1,
	[Status.HYPERBRIDGE_DELIVERED]: 2,
	[Status.DESTINATION]: 3,
	[Status.HYPERBRIDGE_TIMED_OUT]: 4,
	[Status.TIMED_OUT]: 5,
}

export class RequestService {
	/**
	 * Finds a request entity and creates a new one if it doesn't exist
	 * If the request exists, it updates the request details
	 */
	static async createOrUpdate(args: ICreateRequestArgs): Promise<Request> {
		const {
			chain,
			commitment,
			body,
			dest,
			fee,
			from,
			nonce,
			source,
			status,
			timeoutTimestamp,
			to,
			blockNumber,
			blockHash,
			transactionHash,
			blockTimestamp,
		} = args
		let request = await Request.get(commitment)

		logger.info(
			`Processing Request: ${JSON.stringify({
				commitment,
				transactionHash,
				status,
			})}`,
		)

		if (typeof request === "undefined") {
			// Create new request if it doesn't exist
			request = Request.create({
				id: commitment,
				chain,
				body: body || "",
				dest: dest || "",
				fee: fee || BigInt(0),
				from: from || "",
				nonce: nonce || BigInt(0),
				source: source || "",
				status,
				timeoutTimestamp: timeoutTimestamp || BigInt(0),
				to: to || "",
				commitment,
				createdAt: timestampToDate(blockTimestamp),
			})

			await request.save()

			logger.info(
				`Created new request with details ${JSON.stringify({
					commitment,
					transactionHash,
					status,
				})}`,
			)
		} else {
			// Update existing request with new details if provided
			if (body !== undefined) request.body = body
			if (dest !== undefined) request.dest = dest
			if (fee !== undefined) request.fee = fee
			if (from !== undefined) request.from = from
			if (nonce !== undefined) request.nonce = nonce
			if (source !== undefined) request.source = source
			if (timeoutTimestamp !== undefined) request.timeoutTimestamp = timeoutTimestamp
			if (to !== undefined) request.to = to

			await request.save()

			logger.info(
				`Updated existing request with details ${JSON.stringify({
					commitment,
					transactionHash,
					status,
				})}`,
			)
		}

		return request
	}

	/**
	 * Update the status of a request
	 * Also adds a new entry to the request status metadata
	 */
	static async updateStatus(args: IUpdateRequestStatusArgs): Promise<void> {
		const { commitment, blockNumber, blockHash, blockTimestamp, status, transactionHash, chain } = args

		logger.info(
			`Updating Request Status: ${JSON.stringify({
				commitment,
				transactionHash,
				status,
			})}`,
		)

		let request = await Request.get(commitment)

		if (!request) {
			// Create new request and request status metadata

			await this.createOrUpdate({
				commitment,
				chain,
				body: undefined,
				dest: undefined,
				fee: undefined,
				from: undefined,
				nonce: undefined,
				source: undefined,
				timeoutTimestamp: undefined,
				to: undefined,
				blockNumber: "",
				blockHash: "",
				blockTimestamp: 0n,
				status,
				transactionHash: "",
				createdAt: timestampToDate(0n),
			})

			logger.info(
				`Created new request while attempting request update with details ${JSON.stringify({
					commitment,
					transactionHash,
					status,
				})}`,
			)
		}

		let requestStatusMetadata = RequestStatusMetadata.create({
			id: `${commitment}.${status}`,
			requestId: commitment,
			status,
			chain,
			timestamp: blockTimestamp,
			blockNumber,
			blockHash,
			transactionHash,
			createdAt: timestampToDate(blockTimestamp),
		})

		await requestStatusMetadata.save()
	}

	/**
	 * Compute the request commitment
	 */
	static computeRequestCommitment(
		source: string,
		dest: string,
		nonce: bigint,
		timeoutTimestamp: bigint,
		from: string,
		to: string,
		body: string,
	): string {
		logger.info(
			`Computing request commitment with details ${JSON.stringify({
				source,
				dest,
				nonce: nonce.toString(),
				timeoutTimestamp: timeoutTimestamp.toString(),
				from,
				to,
				body,
			})}`,
		)

		// Convert source, dest, from, to, body to bytes
		const sourceByte = ethers.utils.toUtf8Bytes(source)
		const destByte = ethers.utils.toUtf8Bytes(dest)

		let hash = solidityKeccak256(
			["bytes", "bytes", "uint64", "uint64", "bytes", "bytes", "bytes"],
			[sourceByte, destByte, nonce, timeoutTimestamp, from, to, body],
		)
		return hash
	}
}
