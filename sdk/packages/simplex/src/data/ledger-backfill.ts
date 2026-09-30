import type { Logger } from "@/services/Logger"
import type { ActivityStore, WalletTx } from "./types"

/** One vault's share of a transaction, with the metadata the ledger row needs. */
export interface DescribedMovement {
	kind: "sweep" | "redeem"
	vault: string
	symbol: string
	/** Decimal string of the underlying. */
	amount: string
	shareSymbol: string
	/** Decimal string of the shares. */
	shares: string
}

export interface LedgerBackfillOptions {
	store: ActivityStore
	/** Reads a transaction's vault movements from the chain; empty when it touched no known vault. */
	describe: (chainId: number, txHash: string) => Promise<DescribedMovement[]>
	logger: Logger
	limit?: number
}

/**
 * Fills in the amounts on sweep and redeem rows recorded before the ledger
 * carried them. Best effort, like the order backfill: a receipt that cannot be
 * read leaves the row as it is. A transaction that touched several vaults
 * updates the row with the first movement and adds a row per extra vault.
 */
export async function backfillVaultLedger(options: LedgerBackfillOptions): Promise<{ rows: number }> {
	const { store, describe, logger, limit = 200 } = options
	let rows: WalletTx[]
	try {
		rows = await store.walletTxsWithoutAmounts(limit)
	} catch (err) {
		logger.warn({ err }, "Ledger backfill could not list rows without amounts")
		return { rows: 0 }
	}
	if (rows.length === 0) return { rows: 0 }
	logger.info({ rows: rows.length }, "Backfilling vault amounts on the wallet ledger")

	let filled = 0
	for (const row of rows) {
		if (row.chainId === null) continue
		try {
			const movements = (await describe(row.chainId, row.txHash)).filter((move) => move.kind === row.kind)
			const [first, ...rest] = movements
			if (!first) continue
			await store.updateWalletTx(row.id, fieldsFor(first))
			for (const extra of rest) {
				await store.recordWalletTx({
					kind: row.kind,
					chainId: row.chainId,
					txHash: row.txHash,
					sponsored: row.sponsored,
					...fieldsFor(extra),
				})
			}
			filled += 1
		} catch (err) {
			logger.warn({ err, txHash: row.txHash }, "Ledger backfill failed for a transaction")
		}
	}
	logger.info({ rows: filled }, "Ledger backfill finished")
	return { rows: filled }
}

/** A sweep gives the underlying and gets shares; a redeem gives shares and gets the underlying. */
function fieldsFor(move: DescribedMovement): Pick<WalletTx, "token" | "amount" | "to" | "tokenIn" | "amountIn"> {
	const underlying = { token: move.symbol, amount: move.amount }
	const shares = { token: move.shareSymbol, amount: move.shares }
	const out = move.kind === "sweep" ? underlying : shares
	const back = move.kind === "sweep" ? shares : underlying
	return { token: out.token, amount: out.amount, to: move.vault, tokenIn: back.token, amountIn: back.amount }
}
