import { StateMachineUpdatedLog } from "@/configs/src/types/abi-interfaces/EthereumHostAbi"
import { StateMachineService } from "@/services/stateMachine.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import stringify from "safe-stable-stringify"
import { wrap } from "@/utils/event.utils"

/**
 * Handle the StateMachineUpdated event
 */
export const handleStateMachineUpdatedEvent = wrap(async (event: StateMachineUpdatedLog): Promise<void> => {
	if (!event.args) return

	const { blockHash, blockNumber, transactionHash, transactionIndex, block, args } = event
	const { stateMachineId, height } = args

	logger.info(
		`Handling StateMachineUpdated Event: ${stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain: string = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	// Determine if we're on testnet or mainnet based on stateMachineId
	const isTestnet = stateMachineId.includes("KUSAMA")

	// Set consensusStateId to PAS0 for testnet, DOT0 for mainnet
	const consensusStateId = isTestnet ? "PAS0" : "DOT0"

	logger.info(`Using consensusStateId: ${consensusStateId} for stateMachineId: ${stateMachineId}`)

	await StateMachineService.createEvmStateMachineUpdatedEvent(
		{
			transactionHash,
			transactionIndex,
			blockHash,
			blockNumber,
			timestamp: Number(timestamp),
			stateMachineId: stateMachineId,
			height: height.toNumber(),
			consensusStateId,
		},
		chain,
	)
})
