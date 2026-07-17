import { describe, it, expect, afterEach } from "vitest"
import { mkdtempSync, writeFileSync, mkdirSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { discoverConfigPath } from "@/cli/discover-config"

const originalHome = process.env.SIMPLEX_HOME

afterEach(() => {
	if (originalHome === undefined) delete process.env.SIMPLEX_HOME
	else process.env.SIMPLEX_HOME = originalHome
})

describe("discoverConfigPath", () => {
	it("finds filler-config.toml in the working directory", () => {
		const cwd = mkdtempSync(join(tmpdir(), "simplex-discover-"))
		writeFileSync(join(cwd, "filler-config.toml"), "")
		expect(discoverConfigPath(cwd)).toBe(join(cwd, "filler-config.toml"))
	})

	it("falls back to $SIMPLEX_HOME/config.toml", () => {
		const cwd = mkdtempSync(join(tmpdir(), "simplex-discover-"))
		const home = mkdtempSync(join(tmpdir(), "simplex-home-"))
		mkdirSync(home, { recursive: true })
		writeFileSync(join(home, "config.toml"), "")
		process.env.SIMPLEX_HOME = home
		expect(discoverConfigPath(cwd)).toBe(join(home, "config.toml"))
	})

	it("returns undefined when nothing exists", () => {
		const cwd = mkdtempSync(join(tmpdir(), "simplex-discover-"))
		delete process.env.SIMPLEX_HOME
		expect(discoverConfigPath(cwd)).toBeUndefined()
	})
})
