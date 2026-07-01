import { DepositLog } from "@/configs/src/types/abi-interfaces/Erc4626Abi"
import { VaultLedgerEventType } from "@/configs/src/types"
import { YieldVaultService } from "@/services/yieldVault.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

/**
 * Handles the ERC-4626 Deposit event on a supported yield vault — an LP adding principal.
 * `owner` is the share recipient (the LP), `sender` the caller.
 */
export const handleVaultDepositEvent = wrap(async (event: DepositLog): Promise<void> => {
	if (!event.args) return

	const { args, address, blockNumber, blockHash, transactionHash, logIndex } = event
	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	await YieldVaultService.recordLedger({
		chain,
		vault: address,
		lp: args.owner,
		caller: args.sender,
		assets: BigInt(args.assets.toString()),
		shares: BigInt(args.shares.toString()),
		eventType: VaultLedgerEventType.DEPOSIT,
		blockNumber: BigInt(blockNumber),
		transactionHash,
		logIndex,
		timestamp,
	})
})
