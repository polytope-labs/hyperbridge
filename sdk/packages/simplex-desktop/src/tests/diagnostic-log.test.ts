import { spawnSync } from "node:child_process"
import { mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs"
import { createRequire } from "node:module"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { pathToFileURL } from "node:url"
import { afterEach, expect, it } from "vitest"
import { createDiagnosticLog } from "../diagnostic-log"

const directories: string[] = []
const require = createRequire(import.meta.url)
afterEach(() => {
	for (const dir of directories.splice(0)) rmSync(dir, { recursive: true, force: true })
})
function directory() {
	const dir = mkdtempSync(join(tmpdir(), "simplex-diagnostics-"))
	directories.push(dir)
	return dir
}

it("keeps bounded private diagnostics and redacts secrets before writing", () => {
	const dir = directory()
	const log = createDiagnosticLog(dir, "solver", { maxBytes: 512, retainedFiles: 2 })
	for (let i = 0; i < 12; i++)
		log("error", new Error(`failure-${i} key=${"f".repeat(64)} https://rpc.example/private-token`))
	const files = readdirSync(join(dir, "diagnostics"))
	expect(files).toHaveLength(2)
	for (const file of files) {
		const path = join(dir, "diagnostics", file)
		const content = readFileSync(path, "utf8")
		expect(content).not.toContain("f".repeat(64))
		expect(content).not.toContain("private-token")
		expect(Buffer.byteLength(content)).toBeLessThanOrEqual(512)
		if (process.platform !== "win32") expect(statSync(path).mode & 0o777).toBe(0o600)
	}
})

it.each(["console", "throw", "rejection", "import"])(
	"retains %s failure details with ignored stderr and preserves failure exit",
	(mode) => {
		const dir = directory()
		const fixture = join(dir, "fixture.mjs")
		const message = '"fixture failure " + "f".repeat(64)'
		writeFileSync(
			fixture,
			mode === "console"
				? `console.error(new Error(${message})); process.exit(23)`
				: mode === "rejection"
					? `Promise.reject(new Error(${message}))`
					: mode === "import"
						? 'import "./missing-fixture-module.mjs"'
						: `throw new Error(${message})`,
		)
		const child = spawnSync(
			process.execPath,
			[
				"--import",
				pathToFileURL(require.resolve("tsx")).href,
				"--import",
				new URL("../solver-diagnostics.ts", import.meta.url).href,
				fixture,
				"run",
				"--data-dir",
				dir,
			],
			{ stdio: "ignore", timeout: 10_000, env: { ...process.env, NODE_OPTIONS: "" } },
		)
		expect(child.error).toBeUndefined()
		expect(child.status).toBe(mode === "console" ? 23 : 1)
		const content = readdirSync(join(dir, "diagnostics"))
			.map((file) => readFileSync(join(dir, "diagnostics", file), "utf8"))
			.join("")
		expect(content).toContain(mode === "import" ? "Cannot find module" : "fixture failure")
		expect(content).not.toContain("f".repeat(64))
		if (mode !== "console") expect(content).toContain("uncaught")
	},
)

it("does not throw when the diagnostic directory is unavailable", () => {
	const path = join(directory(), "file")
	writeFileSync(path, "not a directory")
	expect(() => createDiagnosticLog(path, "desktop")("error", new Error("failure"))).not.toThrow()
})

it("keeps retention bounded when another writer has pruned this writer's file", () => {
	const dir = directory()
	const first = createDiagnosticLog(dir, "desktop", { retainedFiles: 1 })
	const second = createDiagnosticLog(dir, "desktop", { retainedFiles: 1 })
	first("error", "first")
	second("error", "second")
	first("error", "latest")
	const files = readdirSync(join(dir, "diagnostics"))
	expect(files).toHaveLength(1)
	expect(readFileSync(join(dir, "diagnostics", files[0]), "utf8")).toContain("latest")
})
