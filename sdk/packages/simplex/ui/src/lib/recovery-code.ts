export const RECOVERY_CODE_DIGITS = 64

/** Recovery codes are 64 hex digits shown in groups of eight; accept pasted codes in any case or spacing. */
export function formatRecoveryCode(value: string): string {
	// Prefer a complete code inside pasted text so letters from a label like "Recovery code:" are ignored.
	const code = value.match(/[0-9a-f]{8}(?:[\s-]*[0-9a-f]{8}){7}/i)?.[0] ?? value
	const digits = code
		.replace(/[^0-9a-f]/gi, "")
		.slice(0, RECOVERY_CODE_DIGITS)
		.toUpperCase()
	return digits.match(/.{1,8}/g)?.join("-") ?? ""
}
