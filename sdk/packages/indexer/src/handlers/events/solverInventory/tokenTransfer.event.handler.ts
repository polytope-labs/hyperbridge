import { TransferLog } from "@/configs/src/types/abi-interfaces/Erc20Abi"
import { applyTokenTransfer } from "@/services/solverInventory.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

/**
 * A Transfer of a supported token. Almost every one is between addresses this indexer does not
 * track, and those are dropped in memory; only a tracked solver's side moves its wallet balance.
 */
export const handleSolverTokenTransferEvent = wrap(async (event: TransferLog): Promise<void> => {
	if (!event.args) return
	const { args, address, blockNumber, blockHash, logIndex } = event
	const chain = getHostStateMachine(chainId)
	await applyTokenTransfer({
		chain,
		token: address,
		from: args.from,
		to: args.to,
		value: BigInt(args.value.toString()),
		blockNumber: BigInt(blockNumber),
		logIndex,
		timestamp: () => getBlockTimestamp(blockHash, chain),
	})
})
