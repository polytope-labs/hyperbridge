import type { DatabaseSync } from "node:sqlite"

/** The one column of a `PRAGMA table_info` row this package reads. */
interface TableInfoRow {
	name: string
}

/**
 * The columns `table` currently has.
 *
 * Both stores add columns in place to databases written before those columns
 * existed, and this is how they find out what is already there. `node:sqlite`
 * types every column as `SQLOutputValue`, so the row shape is asserted here
 * once instead of at each call site.
 *
 * `table` is interpolated rather than bound because PRAGMA takes no bound
 * parameters; every caller passes a literal.
 */
export function columnNames(db: DatabaseSync, table: string): Set<string> {
	const rows = db.prepare(`PRAGMA table_info(${table})`).all() as unknown as TableInfoRow[]
	return new Set(rows.map((row) => row.name))
}
