import { describe, expect, it } from "vitest"
import { mkdtempSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { MemoryDataStore } from "@/data/memory"
import { FileStateStore } from "@/data/sqlite/state"

describe("FileStateStore", () => {
	it("round-trips operator state", async () => {
		const store = new FileStateStore(mkdtempSync(join(tmpdir(), "simplex-state-")))

		expect(await store.get()).toEqual({})
		await store.set({ paused: true })
		expect(await store.get()).toEqual({ paused: true })
		await store.set({ paused: false })
		expect(await store.get()).toEqual({ paused: false })
	})

	it("reads empty state from a directory that does not exist", async () => {
		expect(await new FileStateStore("/nonexistent/simplex-test-dir").get()).toEqual({})
	})

	it("swallows write failures so an unwritable data dir cannot break pause", async () => {
		// A pause that cannot be persisted must still pause the filler; only its
		// survival across a restart is lost.
		await expect(new FileStateStore("/nonexistent/simplex-test-dir").set({ paused: true })).resolves.toBeUndefined()
	})
})

describe("MemoryDataStore state", () => {
	it("round-trips within the process", async () => {
		const store = new MemoryDataStore()
		expect(await store.state.get()).toEqual({})
		await store.state.set({ paused: true })
		expect(await store.state.get()).toEqual({ paused: true })
	})
})
