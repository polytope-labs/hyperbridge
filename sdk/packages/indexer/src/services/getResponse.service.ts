import { GetResponse, Status } from "@/configs/src/types"

export interface ICreateGetResponseArgs {
	chain: string
	commitment: string
	response_message?: string[]
	responseTimeoutTimestamp?: bigint | undefined
	request?: string | undefined
	status: Status
	blockNumber: string
	blockHash: string
	transactionHash: string
	blockTimestamp: bigint
}

export class GetResponseService {
	/**
	 * Finds a response enitity and creates a new one if it doesn't exist
	 */
	static async findOrCreate(args: ICreateGetResponseArgs): Promise<GetResponse> {
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
		let response = await GetResponse.get(commitment)

		logger.info(
			`Creating GetResponse Event: ${JSON.stringify({
				commitment,
				transactionHash,
				status,
			})}`,
		)

		if (typeof response === "undefined") {
			response = GetResponse.create({
				id: commitment,
				commitment,
				chain,
				requestId: request,
				response_message: response_message || [""],
				responseTimeoutTimestamp,
				createdAt: new Date(Number(blockTimestamp)),
				blockNumber,
				blockHash,
				transactionHash,
			})

			await response.save()

			logger.info(
				`Created new get response with details ${JSON.stringify({
					commitment,
					transactionHash,
					status,
				})}`,
			)
		}

		return response
	}

	/**
	 * Find responses by chain
	 */
	static async findByChain(chain: string) {
		return GetResponse.getByChain(chain, {
			orderBy: "id",
			limit: -1,
		})
	}

	/**
	 * Find a response by commitment
	 */
	static async findByCommitment(commitment: string) {
		// Since commitment is the ID, we can just use get()
		return GetResponse.get(commitment)
	}

	/**
	 * Find responses by request ID
	 */
	static async findByRequestId(requestId: string) {
		return GetResponse.getByRequestId(requestId, {
			orderBy: "id",
			limit: -1,
		})
	}
}
