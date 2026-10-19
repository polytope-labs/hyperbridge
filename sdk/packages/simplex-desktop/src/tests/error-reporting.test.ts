import { EventEmitter } from "node:events"
import type { ChildProcess } from "node:child_process"
import { mkdtempSync, readdirSync, readFileSync, rmSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import * as Sentry from "@sentry/electron/main"
import { afterEach, expect, it, vi } from "vitest"
import { spawnDaemon } from "../daemon"
import { recordDesktopDiagnostic, reportDesktopError, startDesktopErrorReporting } from "../error-reporting"
import { RendererRecovery } from "../renderer-recovery"

vi.mock("@sentry/electron/main", () => ({ captureException: vi.fn() }))
vi.mock("../diagnostic-log", async (original) => ({
	...(await original<typeof import("../diagnostic-log")>()),
	monitorFatalErrors: vi.fn(),
}))

const directories: string[] = []
afterEach(() => {
	for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true })
	vi.clearAllMocks()
})

it("shares desktop diagnostics across solver exits and reports recovery errors to Sentry only once", async () => {
	const dataDir = mkdtempSync(join(tmpdir(), "simplex-reporting-"))
	directories.push(dataDir)
	startDesktopErrorReporting("test", false, dataDir)
	const failure = new Error("reload failed")
	const recovery = new RendererRecovery({
		canRecover: () => true,
		reload: async () => {
			throw failure
		},
		prompt: async () => false,
		reportCrash: (error) => recordDesktopDiagnostic("renderer-exit", error),
		report: reportDesktopError,
	})
	await recovery.crashed("crashed", 133)
	for (let index = 0; index < 7; index++) {
		const child = Object.assign(new EventEmitter(), { unref: vi.fn() }) as unknown as ChildProcess
		spawnDaemon(
			{
				nodePath: "node",
				solverPath: "solver.js",
				socketPath: "test",
				dataDir,
				diagnosticLog: recordDesktopDiagnostic,
			},
			(() => child) as never,
			"darwin",
		)
		child.emit("exit", 1, null)
	}
	const files = readdirSync(join(dataDir, "diagnostics"))
	expect(files).toHaveLength(1)
	const events = readFileSync(join(dataDir, "diagnostics", files[0]), "utf8")
		.trim()
		.split("\n")
		.map((line) => JSON.parse(line).event)
	expect(events).toEqual(["renderer-exit", "error", ...Array(7).fill("solver-exit")])
	expect(Sentry.captureException).toHaveBeenCalledExactlyOnceWith(failure)
})
