import { TransferLog } from "@/configs/src/types/abi-interfaces/Erc4626Abi"
import { YieldVaultService } from "@/services/yieldVault.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"
import { isOrdinaryVaultTransfer } from "@/utils/vaultAccounting"

/** Account for existing vault shares moving between owners. Mint/burns are handled separately. */
export const handleVaultTransferEvent = wrap(
	async (event: TransferLog): Promise<void> => {
		if (!event.args) throw new Error("[yield-vault] Missing decoded Transfer arguments")
		const { args, address, blockNumber, blockHash, transactionHash, logIndex } = event
		const shares = BigInt(args.value.toString())
		if (!isOrdinaryVaultTransfer(args.from, args.to, shares)) return
		const chain = getHostStateMachine(chainId)
		await YieldVaultService.recordTransfer({
			chain,
			vault: address,
			from: args.from,
			to: args.to,
			shares,
			blockNumber: BigInt(blockNumber),
			transactionHash,
			logIndex,
			timestamp: await getBlockTimestamp(blockHash, chain),
		})
	},
	{ rethrowDecodeErrors: true },
)
