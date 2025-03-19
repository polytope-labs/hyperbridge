import { AssetTeleported } from "@/configs/src/types/models"

// Arguments for creating AssetTeleported records
export interface IAssetTeleportedArgs {
	from: string
	to: string
	amount: bigint
	dest: string
	commitment: string
	chain: string
	blockNumber: string
	blockHash: string
	blockTimestamp: bigint
}

export class AssetTeleportedService {
	/**
	 * Create or update an AssetTeleported record
	 */
	static async createOrUpdate(args: IAssetTeleportedArgs): Promise<AssetTeleported> {
		const { from, to, amount, dest, commitment, chain, blockNumber, blockHash, blockTimestamp } = args

		// Use commitment as the unique identifier for the asset teleport
		const id = commitment

		// Try to find an existing record
		let assetTeleported = await AssetTeleported.get(id)

		// If not found, create a new one
		if (!assetTeleported) {
			assetTeleported = AssetTeleported.create({
				id,
				from,
				to,
				amount,
				dest,
				commitment,
				chain,
				blockNumber: parseInt(blockNumber),
				createdAt: new Date(Number(blockTimestamp)), // Using block timestamp for createdAt instead of current time
			})
		}

		await assetTeleported.save()
		return assetTeleported
	}
}
