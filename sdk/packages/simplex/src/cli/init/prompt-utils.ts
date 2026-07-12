import { cancel, isCancel, log } from "@clack/prompts"

/** Unwraps a clack prompt result, exiting cleanly when the user cancels (ctrl-c / escape). */
export function guard<T>(value: T | symbol): T {
	if (isCancel(value)) {
		cancel("Setup aborted — nothing was written.")
		process.exit(0)
	}
	return value as T
}

/** One-line explanation shown before a prompt: what the value is for and why it's needed. */
export function why(text: string): void {
	log.message(text)
}

export function isValidUrl(value: string, protocols: string[] = ["http:", "https:"]): boolean {
	try {
		const parsed = new URL(value)
		return protocols.includes(parsed.protocol)
	} catch {
		return false
	}
}

/** Parses "amount,value" pair lines used by the curve/point editors. */
export function parsePointInput(input: string): { first: string; second: string } | null {
	const parts = input.split(",").map((part) => part.trim())
	if (parts.length !== 2 || !parts[0] || !parts[1]) return null
	if (!Number.isFinite(Number(parts[0])) || !Number.isFinite(Number(parts[1]))) return null
	return { first: parts[0], second: parts[1] }
}

export function maskSecret(secret: string): string {
	if (secret.length <= 8) return "****"
	return `${secret.slice(0, 4)}…${secret.slice(-4)}`
}
