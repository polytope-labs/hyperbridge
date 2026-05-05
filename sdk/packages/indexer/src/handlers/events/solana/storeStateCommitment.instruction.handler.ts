import { StateMachineService } from "@/services/stateMachine.service"
import { SolanaInstruction } from "@/types/subql-solana"
import { wrap } from "@/utils/event.utils"
import {
	bytesToHexLower,
	decodeStoreStateCommitmentParams,
	sourceStateMachineIdFor,
} from "@/utils/solana.helpers"

// `chainId` is set by SubQuery to the indexer config's chain key, e.g.
// "solana-mainnet". The host's own state machine id
// `SOLANA_STATE_MACHINE = StateMachine::Substrate(*b"sola")`) is "SUBSTRATE-sola".
// We carry the indexer config chain key on the `chain` column to stay
// symmetric with the EVM and Substrate handlers, which use the same key.
export const handleSolanaStoreStateCommitmentInstruction = wrap(
	async (instruction: SolanaInstruction): Promise<void> => {
		const params = decodeStoreStateCommitmentParams(instruction.data)

		const stateMachineId = sourceStateMachineIdFor(chainId, params.stateMachine)
		const blockTimeUnix = instruction.block.blockTime
		if (blockTimeUnix === null) {
			logger.warn(
				`storeStateCommitment seen at slot ${instruction.block.slot} with no blockTime; skipping`,
			)
			return
		}

		// `host_config.consensus_client_id` is `b"BEFY"` for the SP1
		// BEEFY client. The host accepts at most one consensus client id per
		// deployment, so we hard-code rather than read from the host_config PDA
		// on every event. If a deployment ever reconfigures, lift this to the
		// indexer config alongside `ismpHost`.
		const consensusStateId = "BEFY"

		logger.info(
			`Solana storeStateCommitment: stateMachine=${stateMachineId} height=${params.height} ` +
				`slot=${instruction.block.slot} tx=${instruction.transaction.signature}`,
		)

		await StateMachineService.createSolanaStateMachineUpdatedEvent(
			{
				transactionHash: instruction.transaction.signature,
				transactionIndex: instruction.transactionIndex,
				blockHash: instruction.block.blockhash,
				blockNumber: instruction.block.slot,
				timestamp: blockTimeUnix,
				stateMachineId,
				height: Number(params.height),
				consensusStateId,
				commitmentTimestamp: params.timestamp,
				stateRootHex: bytesToHexLower(params.stateRoot),
				overlayRootHex: bytesToHexLower(params.overlayRoot),
			},
			chainId,
		)
	},
)
