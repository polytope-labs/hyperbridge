import { chmodSync, renameSync, unlinkSync, writeFileSync } from "node:fs"
import { dirname, join, basename } from "node:path"
import { randomBytes } from "node:crypto"

/**
 * Atomically writes the filler config: the content lands in a same-directory
 * temp file (mode 600 — it contains private keys) and is renamed over the
 * target, so a crash mid-write can never leave a truncated config behind.
 */
export function writeConfigFileAtomic(path: string, content: string): void {
	const tempPath = join(dirname(path), `.${basename(path)}.${randomBytes(6).toString("hex")}.tmp`)
	try {
		writeFileSync(tempPath, content, { mode: 0o600 })
		chmodSync(tempPath, 0o600)
		renameSync(tempPath, path)
	} catch (error) {
		try {
			unlinkSync(tempPath)
		} catch {}
		throw error
	}
}
