import { SolanaInstruction } from "@/types/subql-solana"
import { wrap } from "@/utils/event.utils"

// `veto_state_commitment` is the admin's one-way veto on a previously stored
// state commitment. The host program flips `StateCommitment.vetoed = true` so
// `handle_post_requests` against it fails closed afterwards.
//
// We don't have a `vetoed` column on `StateMachineUpdateEvent` today, and
// adding one is a GraphQL schema change that requires a coordinated
// re-deployment. Until that lands, this handler logs the veto fact at WARN
// level so it surfaces in operator dashboards but does not silently update
// rows. See solana-indexer-plan.md section 5.5 for the design choice.
//
// The vetoed `state_machine` and `height` are not present in the instruction
// data — `VetoStateCommitment` takes only the StateCommitment account
// reference. Account is at index 2 (admin, host_config,
// state_commitment); we surface its pubkey for downstream correlation but do
// not attempt to derive (state_machine, height) from the seeds without
// `@solana/web3.js`.
const STATE_COMMITMENT_ACCOUNT_INDEX = 2

export const handleSolanaVetoStateCommitmentInstruction = wrap(
	async (instruction: SolanaInstruction): Promise<void> => {
		const stateCommitmentAccount = instruction.accounts[STATE_COMMITMENT_ACCOUNT_INDEX]
		const adminAccount = instruction.accounts[0]

		logger.warn(
			`Solana state commitment vetoed: stateCommitmentPda=${stateCommitmentAccount?.pubkey ?? "unknown"} ` +
				`admin=${adminAccount?.pubkey ?? "unknown"} ` +
				`slot=${instruction.block.slot} tx=${instruction.transaction.signature}`,
		)
	},
)
