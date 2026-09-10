import { describe, expect, it } from "vitest"
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { MemoryDataStore } from "@/data/memory"
import { patchRuntimeState } from "@/data/state"
import { SqliteDataStore } from "@/data/sqlite"

const dataDir = () => mkdtempSync(join(tmpdir(), "simplex-state-"))

/** A store on a fresh data directory, plus the directory it was opened on. */
function openStore(dir = dataDir()) {
	const store = new SqliteDataStore(dir)
	return { dir, state: store.state, close: () => store.close() }
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

	it("reads an empty record when there is nothing to import", async () => {
		expect(await openStore().state.get()).toEqual({})
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
