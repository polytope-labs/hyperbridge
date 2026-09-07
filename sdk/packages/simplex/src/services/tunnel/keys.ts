import { createHash } from "node:crypto"
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import ssh2 from "ssh2"

// ssh2 is CommonJS: named imports resolve under vitest's transform but not in
// the ESM binary, where Node cannot see them statically.
const { utils } = ssh2

/** A key pair on disk: the OpenSSH private key text, its public line, and the SHA256 fingerprint. */
export interface StoredKey {
	privateKey: string
	/** `ssh-ed25519 AAAA… comment`, the line format `authorized_keys` and `known_hosts` use. */
	publicKey: string
	/** `SHA256:…` exactly as OpenSSH prints it. */
	fingerprint: string
}

/** A phone (or tablet) allowed to open the UI through the tunnel. */
export interface DeviceRecord {
	fingerprint: string
	label: string
	publicKey: string
	/** Unix milliseconds. */
	addedAt: number
}

/** The relay host key seen on first contact, pinned from then on. */
export interface KnownRelay {
	relay: string
	fingerprint: string
}

const DEVICE_COMMENT_PREFIX = "simplex-device:"

/** OpenSSH-style `SHA256:` fingerprint (unpadded base64) of a public key blob. */
export function fingerprintOf(publicKeyBlob: Buffer): string {
	return `SHA256:${createHash("sha256").update(publicKeyBlob).digest("base64").replace(/=+$/, "")}`
}

/** Fingerprint of an OpenSSH public-key line or private key text. */
export function fingerprintOfKeyText(keyText: string): string {
	const parsed = utils.parseKey(keyText)
	if (parsed instanceof Error) throw new Error(`Unreadable key: ${parsed.message}`)
	return fingerprintOf(parsed.getPublicSSH())
}

/**
 * Everything the tunnel keeps under `<dataDir>/tunnel/`: the operator key
 * (simplex → relay, the port identity), the host key the phone pins, the
 * authorized device keys, and the relay host key seen on first contact.
 *
 * Files are plain OpenSSH formats so an operator can inspect or hand-edit
 * them with the tools they already know. Secrets are written 0600.
 */
export class TunnelKeyStore {
	readonly dir: string

	constructor(dataDir: string) {
		this.dir = join(dataDir, "tunnel")
		mkdirSync(this.dir, { recursive: true })
	}

	/** Identity toward the relay. Rotating it leases a different public port. */
	operatorKey(): StoredKey {
		return this.loadOrCreate("operator_key", "simplex operator key")
	}

	/** Identity of the embedded SSH server; the phone pins its fingerprint. */
	hostKey(): StoredKey {
		return this.loadOrCreate("host_key", "simplex host key")
	}

	devices(): DeviceRecord[] {
		const path = join(this.dir, "authorized_keys")
		if (!existsSync(path)) return []
		const records: DeviceRecord[] = []
		for (const raw of readFileSync(path, "utf8").split("\n")) {
			const line = raw.trim()
			if (!line || line.startsWith("#")) continue
			const [type, blob, ...comment] = line.split(/\s+/)
			if (!type || !blob) continue
			const parsed = utils.parseKey(`${type} ${blob}`)
			if (parsed instanceof Error) continue
			const meta = parseDeviceComment(comment.join(" "))
			records.push({
				fingerprint: fingerprintOf(parsed.getPublicSSH()),
				label: meta.label,
				publicKey: `${type} ${blob}`,
				addedAt: meta.addedAt,
			})
		}
		return records
	}

	isAuthorized(fingerprint: string): boolean {
		return this.devices().some((d) => d.fingerprint === fingerprint)
	}

	/** Mints a device key pair and authorizes its public half. The private half is returned once and never stored. */
	addDevice(label: string): { device: DeviceRecord; privateKey: string } {
		const cleanLabel = label.trim()
		if (!cleanLabel) throw new Error("Device label is required")
		if (cleanLabel.length > 64) throw new Error("Device label must be 64 characters or fewer")
		const addedAt = Date.now()
		const comment = `${DEVICE_COMMENT_PREFIX}${encodeURIComponent(cleanLabel)}:${addedAt}`
		const pair = utils.generateKeyPairSync("ed25519", { comment })
		const fingerprint = fingerprintOfKeyText(pair.public)
		const lines = this.devices().map(deviceLine)
		lines.push(pair.public.trim())
		this.writePrivate("authorized_keys", `${lines.join("\n")}\n`)
		return {
			device: {
				fingerprint,
				label: cleanLabel,
				publicKey: pair.public.trim().split(/\s+/).slice(0, 2).join(" "),
				addedAt,
			},
			privateKey: pair.private,
		}
	}

	removeDevice(fingerprint: string): boolean {
		const before = this.devices()
		const kept = before.filter((d) => d.fingerprint !== fingerprint)
		if (kept.length === before.length) return false
		this.writePrivate("authorized_keys", kept.length ? `${kept.map(deviceLine).join("\n")}\n` : "")
		return true
	}

	knownRelay(): KnownRelay | undefined {
		const path = join(this.dir, "known_relay")
		if (!existsSync(path)) return undefined
		const [relay, fingerprint] = readFileSync(path, "utf8").trim().split(/\s+/)
		return relay && fingerprint ? { relay, fingerprint } : undefined
	}

	rememberRelay(relay: string, fingerprint: string): void {
		this.writePrivate("known_relay", `${relay} ${fingerprint}\n`)
	}

	private loadOrCreate(name: string, comment: string): StoredKey {
		const privatePath = join(this.dir, name)
		if (!existsSync(privatePath)) {
			const pair = utils.generateKeyPairSync("ed25519", { comment })
			this.writePrivate(name, pair.private)
			writeFileSync(`${privatePath}.pub`, `${pair.public.trim()}\n`, { mode: 0o644 })
		}
		const privateKey = readFileSync(privatePath, "utf8")
		const parsed = utils.parseKey(privateKey)
		if (parsed instanceof Error) throw new Error(`${privatePath}: ${parsed.message}`)
		const blob = parsed.getPublicSSH()
		return {
			privateKey,
			publicKey: `${parsed.type} ${blob.toString("base64")} ${comment}`,
			fingerprint: fingerprintOf(blob),
		}
	}

	/** Atomic 0600 write: temp file then rename, so a crash never leaves a half-written key file. */
	private writePrivate(name: string, content: string): void {
		const path = join(this.dir, name)
		const tmp = `${path}.${process.pid}.tmp`
		writeFileSync(tmp, content, { mode: 0o600 })
		renameSync(tmp, path)
	}
}

function deviceLine(device: DeviceRecord): string {
	return `${device.publicKey} ${DEVICE_COMMENT_PREFIX}${encodeURIComponent(device.label)}:${device.addedAt}`
}

function parseDeviceComment(comment: string): { label: string; addedAt: number } {
	if (comment.startsWith(DEVICE_COMMENT_PREFIX)) {
		const rest = comment.slice(DEVICE_COMMENT_PREFIX.length)
		const sep = rest.lastIndexOf(":")
		if (sep > 0) {
			const addedAt = Number(rest.slice(sep + 1))
			try {
				return {
					label: decodeURIComponent(rest.slice(0, sep)),
					addedAt: Number.isFinite(addedAt) ? addedAt : 0,
				}
			} catch {
				/* fall through to the raw comment */
			}
		}
	}
	// A line added by hand: the comment is the label, as OpenSSH treats it.
	return { label: comment || "unnamed device", addedAt: 0 }
}
