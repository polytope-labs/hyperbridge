import type { BalanceProvider, BalanceSnapshot } from "@/services/BalanceProvider"
import type { Logger } from "@/services/Logger"
import type { InventoryStore } from "./types"

/** How long after one snapshot the next is due. */
export const INVENTORY_INTERVAL_MS = 24 * 60 * 60 * 1000

/**
 * What a balance read says the solver holds: whole tokens per symbol, wallet and vaults together,
 * summed over every chain.
 *
 * Null unless every balance was read. A read that missed a chain whose RPC was down would record
 * a fall in inventory that never happened, and the profit measured against it would be wrong for
 * as long as the snapshot is used.
 */
export function inventoryOf(snapshot: BalanceSnapshot): Record<string, number> | null {
	if (snapshot.status !== "fresh") return null
	const held: Record<string, number> = {}
	for (const chain of snapshot.chains) {
		for (const asset of chain.assets) {
			if (asset.total === null) return null
			held[asset.symbol] = (held[asset.symbol] ?? 0) + asset.total
		}
	}
	return Object.keys(held).length > 0 ? held : null
}

/** Milliseconds since the epoch for the stores' "YYYY-MM-DD HH:MM:SS" UTC timestamps. */
export function parseUtc(value: string): number {
	return Date.parse(`${value.replace(" ", "T")}Z`)
}

/**
 * Records what the solver holds about once a day, from the balance reads it already makes.
 *
 * It listens rather than keeping a timer of its own: a snapshot is only worth taking when a
 * complete read has just come in, and the balance provider already refreshes far more often than
 * once a day. A solver that was down when a snapshot fell due takes one on its first complete
 * read after it starts.
 */
export class InventoryRecorder {
	/** When the last snapshot was taken. Unknown until the store has been asked once. */
	private lastTakenAt: number | undefined
	private recording = false
	private readonly onSnapshot = (snapshot: BalanceSnapshot) => void this.consider(snapshot)

	constructor(
		private store: InventoryStore,
		private balances: Pick<BalanceProvider, "on" | "off" | "getSnapshot">,
		private logger: Logger,
		private now: () => number = Date.now,
	) {}

	start(): void {
		this.balances.on("snapshot", this.onSnapshot)
		// The provider has usually finished its first read by now, and would not announce it again.
		void this.consider(this.balances.getSnapshot())
	}

	stop(): void {
		this.balances.off("snapshot", this.onSnapshot)
	}

	/** Takes a snapshot from `snapshot` if one is due and the read is complete. Never rejects. */
	async consider(snapshot: BalanceSnapshot): Promise<void> {
		// Two reads arriving together would otherwise both find a snapshot due.
		if (this.recording) return
		const held = inventoryOf(snapshot)
		if (!held) return

		this.recording = true
		try {
			if (this.lastTakenAt === undefined) {
				const latest = await this.store.latest()
				this.lastTakenAt = latest ? parseUtc(latest.takenAt) : 0
			}
			if (this.now() - this.lastTakenAt < INVENTORY_INTERVAL_MS) return
			await this.store.record(held)
			this.lastTakenAt = this.now()
			this.logger.info({ tokens: Object.keys(held).length }, "Recorded an inventory snapshot")
		} catch (err) {
			// A missed snapshot costs a figure on the analytics page; it must never cost a fill.
			this.logger.warn({ err }, "Could not record an inventory snapshot")
		} finally {
			this.recording = false
		}
	}
}
