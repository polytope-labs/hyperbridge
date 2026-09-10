import { describe, expect, it } from "vitest"
import { chmodSync, existsSync, mkdirSync, mkdtempSync, writeFileSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { DatabaseSync } from "node:sqlite"
import { LoggerContext, type LogSink } from "@/services/Logger"
import { MemoryDataStore } from "@/data/memory"
import { patchRuntimeState } from "@/data/state"
import { SqliteDataStore } from "@/data/sqlite"
import { SqliteStateStore } from "@/data/sqlite/state"

const dataDir = () => mkdtempSync(join(tmpdir(), "simplex-state-"))

/** Captures every log line the store emits, so a test can assert on warnings. */
function collector(): LogSink & { lines: string[] } {
	const lines: string[] = []
	return { lines, write: (line) => void lines.push(line) }
}

/**
 * A connection whose COMMIT fails the way SQLite fails one: the transaction is
 * already rolled back by the time the error reaches the caller, so `ROLLBACK`
 * from the catch block would throw over it.
 */
function commitFailsWith(db: DatabaseSync, failure: Error): DatabaseSync {
	return new Proxy(db, {
		get(target, prop) {
			if (prop === "exec") {
				return (sql: string) => {
					if (sql.trim().toUpperCase() !== "COMMIT") return target.exec(sql)
					target.exec("ROLLBACK")
					throw failure
				}
			}
			const value = Reflect.get(target, prop, target)
			return typeof value === "function" ? value.bind(target) : value
		},
	})
}

/** A store on a fresh data directory, plus the directory it was opened on. */
function openStore(dir = dataDir()) {
	const logs = collector()
	const store = new SqliteDataStore(dir, new LoggerContext({ level: "warn", sink: logs }))
	return { dir, logs, state: store.state, close: () => store.close() }
}

describe("SqliteStateStore", () => {
	it("round-trips operator state", async () => {
		const { state } = openStore()

		expect(await state.get()).toEqual({})
		await state.set({ paused: true })
		expect(await state.get()).toEqual({ paused: true })
		await state.set({ paused: false })
		expect(await state.get()).toEqual({ paused: false })
	})

	it("survives a reopen of the same data directory", async () => {
		const first = openStore()
		await first.state.set({ paused: true, phantomBids: { "EVM-8453": "0xabc" } })
		await first.close()

		const second = openStore(first.dir)
		expect(await second.state.get()).toEqual({ paused: true, phantomBids: { "EVM-8453": "0xabc" } })
	})

	it("patches one key without reading or rewriting the others", async () => {
		const { state } = openStore()
		await state.set({ paused: true, phantomBids: { "EVM-8453": "0xabc" } })

		// The whole point of the atomic merge: a pause and a phantom bid landing
		// concurrently must not drop one another.
		await Promise.all([
			patchRuntimeState(state, { paused: false }),
			patchRuntimeState(state, { phantomBids: { "EVM-1": "0xdef" } }),
		])

		expect(await state.get()).toEqual({ paused: false, phantomBids: { "EVM-1": "0xdef" } })
	})

	it("drops a key set to undefined", async () => {
		const { state } = openStore()
		await state.set({ paused: true, phantomBids: { "EVM-8453": "0xabc" } })
		await patchRuntimeState(state, { phantomBids: undefined })
		expect(await state.get()).toEqual({ paused: true })
	})

	it("replaces the whole record on set", async () => {
		const { state } = openStore()
		await state.set({ paused: true, phantomBids: { "EVM-8453": "0xabc" } })
		await state.set({ paused: false })
		expect(await state.get()).toEqual({ paused: false })
	})

	it("swallows write failures so an unwritable store cannot break pause", async () => {
		// A pause that cannot be persisted must still pause the filler; only its
		// survival across a restart is lost.
		const store = openStore()
		await store.close()
		await expect(store.state.set({ paused: true })).resolves.toBeUndefined()
	})

	it("does not reject from patch on an unwritable store either", async () => {
		// `patch` is the only path production takes — `Simplex.pause`, the CLI's
		// `setPaused` and the phantom batch all route through `patchRuntimeState`.
		// A rejection here tells the operator the pause failed while the filler is
		// in fact paused, and returns a 500 from POST /api/pause.
		const store = openStore()
		await store.close()
		await expect(store.state.patch!({ paused: true })).resolves.toEqual({ paused: true })
	})

	it("reports why a write failed, not that it could not roll back afterwards", async () => {
		// SQLite rolls a failed COMMIT back on its own, so an unconditional ROLLBACK
		// throws `cannot rollback - no transaction is active` over the real cause —
		// and the real cause is the only one that says anything. Here it reaches the
		// log; on the migration path `write` is outside `persist` and it fails a boot.
		const dir = dataDir()
		const db = new DatabaseSync(join(dir, "bids.db"))
		const logs = collector()
		const state = new SqliteStateStore(
			commitFailsWith(db, new Error("disk I/O error")),
			dir,
			new LoggerContext({ level: "warn", sink: logs }),
		)

		await state.set({ paused: true })
		db.close()

		const logged = logs.lines.join("")
		expect(logged).toContain("disk I/O error")
		expect(logged).not.toContain("cannot rollback")
	})
})

describe("retired runtime-state.json", () => {
	it("is imported on first open and then deleted", async () => {
		const dir = dataDir()
		const file = join(dir, "runtime-state.json")
		writeFileSync(file, JSON.stringify({ paused: true, phantomBids: { "EVM-8453": "0xabc" } }))

		const { state } = openStore(dir)

		expect(await state.get()).toEqual({ paused: true, phantomBids: { "EVM-8453": "0xabc" } })
		expect(existsSync(file)).toBe(false)
	})

	it("is imported from the pre-data-directory location too", async () => {
		const cwd = process.cwd()
		const home = dataDir()
		mkdirSync(join(home, ".filler-data"))
		writeFileSync(join(home, ".filler-data", "runtime-state.json"), JSON.stringify({ paused: true }))

		try {
			process.chdir(home)
			const { state } = openStore()
			expect(await state.get()).toEqual({ paused: true })
			expect(existsSync(join(home, ".filler-data", "runtime-state.json"))).toBe(false)
		} finally {
			process.chdir(cwd)
		}
	})

	it("never overwrites state the database already holds", async () => {
		const first = openStore()
		await first.state.set({ paused: false })
		await first.close()
		writeFileSync(join(first.dir, "runtime-state.json"), JSON.stringify({ paused: true }))

		const second = openStore(first.dir)
		expect(await second.state.get()).toEqual({ paused: false })
	})

	it("imports once when the data directory is the legacy directory", async () => {
		// `--data-dir .filler-data` makes both candidate paths the same file under
		// different strings. Unlinking it twice used to log that a stale copy had
		// survived when it had not.
		const cwd = process.cwd()
		const home = dataDir()
		mkdirSync(join(home, ".filler-data"))
		writeFileSync(join(home, ".filler-data", "runtime-state.json"), JSON.stringify({ paused: true }))

		try {
			process.chdir(home)
			const store = openStore(".filler-data")

			expect(await store.state.get()).toEqual({ paused: true })
			expect(existsSync(join(home, ".filler-data", "runtime-state.json"))).toBe(false)
			expect(store.logs.lines.filter((line) => line.includes("Could not delete"))).toEqual([])
		} finally {
			process.chdir(cwd)
		}
	})

	it("reads an empty record when there is nothing to import", async () => {
		expect(await openStore().state.get()).toEqual({})
	})

	it("keeps a file it could not parse instead of deleting it", async () => {
		const dir = dataDir()
		const file = join(dir, "runtime-state.json")
		writeFileSync(file, '{"paused": tru')

		const { state } = openStore(dir)

		// Deleting a file we could not read destroys the very state this import
		// exists to rescue. It stays put for the operator to recover.
		expect(await state.get()).toEqual({})
		expect(existsSync(file)).toBe(true)
	})

	it("keeps a file holding something other than an object", async () => {
		const dir = dataDir()
		const file = join(dir, "runtime-state.json")
		writeFileSync(file, "null")

		const { state } = openStore(dir)

		expect(await state.get()).toEqual({})
		expect(existsSync(file)).toBe(true)
	})

	// Root reads a mode-000 file regardless, which is how some CI containers run.
	it.skipIf(process.getuid?.() === 0)("keeps a file it could not read instead of deleting it", async () => {
		const dir = dataDir()
		const file = join(dir, "runtime-state.json")
		writeFileSync(file, JSON.stringify({ paused: true, phantomBids: { "EVM-8453": "0xabc" } }))
		chmodSync(file, 0o000)

		try {
			const { state } = openStore(dir)
			expect(await state.get()).toEqual({})
			expect(existsSync(file)).toBe(true)
		} finally {
			chmodSync(file, 0o600)
		}
	})

	it("deletes every readable copy, not just the one it imported", async () => {
		const cwd = process.cwd()
		const home = dataDir()
		const dir = join(home, "data")
		mkdirSync(dir)
		mkdirSync(join(home, ".filler-data"))
		writeFileSync(join(dir, "runtime-state.json"), JSON.stringify({ paused: true }))
		writeFileSync(join(home, ".filler-data", "runtime-state.json"), JSON.stringify({ paused: false }))

		try {
			process.chdir(home)
			const { state } = openStore(dir)

			// The data directory wins, and the older copy goes too — left behind, it
			// would be re-imported by the next empty database.
			expect(await state.get()).toEqual({ paused: true })
			expect(existsSync(join(dir, "runtime-state.json"))).toBe(false)
			expect(existsSync(join(home, ".filler-data", "runtime-state.json"))).toBe(false)
		} finally {
			process.chdir(cwd)
		}
	})
})

describe("MemoryDataStore state", () => {
	it("round-trips within the process", async () => {
		const store = new MemoryDataStore()
		expect(await store.state.get()).toEqual({})
		await store.state.set({ paused: true })
		expect(await store.state.get()).toEqual({ paused: true })
	})

	it("merges through the get-then-set fallback", async () => {
		const store = new MemoryDataStore()
		await store.state.set({ paused: true })
		await patchRuntimeState(store.state, { phantomBids: { "EVM-1": "0xdef" } })
		expect(await store.state.get()).toEqual({ paused: true, phantomBids: { "EVM-1": "0xdef" } })
	})
})
