import { mkdirSync, readFileSync, writeFileSync } from "fs"
import { join } from "path"

export interface RuntimeState {
	paused?: boolean
}

const STATE_FILE = "runtime-state.json"
const DEFAULT_DATA_DIR = ".filler-data"

/** Operator state that must survive restarts (e.g. paused stays paused). */
export function loadRuntimeState(dataDir = DEFAULT_DATA_DIR): RuntimeState {
	try {
		return JSON.parse(readFileSync(join(dataDir, STATE_FILE), "utf-8")) as RuntimeState
	} catch {
		return {}
	}
}

export function saveRuntimeState(state: RuntimeState, dataDir = DEFAULT_DATA_DIR): void {
	try {
		mkdirSync(dataDir, { recursive: true })
		writeFileSync(join(dataDir, STATE_FILE), JSON.stringify(state))
	} catch {
		// best-effort: a read-only data dir must not break pause/resume
	}
}
