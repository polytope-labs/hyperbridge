import { Status } from "@/configs/src/types"
import { RequestService } from "@/services/request.service"
import { SolanaInstruction } from "@/types/subql-solana"
import { wrap } from "@/utils/event.utils"
import { bytesToHexLower, decodeDispatchIncomingParams } from "@/utils/solana.helpers"

// `dispatchIncoming` is the host program's CPI entrypoint that delivers an
// inbound POST request from a source chain. The `commitment` parameter ties
// this delivery back to the request originated on the source chain, mirroring
// EVM's `PostRequestHandled(bytes32 commitment, address relayer)` event.
//
// instruction accounts list places the relayer at index 1
// (handler_authority is index 0).
const RELAYER_ACCOUNT_INDEX = 1

export const handleSolanaDispatchIncomingInstruction = wrap(
	async (instruction: SolanaInstruction): Promise<void> => {
		const params = decodeDispatchIncomingParams(instruction.data)
		const commitment = bytesToHexLower(params.commitment)

		const blockTimeUnix = instruction.block.blockTime
		if (blockTimeUnix === null) {
			logger.warn(
				`dispatchIncoming seen at slot ${instruction.block.slot} with no blockTime; skipping`,
			)
			return
		}

		const relayerAccount = instruction.accounts[RELAYER_ACCOUNT_INDEX]
		if (!relayerAccount) {
			logger.warn(
				`dispatchIncoming missing relayer account at index ${RELAYER_ACCOUNT_INDEX}; skipping`,
			)
			return
		}

		logger.info(
			`Solana dispatchIncoming: commitment=${commitment} relayer=${relayerAccount.pubkey} ` +
				`slot=${instruction.block.slot} tx=${instruction.transaction.signature}`,
		)

		await RequestService.updateStatus({
			commitment,
			chain: chainId,
			blockNumber: instruction.block.slot.toString(),
			blockHash: instruction.block.blockhash,
			blockTimestamp: BigInt(blockTimeUnix),
			status: Status.DESTINATION,
			transactionHash: instruction.transaction.signature,
		})
	},
)
