import { AssetTeleportedV2, RequestV2 } from "@/configs/src/types/models"
import { timestampToDate } from "@/utils/date.helpers"

// Arguments for creating AssetTeleportedV2 records
export interface IAssetTeleportedArgs {
	from: string
	to: string
	amount: bigint
	dest: string
	commitment: string
	message_id: string
	chain: string
	blockNumber: string
	blockHash: string
	blockTimestamp: bigint
}

export class AssetTeleportedService {
	/**
	 * Create or update an AssetTeleportedV2 record
	 */
	static async createOrUpdate(args: IAssetTeleportedArgs): Promise<AssetTeleportedV2> {
		const { from, to, amount, dest, commitment, message_id, chain, blockNumber, blockHash, blockTimestamp } = args

		// Use commitment as the unique identifier for the asset teleport
		const id = message_id

		// Try to find an existing record
		let assetTeleported = await AssetTeleportedV2.get(id)

		// If not found, create a new one
		if (!assetTeleported) {
			// Try to find the associated request by commitment
			const request = await RequestV2.get(commitment)

			assetTeleported = AssetTeleportedV2.create({
				id,
				from,
				to,
				amount,
				dest,
				commitment,
				chain,
				blockNumber: parseInt(blockNumber),
				createdAt: timestampToDate(blockTimestamp), // Using block timestamp for createdAt instead of current time
				requestId: request?.id,
			})
		}

		await assetTeleported.save()
		return assetTeleported
	}
}
