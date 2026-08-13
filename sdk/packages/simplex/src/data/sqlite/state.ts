import { readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import type { RuntimeState, StateStore } from "@/data/types"

const STATE_FILE = "runtime-state.json"

/**
 * Operator state in a small JSON file beside the SQLite databases.
 *
 * Best-effort by design: both reads and writes swallow their errors. A
 * read-only data directory should degrade to "pause does not survive restart",
 * never to a filler that refuses to pause.
 */
export class FileStateStore implements StateStore {
	private path: string

	constructor(dataDir: string) {
		this.path = join(dataDir, STATE_FILE)
	}

	async get(): Promise<RuntimeState> {
		try {
			return JSON.parse(readFileSync(this.path, "utf-8")) as RuntimeState
		} catch {
			return {}
		}
	}

	async set(state: RuntimeState): Promise<void> {
		try {
			writeFileSync(this.path, JSON.stringify(state))
		} catch {
			// best-effort: a read-only data dir must not break pause/resume
		}
	}
}
