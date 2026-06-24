import { SubstrateEvent } from "@subql/types"
import { hexToU8a } from "@polkadot/util"

import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { PhantomOrder } from "@/configs/src/types"
import { registerPhantomCommitment } from "@/active-phantom-interval"

export const handlePhantomOrderRegistered = wrap(async (event: SubstrateEvent): Promise<void> => {
	const [commitmentData, chainData, createdAtData, tokenAData, tokenBData, standardAmountData] =
		event.event.data

	const commitment = commitmentData.toString()

	if (await PhantomOrder.get(commitment)) return

	const chain = Buffer.from(hexToU8a(chainData.toHex())).toString("utf8")
	const createdAtBlock = BigInt(createdAtData.toString())
	const tokenA = tokenAData.toHex()
	const tokenB = tokenBData.toHex()
	const standardAmount = BigInt(standardAmountData.toString())

	const blockHash = event.block.block.header.hash.toString()
	const blockTimestamp = await getBlockTimestamp(blockHash, chainId)

	await PhantomOrder.create({
		id: commitment,
		chain,
		tokenA,
		tokenB,
		standardAmount,
		createdAtBlock,
		blockTimestamp: timestampToDate(blockTimestamp),
	}).save()

	registerPhantomCommitment(commitment, createdAtBlock)

	logger.info({ commitment, chain, tokenA, tokenB }, "PhantomOrder indexed")
})
