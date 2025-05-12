import { GetRequest, GetRequestStatusMetadata, Status } from "@/configs/src/types"
import { ethers } from "ethers"
import { solidityKeccak256 } from "ethers/lib/utils"
import { timestampToDate } from "@/utils/date.helpers"

export interface IGetRequestArgs {
	id: string
	source?: string
	dest?: string
	from?: string
	keys?: string[]
	nonce?: bigint
	height?: bigint
	context?: string
	timeoutTimestamp?: bigint
	fee?: bigint
	blockNumber?: string
	blockHash?: string
	transactionHash?: string
	blockTimestamp?: bigint
	status?: Status
	chain?: string
	commitment?: string
}

export interface IUpdateGetRequestStatusArgs {
	commitment: string
	blockNumber: string
	blockHash: string
	blockTimestamp: bigint
	status: Status
	transactionHash: string
	chain: string
}

export class GetRequestService {
	static async createOrUpdate(args: IGetRequestArgs): Promise<GetRequest> {
		const {
			id,
			source,
			dest,
			from,
			keys,
			nonce,
			height,
			context,
			timeoutTimestamp,
			fee,
			blockNumber,
			blockHash,
			blockTimestamp,
			transactionHash,
			status,
			chain,
		} = args
		let getRequest = await GetRequest.get(id)

		logger.info(
			`Processing Get Request: ${JSON.stringify({
				id,
				transactionHash,
				status,
			})}`,
		)

		if (!getRequest) {
			getRequest = GetRequest.create({
				id,
				chain: chain || "",
				source: source || "",
				dest: dest || "",
				from: from || "",
				keys: keys || [""],
				nonce: nonce || BigInt(0),
				height: height || BigInt(0),
				context: context || "",
				timeoutTimestamp: timeoutTimestamp || BigInt(0),
				fee: fee || BigInt(0),
				blockNumber: blockNumber || "",
				blockHash: blockHash || "",
				transactionHash: transactionHash || "",
				blockTimestamp: blockTimestamp || BigInt(0),
				status: status || Status.SOURCE,
				commitment: id,
			})

			getRequest.save()

			logger.info(
				`Saved GetRequest Event: ${JSON.stringify({
					id: getRequest.id,
				})}`,
			)
		} else {
			if (source !== undefined) getRequest.source = source
			if (dest !== undefined) getRequest.dest = dest
			if (from !== undefined) getRequest.from = from
			if (keys !== undefined) getRequest.keys = keys
			if (nonce !== undefined) getRequest.nonce = nonce
			if (height !== undefined) getRequest.height = height
			if (context !== undefined) getRequest.context = context
			if (timeoutTimestamp !== undefined) getRequest.timeoutTimestamp = timeoutTimestamp
			if (fee !== undefined) getRequest.fee = fee
			if (blockNumber !== undefined) getRequest.blockNumber = blockNumber
			if (blockHash !== undefined) getRequest.blockHash = blockHash
			if (transactionHash !== undefined) getRequest.transactionHash = transactionHash
			if (blockTimestamp !== undefined) getRequest.blockTimestamp = blockTimestamp
			if (status !== undefined) getRequest.status = status
			if (chain !== undefined) getRequest.chain = chain

			getRequest.save()

			logger.info(
				`Updated GetRequest Event: ${JSON.stringify({
					id: getRequest.id,
				})}`,
			)
		}

		return getRequest
	}

	/**
	 * Update the status of a get request
	 * Also adds a new entry to the get request status metadata
	 */
	static async updateStatus(args: IUpdateGetRequestStatusArgs): Promise<void> {
		const { commitment, blockNumber, blockHash, blockTimestamp, status, transactionHash, chain } = args

		logger.info(
			`Updating Get Request Status: ${JSON.stringify({
				commitment,
				transactionHash,
				status,
			})}`,
		)

		let getRequest = await this.createOrUpdate({
			id: commitment,
			status,
		})

		getRequest.save()

		logger.info(
			`Created new get request while attempting get request update with details ${JSON.stringify({
				commitment,
				transactionHash,
				status,
			})}`,
		)

		let getRequestStatusMetadata = GetRequestStatusMetadata.create({
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

		await getRequestStatusMetadata.save()
	}

	/**
	 * Compute the getRequest commitment matching the solidity `encode` function for GetRequestEvent
	 */
	static computeRequestCommitment(
		source: string,
		dest: string,
		nonce: bigint,
		height: bigint,
		timeoutTimestamp: bigint,
		from: string,
		keys: string[],
		context: string,
	): string {
		logger.info(
			`Computing request commitment with details ${JSON.stringify({
				source,
				dest,
				nonce: nonce.toString(),
				height: height.toString(),
				timeoutTimestamp: timeoutTimestamp.toString(),
				from,
				keys,
				context,
			})}`,
		)

		let keysEncoding = "0x".concat(keys.map((key) => key.slice(2)).join(""))

		// Convert strings to bytes
		const sourceBytes = ethers.utils.toUtf8Bytes(source)
		const destBytes = ethers.utils.toUtf8Bytes(dest)

		// Pack the data in the same order as the Solidity code
		const hash = solidityKeccak256(
			["bytes", "bytes", "uint64", "uint64", "uint64", "bytes", "bytes", "bytes"],
			[sourceBytes, destBytes, nonce, height, timeoutTimestamp, from, keysEncoding, context],
		)

		return hash
	}
}
