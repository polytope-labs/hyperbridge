import { describe, it, expect } from "vitest"
import { mkdtempSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
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
