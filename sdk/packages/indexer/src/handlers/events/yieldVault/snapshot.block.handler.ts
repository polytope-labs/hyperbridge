import { EthereumBlock } from "@subql/types-ethereum"

import { YieldVaultService } from "@/services/yieldVault.service"
import { getHostStateMachine } from "@/utils/substrate.helpers"
import { wrap } from "@/utils/event.utils"

/**
 * EVM block handler that snapshots LP yield positions on the supported vaults of this chain once per
 * UTC day. Records each LP's live share balance priced into assets (convertToAssets) alongside their
 * net principal, plus a vault-level snapshot. The modulo only sets how often this runs; the service
 * dedupes on the day bucket, so most invocations bail on a cheap store read.
 */
export const handleVaultSnapshotIndexing = wrap(async (event: EthereumBlock): Promise<void> => {
	try {
		const chain = getHostStateMachine(chainId)
		await YieldVaultService.snapshotChain(chain, BigInt(event.number), event.timestamp)
	} catch (error) {
		const message = error instanceof Error ? error.message : String(error)
		logger.error(`[handleVaultSnapshotIndexing] chain=${chainId} failed at block #${event.number}: ${message}`)
	}
})
