import { SubstrateEvent } from "@subql/types"
import { formatChain, getHostStateMachine } from "@/utils/substrate.helpers"
import { AssetTeleportedService } from "@/services/assetTeleported.service"
import { decodeAddress } from "@polkadot/util-crypto"
import { u8aToHex } from "@polkadot/util"

export async function handleSubstrateAssetTeleportedEvent(event: SubstrateEvent): Promise<void> {
	logger.info(`Saw XcmGateway.AssetTeleported Event on ${getHostStateMachine(chainId)}`)

	if (!event.event.data) return

	const [from, to, amount, dest, commitment] = event.event.data

	// Convert the SS58 address to hex format
	let fromHex: string
	try {
		// Decode SS58 address to get the public key as Uint8Array
		const publicKey = decodeAddress(from.toString())
		// Convert the public key to hex format with 0x prefix
		fromHex = u8aToHex(publicKey)

		logger.info(`Decoded SS58 address ${from.toString()} to hex ${fromHex}`)
	} catch (error) {
		logger.error(`Failed to decode SS58 address ${from.toString()}: ${error}`)
		// Fall back to the original address if decoding fails
		fromHex = from.toString()
	}

	logger.info(
		`Handling AssetTeleported Event: ${JSON.stringify({
			from: fromHex,
			to: to.toString(),
			amount: amount.toString(),
			dest: dest.toString(),
			commitment: commitment.toString(),
		})}`,
	)

	const destId = formatChain(dest.toString())
	const host = getHostStateMachine(chainId)

	await AssetTeleportedService.createOrUpdate({
		from: fromHex,
		to: to.toString(),
		amount: BigInt(amount.toString()),
		dest: destId,
		commitment: commitment.toString(),
		chain: host,
		blockNumber: event.block.block.header.number.toString(),
		blockHash: event.block.block.header.hash.toString(),
		blockTimestamp: BigInt(event.block?.timestamp!.getTime()),
	})
}
