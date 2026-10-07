import type { DatabaseSync } from "node:sqlite"
import type { InventorySnapshot, InventoryStore } from "@/data/types"

/** A row as SQLite returns it, with the balances still as the JSON they are stored as. */
interface InventoryRow {
	id: number
	takenAt: string
	balances: string
}

/**
 * SQLite-backed {@link InventoryStore}, in `activity.db` beside the rest of the history.
 *
 * One row per snapshot, with the balances as a JSON object rather than a row per token: a
 * snapshot is only ever read whole, and the set of tokens a solver holds changes over time.
 * A database written before snapshots existed simply gains the table, with nothing in it.
 */
export class SqliteInventoryStore implements InventoryStore {
	constructor(private db: DatabaseSync) {
		this.db.exec(`
			CREATE TABLE IF NOT EXISTS inventory_snapshots (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				taken_at TEXT NOT NULL DEFAULT (datetime('now')),
				balances TEXT NOT NULL
			);

			CREATE INDEX IF NOT EXISTS idx_inventory_snapshots_taken_at ON inventory_snapshots(taken_at);
		`)
	}

	private toSnapshot(row: InventoryRow): InventorySnapshot {
		return { id: row.id, takenAt: row.takenAt, balances: JSON.parse(row.balances) }
	}

	async record(balances: Record<string, number>): Promise<void> {
		this.db.prepare("INSERT INTO inventory_snapshots (balances) VALUES (?)").run(JSON.stringify(balances))
	}

	async latest(): Promise<InventorySnapshot | null> {
		const row = this.db
			.prepare("SELECT id, taken_at as takenAt, balances FROM inventory_snapshots ORDER BY id DESC LIMIT 1")
			.get() as unknown as InventoryRow | undefined
		return row ? this.toSnapshot(row) : null
	}

	async since(from: Date): Promise<InventorySnapshot[]> {
		// The same "YYYY-MM-DD HH:MM:SS" the column holds, so the comparison is between like strings.
		const cutoff = from
			.toISOString()
			.replace("T", " ")
			.replace(/\.\d{3}Z$/, "")
		const rows = this.db
			.prepare(
				"SELECT id, taken_at as takenAt, balances FROM inventory_snapshots WHERE taken_at >= ? ORDER BY taken_at, id",
			)
			.all(cutoff) as unknown as InventoryRow[]
		return rows.map((row) => this.toSnapshot(row))
	}
}
