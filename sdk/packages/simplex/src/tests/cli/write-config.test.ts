import { describe, it, expect } from "vitest"
import { mkdtempSync, readdirSync, readFileSync, statSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { writeConfigFileAtomic } from "@/config/write-config"

describe("writeConfigFileAtomic", () => {
	it("writes content with mode 600 and leaves no temp file behind", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-write-"))
		const path = join(dir, "filler-config.toml")

		writeConfigFileAtomic(path, "a = 1\n")
		expect(readFileSync(path, "utf-8")).toBe("a = 1\n")
		expect(statSync(path).mode & 0o777).toBe(0o600)
		expect(readdirSync(dir)).toEqual(["filler-config.toml"])

		// overwrites atomically, keeping the mode
		writeConfigFileAtomic(path, "a = 2\n")
		expect(readFileSync(path, "utf-8")).toBe("a = 2\n")
		expect(statSync(path).mode & 0o777).toBe(0o600)
		expect(readdirSync(dir)).toEqual(["filler-config.toml"])
	})

	it("cleans up the temp file when the write fails", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-write-"))
		const path = join(dir, "missing-subdir", "filler-config.toml")
		expect(() => writeConfigFileAtomic(path, "x")).toThrow()
		expect(readdirSync(dir)).toEqual([])
	})
})
