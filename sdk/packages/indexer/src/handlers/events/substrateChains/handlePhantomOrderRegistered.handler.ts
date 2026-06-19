import { SubstrateEvent } from "@subql/types"
import { wrap } from "@/utils/event.utils"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { timestampToDate } from "@/utils/date.helpers"
import { PhantomOrder } from "@/configs/src/types"

export const handlePhantomOrderRegistered = wrap(async (event: SubstrateEvent): Promise<void> => {
	logger.info("Saw IntentsCoprocessor.PhantomOrderRegistered")

	const [commitment, chainData] = event.event.data

	const host = getHostStateMachine(chainId)
	const blockHash = event.block.block.header.hash.toString()
	const blockNumber = event.block.block.header.number.toBigInt()
	const blockTimestamp = await getBlockTimestamp(blockHash, host)

	// chain is Vec<u8> on-chain; polkadot.js exposes it as Bytes whose .toHex() gives the raw bytes.
	const chainHex = chainData.toHex().replace("0x", "")
	const chain = Buffer.from(chainHex, "hex").toString("utf8")

	const existing = await PhantomOrder.get(commitment.toString())
	if (existing) {
		logger.warn(`PhantomOrder ${commitment} already indexed — skipping duplicate`)
		return
	}

	const order = PhantomOrder.create({
		id: commitment.toString(),
		chain,
		blockNumber,
		blockTimestamp: timestampToDate(blockTimestamp),
		createdAt: timestampToDate(blockTimestamp),
	})

	await order.save()
	logger.info(`PhantomOrder indexed: commitment=${commitment}, chain=${chain}`)
})
