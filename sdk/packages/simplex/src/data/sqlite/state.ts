import { readFileSync, unlinkSync } from "node:fs"
import { resolve } from "node:path"
import type { DatabaseSync } from "node:sqlite"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { RuntimeState, StateStore } from "@/data/types"

/**
 * Where operator state lived before it moved into the database, newest first:
 * beside the databases, then the pre-data-directory location. Imported once and
 * deleted — a paused filler that silently resumed on upgrade would start
 * committing capital the operator had stopped.
 */
const RETIRED_STATE_FILE = "runtime-state.json"
const RETIRED_STATE_DIR = ".filler-data"

/**
 * SQLite-backed {@link StateStore}, sharing `bids.db` with the bid store.
 *
 * `node:sqlite` is synchronous, so every method resolves immediately — the
 * promises satisfy the interface, they do not defer work.
 *
 * One row per {@link RuntimeState} key, JSON-encoded, so a write touches only
 * the keys it names. That is what the JSON file this replaces could not do: it
 * was rewritten whole and non-atomically, so a crash mid-write truncated it and
 * lost both the operator's pause and the live phantom bids — precisely the two
 * things it existed to carry across a restart — while two overlapping
 * read-modify-writes could drop one another's key.
 */
export class SqliteStateStore implements StateStore {
	private logger: Logger

	constructor(
		private db: DatabaseSync,
		dataDir: string,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		this.logger = loggers.get("state-storage")
		this.initializeSchema()
		this.importRetiredStateFile(dataDir)
	}

	private initializeSchema(): void {
		this.db.exec(`
			CREATE TABLE IF NOT EXISTS runtime_state (
				key TEXT PRIMARY KEY,
				value TEXT NOT NULL
			);
		`)
	}

	/**
	 * Moves state out of the JSON file a previous version wrote, then removes
	 * the copies it managed to read. Runs only against an empty table, so it can
	 * never overwrite state this database already holds.
	 */
	private importRetiredStateFile(dataDir: string): void {
		const { rows } = this.db.prepare("SELECT COUNT(*) as rows FROM runtime_state").get() as unknown as {
			rows: number
		}
		if (rows > 0) return

		// Resolved and de-duplicated: `--data-dir .filler-data` makes both entries
		// the same file under different strings, which would import it twice and
		// then unlink it twice — the second failing, and logging that a stale file
		// survived when it did not.
		const paths = [
			...new Set([
				resolve(dataDir, RETIRED_STATE_FILE),
				resolve(process.cwd(), RETIRED_STATE_DIR, RETIRED_STATE_FILE),
			]),
		]
		const recovered: { path: string; state: RuntimeState }[] = []

		for (const path of paths) {
			try {
				const parsed = JSON.parse(readFileSync(path, "utf-8")) as unknown
				if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
					throw new Error(`expected a JSON object, got ${Array.isArray(parsed) ? "an array" : typeof parsed}`)
				}
				recovered.push({ path, state: parsed as RuntimeState })
			} catch (err) {
				// An absent file is the normal case and says nothing. Anything else —
				// no read permission, an I/O error, malformed JSON — may still hold the
				// operator's pause, so the file stays put for them to recover. Deleting
				// it would destroy the state this import exists to rescue.
				if ((err as NodeJS.ErrnoException).code !== "ENOENT") {
					this.logger.warn({ err, path }, "Could not read the retired state file; leaving it in place")
				}
			}
		}

		const [first] = recovered
		if (first) {
			this.write(first.state, true)
			this.logger.info(
				{ path: first.path, keys: Object.keys(first.state) },
				"Imported operator state from its retired JSON file",
			)
		}

		// Delete only what was read back. A copy left behind is read again by the
		// next empty database, resurrecting a pause the operator has since lifted,
		// so every readable copy goes — not just the one that was imported.
		for (const { path } of recovered) {
			try {
				unlinkSync(path)
			} catch (err) {
				this.logger.warn({ err, path }, "Could not delete the retired state file")
			}
		}
	}

	private read(): RuntimeState {
		const rows = this.db.prepare("SELECT key, value FROM runtime_state").all() as unknown as {
			key: string
			value: string
		}[]
		const state: Record<string, unknown> = {}
		for (const row of rows) state[row.key] = JSON.parse(row.value)
		return state as RuntimeState
	}

	/**
	 * Writes the keys `state` names. With `replace`, keys it does not name are
	 * dropped — the whole-record semantics {@link StateStore.set} promises.
	 */
	private write(state: Partial<RuntimeState>, replace: boolean): void {
		const upsert = this.db.prepare(`
			INSERT INTO runtime_state (key, value) VALUES (?, ?)
			ON CONFLICT(key) DO UPDATE SET value = excluded.value
		`)
		const remove = this.db.prepare("DELETE FROM runtime_state WHERE key = ?")

		// `node:sqlite` has no transaction helper of its own, so the statements are
		// bracketed by hand. Without this a `set` could be observed with the old
		// rows deleted and the new ones not yet written.
		this.db.exec("BEGIN")
		try {
			if (replace) this.db.exec("DELETE FROM runtime_state")
			for (const [key, value] of Object.entries(state)) {
				if (value === undefined) remove.run(key)
				else upsert.run(key, JSON.stringify(value))
			}
			this.db.exec("COMMIT")
		} catch (err) {
			this.db.exec("ROLLBACK")
			throw err
		}
	}

	async get(): Promise<RuntimeState> {
		return this.read()
	}

	async set(state: RuntimeState): Promise<void> {
		this.persist(() => this.write(state, true))
	}

	/**
	 * Merges `patch` in one transaction, leaving every other key untouched. The
	 * generic {@link StateStore} fallback reads and writes back the whole record,
	 * which loses a concurrent writer's key; this cannot.
	 *
	 * The read-back is inside the guard with the write. Every production caller
	 * reaches this store through here — `Simplex.pause/resume`, the CLI's
	 * `setPaused`, the phantom batch — so a throw is a pause that reports failure
	 * while the filler is actually paused, which is the one thing {@link persist}
	 * exists to prevent. With the database unreadable the requested patch is the
	 * most that can honestly be said about the state; the failure is in the log,
	 * and no caller reads this value.
	 */
	async patch(patch: Partial<RuntimeState>): Promise<RuntimeState> {
		return (
			this.persist(() => {
				this.write(patch, false)
				return this.read()
			}) ?? patch
		)
	}

	/**
	 * Runs a write, logging rather than throwing if it fails, and returning
	 * `undefined` in that case. A pause that cannot be persisted must still pause
	 * the filler; only its survival across a restart is lost.
	 */
	private persist<T>(work: () => T): T | undefined {
		try {
			return work()
		} catch (err) {
			this.logger.warn({ err }, "Could not persist operator state")
			return undefined
		}
	}
}
