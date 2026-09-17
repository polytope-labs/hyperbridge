import { TransferLog } from "@/configs/src/types/abi-interfaces/Erc4626Abi"
import { applyVaultShareTransfer } from "@/services/solverInventory.service"
import { YieldVaultService } from "@/services/yieldVault.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"
import { isOrdinaryVaultTransfer } from "@/utils/vaultAccounting"

/** Account for vault shares moving: every Transfer for tracked solvers, ordinary ones for the yield ledger. */
export const handleVaultTransferEvent = wrap(
	async (event: TransferLog): Promise<void> => {
		if (!event.args) throw new Error("[yield-vault] Missing decoded Transfer arguments")
		const { args, address, blockNumber, blockHash, transactionHash, logIndex } = event
		const shares = BigInt(args.value.toString())
		const chain = getHostStateMachine(chainId)
		// Tracked solvers' share balances follow every Transfer, mints and burns included; the yield
		// ledger below accounts mints and burns through Deposit/Withdraw instead.
		await applyVaultShareTransfer({
			chain,
			vault: address,
			from: args.from,
			to: args.to,
			shares,
			blockNumber: BigInt(blockNumber),
			logIndex,
			timestamp: () => getBlockTimestamp(blockHash, chain),
		})
		if (!isOrdinaryVaultTransfer(args.from, args.to, shares)) return
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
