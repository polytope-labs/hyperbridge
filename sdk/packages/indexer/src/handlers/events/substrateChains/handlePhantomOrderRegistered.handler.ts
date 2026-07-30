import { SubstrateEvent } from "@subql/types"
import { hexToU8a } from "@polkadot/util"

import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { bytes32ToBytes20 } from "@/utils/transfer.helpers"
import { PhantomOrder, PhantomOrderPair } from "@/configs/src/types"

// The pallet bundles every configured token pair into one order, so a registration carries a list of
// pairs. Their position in the list is the leg's position in the order's asset lists, which is what
// ties a pair back to the bid amounts quoted for it.
export const handlePhantomOrderRegistered = wrap(async (event: SubstrateEvent): Promise<void> => {
	const [commitmentData, chainData, createdAtData, pairsData] = event.event.data

	const commitment = commitmentData.toString()

	if (await PhantomOrder.get(commitment)) return

	const chain = Buffer.from(hexToU8a(chainData.toHex())).toString("utf8")
	const createdAtBlock = BigInt(createdAtData.toString())

	const blockHash = event.block.block.header.hash.toString()
	const blockTimestamp = await getBlockTimestamp(blockHash, chainId)

	await PhantomOrder.create({
		id: commitment,
		chain,
		createdAtBlock,
		blockTimestamp: timestampToDate(blockTimestamp),
	}).save()

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const pairs = pairsData as unknown as any[]
	for (const [pairIndex, pair] of pairs.entries()) {
		await PhantomOrderPair.create({
			id: `${commitment}-${pairIndex}`,
			orderId: commitment,
			pairIndex,
			tokenA: bytes32ToBytes20(pair.tokenA.toHex()),
			tokenB: bytes32ToBytes20(pair.tokenB.toHex()),
			standardAmount: BigInt(pair.standardAmount.toString()),
		}).save()
	}

	logger.info({ commitment, chain, pairCount: pairs.length }, "PhantomOrder indexed")
})
