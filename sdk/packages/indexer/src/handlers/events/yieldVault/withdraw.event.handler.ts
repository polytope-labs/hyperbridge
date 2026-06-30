import { WithdrawLog } from "@/configs/src/types/abi-interfaces/Erc4626Abi"
import { VaultLedgerEventType } from "@/configs/src/types"
import { YieldVaultService } from "@/services/yieldVault.service"
import { getBlockTimestamp } from "@/utils/rpc.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

/**
 * Handles the ERC-4626 Withdraw event on a supported yield vault — an LP removing principal.
 * Both `withdraw` and `redeem` emit this; `owner` is the LP whose shares were burned, `receiver`
 * the address that got the assets, `sender` the caller.
 */
export const handleVaultWithdrawEvent = wrap(async (event: WithdrawLog): Promise<void> => {
	if (!event.args) return

	const { args, address, blockNumber, blockHash, transactionHash, logIndex } = event
	const chain = getHostStateMachine(chainId)
	const timestamp = await getBlockTimestamp(blockHash, chain)

	await YieldVaultService.recordLedger({
		chain,
		vault: address,
		lp: args.owner,
		caller: args.sender,
		receiver: args.receiver,
		assets: BigInt(args.assets.toString()),
		shares: BigInt(args.shares.toString()),
		eventType: VaultLedgerEventType.WITHDRAW,
		blockNumber: BigInt(blockNumber),
		transactionHash,
		logIndex,
		timestamp,
	})
})
