import { cancel, isCancel, log, password, text } from "@clack/prompts"
import { isAddress } from "viem"

/** Timeout for all wizard/setup network probes (RPC checks, key validation, …). */
export const PROBE_TIMEOUT_MS = 10_000

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

export interface AskTextOptions {
	initial?: string
	placeholder?: string
	/** Reject empty input; a string overrides the error message. Default true. */
	required?: boolean | string
	/** Extra validation, called with the trimmed non-empty value. */
	validate?: (trimmed: string) => string | undefined
}

/** Text prompt returning the trimmed value; empty submissions allowed only with `required: false`. */
export async function askText(message: string, options: AskTextOptions = {}): Promise<string> {
	const required = options.required ?? true
	const value = guard(
		await text({
			message,
			initialValue: options.initial,
			placeholder: options.placeholder,
			...(required ? {} : { defaultValue: "" }),
			validate: (input) => {
				const trimmed = (input ?? "").trim()
				if (!trimmed) {
					if (!required) return undefined
					return typeof required === "string" ? required : "This value is required"
				}
				return options.validate?.(trimmed)
			},
		}),
	)
	return (value ?? "").trim()
}

export async function askNumber(
	message: string,
	initial: number,
	check?: (parsed: number) => string | undefined,
): Promise<number> {
	const value = await askText(message, {
		initial: String(initial),
		validate: (trimmed) => {
			const parsed = Number(trimmed)
			if (!Number.isFinite(parsed)) return "Enter a number"
			return check?.(parsed)
		},
	})
	return Number(value)
}

export async function askAddress(message: string, options: AskTextOptions = {}): Promise<string> {
	return askText(message, {
		...options,
		validate: (trimmed) => (isAddress(trimmed) ? options.validate?.(trimmed) : "Enter a valid EVM address"),
	})
}

export async function askUrl(
	message: string,
	options: AskTextOptions & { protocols?: string[] } = {},
): Promise<string> {
	return askText(message, {
		...options,
		validate: (trimmed) =>
			isValidUrl(trimmed, options.protocols)
				? options.validate?.(trimmed)
				: `Enter a valid ${(options.protocols ?? ["http:", "https:"]).map((p) => p.replace(":", "")).join("/")} URL`,
	})
}

/** Masked input; when a previous value exists, pressing Enter keeps it. */
export async function askSecret(
	message: string,
	previous?: string,
	validate?: (value: string) => string | undefined,
): Promise<string> {
	for (;;) {
		const input = guard(
			await password({
				message: previous ? `${message} (press Enter to keep the current value)` : message,
				validate: (value) => {
					const trimmed = (value ?? "").trim()
					if (!trimmed) return previous ? undefined : "This value is required"
					return validate?.(trimmed)
				},
			}),
		)
		const trimmed = (input ?? "").trim()
		if (trimmed) return trimmed
		if (previous) return previous
	}
}

export function isValidUrl(value: string, protocols: string[] = ["http:", "https:"]): boolean {
	try {
		const parsed = new URL(value)
		return protocols.includes(parsed.protocol)
	} catch {
		return false
	}
}

export function maskSecret(secret: string): string {
	if (secret.length <= 8) return "****"
	return `${secret.slice(0, 4)}…${secret.slice(-4)}`
}

export function withTimeout<T>(promise: Promise<T>, ms: number, label = "operation"): Promise<T> {
	return Promise.race([
		promise,
		new Promise<never>((_, reject) => {
			const timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms)
			timer.unref?.()
		}),
	])
}
