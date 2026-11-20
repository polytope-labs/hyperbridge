import { format } from "node:util"
import { createDiagnosticLog, monitorFatalErrors } from "./diagnostic-log"

// Loaded with Node --import before the solver module, including its static imports.
// The desktop owns these arguments. This module is never loaded by the normal CLI.
const index = process.argv.indexOf("--data-dir")
const dataDir = index >= 0 ? process.argv[index + 1] : undefined
if (dataDir) {
	const log = createDiagnosticLog(dataDir, "solver")
	monitorFatalErrors(log)
	const original = console.error.bind(console)
	console.error = (...args: unknown[]) => {
		// The CLI's handled startup failures use console.error followed by process.exit.
		try {
			log("stderr", format(...args))
		} catch {
			/* Preserve normal error handling. */
		}
		original(...args)
	}
}
