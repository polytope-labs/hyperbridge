import { describe, it, expect } from "vitest"
import { mkdtempSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import pQueue from "p-queue"
import { loadRuntimeState, saveRuntimeState } from "@/core/runtime-state"

describe("runtime-state", () => {
	it("round-trips paused state", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-state-"))
		expect(loadRuntimeState(dir)).toEqual({})
		saveRuntimeState({ paused: true }, dir)
		expect(loadRuntimeState(dir)).toEqual({ paused: true })
		saveRuntimeState({ paused: false }, dir)
		expect(loadRuntimeState(dir)).toEqual({ paused: false })
	})

	it("returns empty state for a missing directory", () => {
		expect(loadRuntimeState("/nonexistent/simplex-test-dir")).toEqual({})
	})
})

describe("paused queue drain", () => {
	// stop() clears + restarts a paused globalQueue before awaiting onIdle;
	// this pins the p-queue behavior that makes that necessary.
	it("onIdle on a paused queue with pending work never resolves until started", async () => {
		const queue = new pQueue({ concurrency: 1 })
		queue.pause()
		let ran = false
		queue.add(async () => {
			ran = true
		})

		const raced = await Promise.race([
			queue.onIdle().then(() => "idle"),
			new Promise((resolve) => setTimeout(() => resolve("timeout"), 200)),
		])
		expect(raced).toBe("timeout")
		expect(ran).toBe(false)

		queue.clear()
		queue.start()
		await queue.onIdle()
		expect(ran).toBe(false)
	})
})
