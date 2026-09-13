import { Interface } from "@ethersproject/abi"
import type { EthereumLog, EthereumTransaction } from "@subql/types-ethereum"
import { extractAddressFromTopic } from "./transfer.helpers"

// These events have the same ABI in canonical EntryPoint v0.6/v0.7/v0.8.
const entryPoint = new Interface([
	"event UserOperationEvent(bytes32 indexed userOpHash, address indexed sender, address indexed paymaster, uint256 nonce, bool success, uint256 actualGasCost, uint256 actualGasUsed)",
	"event BeforeExecution()",
])
const USER_OPERATION_EVENT_TOPIC = entryPoint.getEventTopic("UserOperationEvent")
const BEFORE_EXECUTION_TOPIC = entryPoint.getEventTopic("BeforeExecution")
const ENTRY_POINTS = new Set([
	"0x5ff137d4b0fdcd49dca30c7cf57e578a026d2789",
	"0x0000000071727de22e5e9d8baf0edac6f37da032",
	"0x4337084d9e255ff0702461cf8895ce9e3b5ff108",
])

function nextUserOpEvent(logs: EthereumLog[], logIndex: number): EthereumLog | undefined {
	return logs
		.filter(
			(log) =>
				log.topics?.[0] === USER_OPERATION_EVENT_TOPIC &&
				log.topics.length === 4 &&
				ENTRY_POINTS.has(log.address.toLowerCase()) &&
				log.logIndex > logIndex,
		)
		.sort((a, b) => a.logIndex - b.logIndex)[0]
}

/** Match the first operation boundary after the log; never skip another sender's operation. */
export function findUserOpHash(logs: EthereumLog[], sender: string, logIndex: number): string | undefined {
	const event = nextUserOpEvent(logs, logIndex)
	return event && extractAddressFromTopic(event.topics[2]) === sender.toLowerCase() ? event.topics[1] : undefined
}

/** Optional placement enrichment must not prevent the order itself from being indexed. */
export async function resolvePlacementUserOpHash(
	log: Pick<EthereumLog, "logIndex" | "transactionHash"> & { transaction?: EthereumTransaction },
	sender: string,
): Promise<string | undefined> {
	try {
		if (!log.transaction) return undefined
		const receipt = await log.transaction.receipt()
		const logs = receipt.logs ?? []
		const event = nextUserOpEvent(logs, log.logIndex)
		if (!event || extractAddressFromTopic(event.topics[2]) !== sender.toLowerCase()) return undefined

		// Validation (including account creation) runs before BeforeExecution. A placement
		// there, or before a separate handleOps call, cannot be assigned to the next op.
		const start = logs
			.filter(
				(item) =>
					item.address.toLowerCase() === event.address.toLowerCase() &&
					item.topics?.[0] === BEFORE_EXECUTION_TOPIC &&
					item.logIndex < event.logIndex,
			)
			.sort((a, b) => b.logIndex - a.logIndex)[0]
		return start && start.logIndex < log.logIndex ? event.topics[1] : undefined
	} catch (error) {
		logger.warn(`Could not resolve placement userOpHash for ${log.transactionHash}: ${error}`)
		return undefined
	}
}
