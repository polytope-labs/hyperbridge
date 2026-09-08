import { createHash } from "node:crypto"
import { existsSync, mkdirSync, readFileSync, renameSync, writeFileSync } from "node:fs"
import { join } from "node:path"
import ssh2 from "ssh2"

// ssh2 is CommonJS: named imports resolve under vitest's transform but not in
// the ESM binary, where Node cannot see them statically.
const { utils } = ssh2

/** How many times a rejected generated pair is retried before giving up. */
const KEYGEN_ATTEMPTS = 8

/**
 * Generates an ed25519 pair that ssh2 can read back.
 *
 * ssh2's generator emits a key its own parser refuses roughly once in 256 —
 * measured at 0.4–0.6% over thousands of pairs, which is the signature of a
 * dropped leading zero byte. For a key that is written to disk and loaded on
 * every boot that is not a transient error: remote access stays broken until
 * someone deletes the file. So every pair is parsed before it is used, and a
 * bad one is thrown away rather than stored.
 */
export function generateKeyPair(comment?: string): { public: string; private: string } {
	for (let attempt = 0; attempt < KEYGEN_ATTEMPTS; attempt++) {
		const pair = comment === undefined
			? utils.generateKeyPairSync("ed25519")
			: utils.generateKeyPairSync("ed25519", { comment })
		if (utils.parseKey(pair.private) instanceof Error) continue
		if (utils.parseKey(pair.public) instanceof Error) continue
		return pair
	}
	throw new Error(`Could not generate a usable ed25519 key pair in ${KEYGEN_ATTEMPTS} attempts`)
}

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

	/**
	 * Authorizes a device. With `publicKey` (the phone generated its own pair and
	 * pasted the `.pub` line) nothing secret is ever seen here. Without it a key
	 * pair is minted and the private half returned once, never stored.
	 */
	addDevice(label: string, publicKey?: string): { device: DeviceRecord; privateKey?: string } {
		const cleanLabel = label.trim()
		if (!cleanLabel) throw new Error("Device label is required")
		if (cleanLabel.length > 64) throw new Error("Device label must be 64 characters or fewer")
		const addedAt = Date.now()
		const comment = `${DEVICE_COMMENT_PREFIX}${encodeURIComponent(cleanLabel)}:${addedAt}`
		let publicLine: string
		let privateKey: string | undefined
		if (publicKey !== undefined) {
			publicLine = normalizePublicKey(publicKey)
		} else {
			const pair = generateKeyPair(comment)
			publicLine = pair.public.trim().split(/\s+/).slice(0, 2).join(" ")
			privateKey = pair.private
		}
		const fingerprint = fingerprintOfKeyText(publicLine)
		if (this.isAuthorized(fingerprint)) throw new Error(`That key is already paired (${fingerprint})`)
		const lines = this.devices().map(deviceLine)
		lines.push(`${publicLine} ${comment}`)
		this.writePrivate("authorized_keys", `${lines.join("\n")}\n`)
		return { device: { fingerprint, label: cleanLabel, publicKey: publicLine, addedAt }, privateKey }
	}

	removeDevice(fingerprint: string): boolean {
		const before = this.devices()
		const kept = before.filter((d) => d.fingerprint !== fingerprint)
		if (kept.length === before.length) return false
		this.writePrivate("authorized_keys", kept.length ? `${kept.map(deviceLine).join("\n")}\n` : "")
		return true
	}

	/**
	 * The fingerprint pinned for one relay address. Keyed by address rather than
	 * held as a single line: an operator who moves from relay A to B and back
	 * would otherwise return to A with no pin at all and trust whatever key it
	 * presents next — the pin has to survive the trip.
	 */
	knownRelay(relay: string): KnownRelay | undefined {
		for (const line of this.relayLines()) {
			const [storedRelay, fingerprint] = line.split(/\s+/)
			if (storedRelay === relay && fingerprint) return { relay: storedRelay, fingerprint }
		}
		return undefined
	}

	rememberRelay(relay: string, fingerprint: string): void {
		const kept = this.relayLines().filter((line) => line.split(/\s+/)[0] !== relay)
		kept.push(`${relay} ${fingerprint}`)
		this.writePrivate("known_relay", `${kept.join("\n")}\n`)
	}

	/** Non-empty lines of `known_relay`; a pre-per-relay file is one such line. */
	private relayLines(): string[] {
		const path = join(this.dir, "known_relay")
		if (!existsSync(path)) return []
		return readFileSync(path, "utf8")
			.split("\n")
			.map((line) => line.trim())
			.filter(Boolean)
	}

	private loadOrCreate(name: string, comment: string): StoredKey {
		const privatePath = join(this.dir, name)
		if (!existsSync(privatePath)) {
			const pair = generateKeyPair(comment)
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

/**
 * Validates a pasted public key and returns it as `<type> <base64>`, dropping
 * whatever comment the phone's app attached. Refuses private keys outright:
 * the whole point of pasting is that the private half stays on the phone.
 */
export function normalizePublicKey(text: string): string {
	const trimmed = text.trim()
	if (!trimmed) throw new Error("Paste the device's public key")
	if (/PRIVATE KEY/i.test(trimmed)) {
		throw new Error(
			"That is a private key. Paste the public key instead (the .pub line starting with ssh-ed25519, ecdsa-sha2-… or ssh-rsa)",
		)
	}
	const parsed = utils.parseKey(trimmed)
	if (parsed instanceof Error) throw new Error(`Not a valid SSH public key: ${parsed.message}`)
	if (parsed.isPrivateKey()) throw new Error("That is a private key. Paste the public key instead")
	return `${parsed.type} ${parsed.getPublicSSH().toString("base64")}`
}
