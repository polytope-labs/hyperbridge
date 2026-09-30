import { ERC4626_ABI } from "@/config/abis/Erc4626"
import type { HexString } from "@hyperbridge/sdk"
import { type Log, parseEventLogs } from "viem"

/** What a vault-touching transaction did, read back from its receipt. */
export interface VaultReceiptMovement {
	kind: "sweep" | "redeem"
	vault: HexString
	/** Base units of the underlying. */
	assets: bigint
	/** Base units of the share token. */
	shares: bigint
}

/**
 * ERC-4626 `Deposit` and `Withdraw` events emitted by any of `vaults` in a
 * receipt's logs. A sweep deposits, a redeem withdraws; other logs (approvals,
 * paymaster accounting, the EntryPoint) are ignored.
 */
export function vaultMovementsFromLogs(logs: Log[], vaults: Iterable<string>): VaultReceiptMovement[] {
	const known = new Set(Array.from(vaults, (address) => address.toLowerCase()))
	const events = parseEventLogs({ abi: ERC4626_ABI, logs, eventName: ["Deposit", "Withdraw"] })
	return events.flatMap((event) => {
		if (!known.has(event.address.toLowerCase())) return []
		const { assets, shares } = event.args as { assets: bigint; shares: bigint }
		return [{ kind: event.eventName === "Deposit" ? "sweep" : "redeem", vault: event.address as HexString, assets, shares }]
	})
}
