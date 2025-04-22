import { SubstrateEvent } from "@subql/types"
import { StateMachineService } from "@/services/stateMachine.service"
import {
	extractStateMachineIdFromSubstrateEventData,
	getHostStateMachine,
	StateMachineError,
	SubstrateEventValidator,
} from "@/utils/substrate.helpers"

/**
 * Extract consensusStateId from event data
 * @param event SubstrateEvent containing StateMachineUpdated event data
 * @returns consensusStateId string if found, undefined otherwise
 */
export function extractConsensusStateIdFromEvent(event: SubstrateEvent): string | undefined {
	try {
		const { data } = event.event

		// Validate data structure
		if (!Array.isArray(data) || data.length < 1) {
			logger.error("Invalid event data structure for extracting consensusStateId")
			return undefined
		}

		// Extract the first element's JSON representation which contains consensusStateId
		const stateData = data[0].toJSON()

		// Ensure stateData is an object with consensusStateId
		if (typeof stateData !== "object" || stateData === null || !("consensusStateId" in stateData)) {
			logger.error("Missing consensusStateId in event data", { stateData })
			return undefined
		}

		let consensusStateId = stateData.consensusStateId
		if (typeof consensusStateId !== "string") {
			logger.error("Invalid consensusStateId format", { consensusStateId })
			return undefined
		}

		// Convert hex to UTF-8 string if it's a hex string
		if (consensusStateId.startsWith("0x")) {
			try {
				// Remove 0x prefix and convert to Buffer
				const buffer = Buffer.from(consensusStateId.slice(2), "hex")
				// Convert buffer to UTF-8 string
				const utf8String = buffer.toString("utf8")

				// Log the conversion for debugging
				logger.info(`Converted consensusStateId from hex ${consensusStateId} to UTF-8: ${utf8String}`)

				consensusStateId = utf8String
			} catch (hexError) {
				logger.error("Error converting hex consensusStateId to UTF-8", {
					hexError,
					originalConsensusStateId: consensusStateId,
				})
				// Return the original hex string if conversion fails
			}
		}

		// Extract the consensusStateId
		return consensusStateId
	} catch (error) {
		logger.error("Error extracting consensusStateId from event", { error })
		return undefined
	}
}

export async function handleIsmpStateMachineUpdatedEvent(event: SubstrateEvent): Promise<void> {
	logger.info(`Saw Ismp.StateMachineUpdated Event on ${getHostStateMachine(chainId)}`)

	const stateMachineId = extractStateMachineIdFromSubstrateEventData(event.event.data.toString())
	const consensusStateId = extractConsensusStateIdFromEvent(event)

	const host = getHostStateMachine(chainId)

	if (typeof stateMachineId === "undefined") {
		logger.error("Failed to extract stateMachineId from event data")
		return
	}

	if (typeof consensusStateId === "undefined") {
		logger.error("Failed to extract consensusStateId from event data")
		return
	}

	try {
		if (!SubstrateEventValidator.validateChainMetadata(host, stateMachineId)) {
			throw new StateMachineError("Invalid chain metadata", host, event.block.block.header.number.toNumber())
		}

		if (!SubstrateEventValidator.validateStateMachineEvent(event)) {
			logger.error(`Invalid state machine event data: ${JSON.stringify(event.event)}`)
			throw new StateMachineError(
				"Invalid state machine event data",
				host,
				event.block.block.header.number.toNumber(),
			)
		}

		logger.info(
			`Handling ISMP StateMachineUpdatedEvent. Block Number: ${event.block.block.header.number.toNumber()}`,
		)

		const { method, data } = event.event

		const timestamp = Math.floor(event.block.timestamp!.getTime() / 1000)
		const height = Number(data[1].toString())
		const blockNumber = event.block.block.header.number.toNumber()
		const blockHash = event.block.block.header.hash.toString()
		const transactionHash = event.extrinsic?.extrinsic?.hash?.toString() || ""
		const transactionIndex = event.extrinsic?.idx || 0

		if (isNaN(height)) {
			logger.error(`Invalid height value in event data: ${data[1].toString()}`)
			return
		}

		switch (method) {
			case "StateMachineUpdated":
				await StateMachineService.createSubstrateStateMachineUpdatedEvent(
					{
						blockHash,
						blockNumber,
						transactionHash,
						transactionIndex,
						timestamp,
						stateMachineId,
						height,
						consensusStateId,
					},
					host,
				)
				break

			default:
				throw new StateMachineError(
					`Unsupported method: ${method}`,
					host,
					event.block.block.header.number.toNumber(),
				)
		}
	} catch (error) {
		logger.error("State machine event processing failed", {
			error: error instanceof Error ? error.message : "Unknown error",
			host,
			blockNumber: event.block.block.header.number.toNumber(),
			method: event.event.method,
		})

		// Re-throw to maintain indexer state
		throw error
	}
}
