import { readFileSync, writeFileSync } from "node:fs"
import { join, resolve } from "node:path"
import type { RuntimeState, StateStore } from "@/data/types"

const STATE_FILE = "runtime-state.json"

/**
 * Where operator state lived before it moved into the data directory. Read once
 * as a fallback so an upgrade does not silently resume a paused filler.
 */
const LEGACY_DIR = ".filler-data"

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
			// Fall back to the pre-move location, and rewrite it forward so the
			// migration happens once. A paused filler that silently resumed on
			// upgrade would start committing capital the operator had stopped.
			try {
				const legacy = JSON.parse(
					readFileSync(resolve(process.cwd(), LEGACY_DIR, STATE_FILE), "utf-8"),
				) as RuntimeState
				await this.set(legacy)
				return legacy
			} catch {
				return {}
			}
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
