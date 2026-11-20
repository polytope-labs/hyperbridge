import { randomUUID } from "node:crypto"
import { appendFileSync, existsSync, mkdirSync, readdirSync, unlinkSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import { scrubText } from "@hyperbridge/simplex/error-reporting"

export type DiagnosticLog = (event: string, error: unknown) => void

/** Local, scrubbed, bounded diagnostics; synchronous writes survive an immediate fatal exit. */
export function createDiagnosticLog(
	dataDir: string,
	source: "desktop" | "solver",
	options: { maxBytes?: number; retainedFiles?: number } = {},
): DiagnosticLog {
	const directory = join(dataDir, "diagnostics")
	const maxBytes = Math.max(512, options.maxBytes ?? 1024 * 1024)
	const retainedFiles = Math.max(1, options.retainedFiles ?? 5)
	let file: string | undefined
	let bytes = 0
	let lastStamp = 0
	return (event, error) => {
		try {
			let detail = scrubText(error instanceof Error ? (error.stack ?? error.message) : String(error)).slice(
				0,
				16_384,
			)
			const record = () =>
				JSON.stringify({ time: new Date().toISOString(), pid: process.pid, source, event, detail }) + "\n"
			let line = record()
			while (Buffer.byteLength(line) > maxBytes && detail.length > 0) {
				detail = detail.slice(0, Math.floor(detail.length / 2))
				line = record()
			}
			if (!file || !existsSync(file) || bytes + Buffer.byteLength(line) > maxBytes) {
				mkdirSync(directory, { recursive: true, mode: 0o700 })
				const pattern = new RegExp(`^${source}-[0-9TZ-]+-[0-9a-f-]{36}\\.log$`)
				const files = readdirSync(directory)
					.filter((name) => pattern.test(name))
					.sort()
				let remaining = files.length
				for (const stale of files) {
					if (remaining < retainedFiles) break
					try {
						unlinkSync(join(directory, stale))
						remaining--
					} catch {
						// Windows may hold a file open; do not grow retention without a bound.
					}
				}
				if (remaining >= retainedFiles) return
				lastStamp = Math.max(Date.now(), lastStamp + 1)
				const stamp = new Date(lastStamp).toISOString().replace(/[:.]/g, "-")
				const next = join(directory, `${source}-${stamp}-${randomUUID()}.log`)
				writeFileSync(next, "", { flag: "wx", mode: 0o600 })
				file = next
				bytes = 0
			}
			appendFileSync(file, line, { mode: 0o600 })
			bytes += Buffer.byteLength(line)
		} catch {
			// A failed diagnostic write must never mask the original failure or stop filling.
		}
	}
}

/** Observe fatal JS failures without suppressing Node's default failure exit. */
export function monitorFatalErrors(log: DiagnosticLog): void {
	process.on("uncaughtExceptionMonitor", (error, origin) => log(`uncaught:${origin}`, error))
	process.on("exit", (code) => {
		if (code !== 0) log("exit", `Process exited with code ${code}`)
	})
}
