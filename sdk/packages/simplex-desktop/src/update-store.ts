import { randomUUID } from "node:crypto"
import { mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs"
import { dirname, join } from "node:path"

export type UpdateChannel = "stable" | "beta"

export interface UpdateReceipt {
	fromVersion: string
	targetVersion: string
	downloadedAt: number
	installAttemptedAt?: number
}

export interface UpdatePreferences {
	channel: UpdateChannel
	receipt?: UpdateReceipt
	lastNagAt?: number
}

export interface UpdateStore {
	read(): UpdatePreferences
	write(value: UpdatePreferences): void
}

const DEFAULTS: UpdatePreferences = { channel: "stable" }

function validReceipt(value: unknown): value is UpdateReceipt {
	if (!value || typeof value !== "object") return false
	const receipt = value as Record<string, unknown>
	return (
		typeof receipt.fromVersion === "string" &&
		typeof receipt.targetVersion === "string" &&
		typeof receipt.downloadedAt === "number" &&
		(receipt.installAttemptedAt === undefined || typeof receipt.installAttemptedAt === "number")
	)
}

export class FileUpdateStore implements UpdateStore {
	private readonly path: string

	constructor(dataDirectory: string) {
		this.path = join(dataDirectory, "desktop-updates.json")
	}

	read(): UpdatePreferences {
		try {
			const parsed = JSON.parse(readFileSync(this.path, "utf8")) as Record<string, unknown>
			return {
				channel: parsed.channel === "beta" ? "beta" : "stable",
				receipt: validReceipt(parsed.receipt) ? parsed.receipt : undefined,
				lastNagAt: typeof parsed.lastNagAt === "number" ? parsed.lastNagAt : undefined,
			}
		} catch {
			return { ...DEFAULTS }
		}
	}

	write(value: UpdatePreferences): void {
		mkdirSync(dirname(this.path), { recursive: true })
		const temporary = `${this.path}.${process.pid}.${randomUUID()}.tmp`
		writeFileSync(temporary, `${JSON.stringify(value, null, 2)}\n`, { encoding: "utf8", mode: 0o600 })
		renameSync(temporary, this.path)
	}
}
