import { HyperBridgeService } from "@/services/hyperbridge.service"
import { RelayerService } from "@/services/relayer.service"
import { TransferService } from "@/services/transfer.service"
import { TransferLog } from "@/configs/src/types/abi-interfaces/ERC6160Ext20Abi"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { CHAINS_BY_ISMP_HOST } from "@/chains-by-ismp-host"

/**
 * Handles the Transfer event from the Fee Token contract
 */
export async function handleTransferEvent(event: TransferLog): Promise<void> {
	if (!event.args) return

	const { args, transactionHash, transaction, blockNumber, blockHash } = event
	const { from, to, value } = args
	const HOST_ADDRESSES = Object.values(CHAINS_BY_ISMP_HOST)

	const chain: string = getHostStateMachine(chainId)

	// Only handle transfers from/to the Hyperbridge host contracts
	if (!HOST_ADDRESSES.includes(from) && !HOST_ADDRESSES.includes(to)) {
		return
	}

	logger.info(
		`Handling Transfer event: ${JSON.stringify({
			blockNumber,
			transactionHash,
		})}`,
	)

	const transfer = await TransferService.storeTransfer({
		from,
		to,
		value: BigInt(value.toString()),
		transactionHash,
		chain,
	})

	if (HOST_ADDRESSES.includes(from)) {
		try {
			const timestamp = await getBlockTimestamp(blockHash, chain)

			await RelayerService.updateFeesEarned(transfer, timestamp)
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
