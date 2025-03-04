import { GET_HOST_ADDRESSES } from "../../../addresses/state-machine.addresses"
import { HyperBridgeService } from "../../../services/hyperbridge.service"
import { RelayerService } from "../../../services/relayer.service"
import { TransferService } from "../../../services/transfer.service"
import { TransferLog } from "../../../../configs/src/types/abi-interfaces/ERC6160Ext20Abi"
import { getHostStateMachine } from "../../../utils/substrate.helpers"

/**
 * Handles the Transfer event from the Fee Token contract
 */
export async function handleTransferEvent(event: TransferLog): Promise<void> {
	if (!event.args) return

	const { args, transactionHash, transaction, blockNumber } = event
	const { from, to, value } = args
	const HOST_ADDRESSES = GET_HOST_ADDRESSES()

	logger.info(
		`Handling Transfer event: ${JSON.stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const chain: string = getHostStateMachine(chainId)

	// Only store transfers from/to the Hyperbridge host contracts
	if (HOST_ADDRESSES.includes(from) || HOST_ADDRESSES.includes(to)) {
		const transfer = await TransferService.storeTransfer({
			from,
			to,
			value,
			transactionHash,
			chain,
		})

		if (HOST_ADDRESSES.includes(from)) {
			try {
				await RelayerService.updateFeesEarned(transfer)
				await HyperBridgeService.handleTransferOutOfHostAccounts(transfer, chain)
			} catch (error) {
				logger.error(
					`Error handling transfer event: ${JSON.stringify({
						error,
						transfer,
					})}`,
				)
			}
		}

		if (HOST_ADDRESSES.includes(to)) {
			await HyperBridgeService.updateTotalTransfersIn(transfer, chain)
		}
	}
}
