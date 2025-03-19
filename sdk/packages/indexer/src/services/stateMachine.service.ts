import { StateMachineUpdateEvent } from "@/configs/src/types"
import { fetchStateCommitmentsEVM, fetchStateCommitmentsSubstrate, getStateId } from "@/utils/state-machine.helper"

// Arguments to functions that create StateMachineUpdated events
export interface ICreateStateMachineUpdatedEventArgs {
	stateMachineId: string
	height: number
	blockHash: string
	blockNumber: number
	transactionHash: string
	transactionIndex: number
	timestamp: number
	consensusStateId: string
}

export class StateMachineService {
	/**
	 * Create a new Evm Host StateMachineUpdated event entity
	 */
	static async createEvmStateMachineUpdatedEvent(
		args: ICreateStateMachineUpdatedEventArgs,
		chain: string,
	): Promise<void> {
		const { blockHash, blockNumber, transactionHash, transactionIndex, timestamp, stateMachineId, height, consensusStateId } = args

		logger.info(
			`Creating StateMachineUpdated Event: ${JSON.stringify({
				args,
			})}`,
		)

		logger.info(`Using Consensus State ID: ${consensusStateId}`)

		const stateCommitment = await fetchStateCommitmentsEVM({
			client: api,
			stateMachineId,
			consensusStateId,
			height: BigInt(height.toString()),
		})

		if (!stateCommitment) {
			logger.info(`State Commitment not found for ${stateMachineId}`)
			return
		}

		const event = StateMachineUpdateEvent.create({
			id: `${chain}_${transactionHash}_${stateMachineId}_${height}`,
			stateMachineId,
			height,
			chain,
			transactionHash,
			transactionIndex: Number(transactionIndex),
			blockHash,
			blockNumber: Number(blockNumber),
			commitmentTimestamp: stateCommitment.timestamp,
			createdAt: new Date(timestamp * 1000),
		})

		await event.save()
	}

	/**
	 * Create a new Hyperbridge StateMachineUpdated event entity
	 */
	static async createSubstrateStateMachineUpdatedEvent(
		args: ICreateStateMachineUpdatedEventArgs,
		chain: string,
	): Promise<void> {
		const { blockHash, blockNumber, transactionHash, transactionIndex, timestamp, stateMachineId, height, consensusStateId } = args

		logger.info(
			`Creating StateMachineUpdated Event: ${JSON.stringify({
				args,
			})}`,
		)

		logger.info(`Using Consensus State ID: ${consensusStateId}`)

		const stateCommitment = await fetchStateCommitmentsSubstrate({
			api: api,
			stateMachineId,
			consensusStateId,
			height: BigInt(height.toString()),
		})

		if (!stateCommitment) {
			logger.info(`State Commitment not found for ${stateMachineId}`)
			return
		}

		const event = StateMachineUpdateEvent.create({
			id: `${stateMachineId}-${transactionHash}-${height}`,
			stateMachineId,
			height,
			chain,
			transactionHash,
			transactionIndex: Number(transactionIndex),
			blockHash,
			blockNumber: Number(blockNumber),
			commitmentTimestamp: stateCommitment.timestamp,
			createdAt: new Date(timestamp * 1000),
		})

		await event.save()
	}

	/**
	 * Get updates by state machine ID
	 */
	static async getByStateMachineId(stateMachineId: string) {
		return StateMachineUpdateEvent.getByStateMachineId(stateMachineId, {
			orderBy: "height",
			limit: -1,
		})
	}

	/**
	 * Get updates by height
	 */
	static async getByHeight(height: number) {
		return StateMachineUpdateEvent.getByHeight(height, {
			orderBy: "blockNumber",
			limit: -1,
		})
	}

	/**
	 * Get updates by block number
	 */
	static async getByBlockNumber(blockNumber: number) {
		return StateMachineUpdateEvent.getByBlockNumber(blockNumber, {
			orderBy: "transactionIndex",
			limit: -1,
		})
	}

	/**
	 * Get updates by creation date
	 */
	static async getByCreatedAt(createdAt: Date) {
		return StateMachineUpdateEvent.getByCreatedAt(createdAt, {
			orderBy: "blockNumber",
			limit: -1,
		})
	}
}
