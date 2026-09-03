import { SubstrateEvent } from "@subql/types"
import { hexToU8a } from "@polkadot/util"

import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { PhantomOrderLeg, PhantomOrderV2 } from "@/configs/src/types"

// The pallet expands every configured token pair into both of its directions inside one order, so
// a registration carries a list of directed legs. Their position in the list is the leg's position
// in the order's asset lists, which is what ties a leg back to the bid amounts quoted for it.
export const handlePhantomOrderRegistered = wrap(async (event: SubstrateEvent): Promise<void> => {
	// Before pairs were bundled, the pallet emitted one order per pair as a flat
	// (commitment, chain, created_at, token_a, token_b, standard_amount). Nothing distinguishes the
	// two shapes at the filter, so pre-upgrade blocks reach this handler too, and their fourth field
	// is an H160 rather than a leg list — a Uint8Array, so reading it as legs iterates raw bytes
	// instead of failing outright. Those orders belong to the retired PhantomOrder entity, so there
	// is nothing to index for them.
	if (event.event.data.length !== 4) return

	const [commitmentData, chainData, createdAtData, legsData] = event.event.data

	const commitment = commitmentData.toString()

	if (await PhantomOrderV2.get(commitment)) return

	const chain = Buffer.from(hexToU8a(chainData.toHex())).toString("utf8")
	const createdAtBlock = BigInt(createdAtData.toString())

	const blockHash = event.block.block.header.hash.toString()
	const blockTimestamp = await getBlockTimestamp(blockHash, chainId)

	await PhantomOrderV2.create({
		id: commitment,
		chain,
		createdAtBlock,
		blockTimestamp: timestampToDate(blockTimestamp),
	}).save()

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const legs = legsData as unknown as any[]
	for (const [legIndex, leg] of legs.entries()) {
		await PhantomOrderLeg.create({
			id: `${commitment}-${legIndex}`,
			orderId: commitment,
			legIndex,
			tokenA: bytes32ToBytes20(leg.tokenA.toHex()),
			tokenB: bytes32ToBytes20(leg.tokenB.toHex()),
			standardAmount: BigInt(leg.standardAmount.toString()),
		}).save()
	}

	logger.info({ commitment, chain, legCount: legs.length }, "PhantomOrderV2 indexed")
})
