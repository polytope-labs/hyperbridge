/**
 * Normalizes a BigInt timestamp value that might represent seconds or milliseconds.
 * It uses a heuristic based on digit length to guess the unit.
 *
 * - Input timestamps with 11 or fewer digits (representing dates up to year 5138 if seconds)
 *   are assumed to be in SECONDS and are converted to milliseconds by multiplying by 1000n.
 * - Input timestamps with more than 11 digits are assumed to already be in MILLISECONDS
 *   and are returned unchanged.
 *
 * IMPORTANT: This function is specifically designed for BigInt inputs where the unit
 *            (seconds vs milliseconds) is uncertain and precise BigInt arithmetic is desired
 *            for the conversion. Do NOT use this for standard JavaScript Date objects or
 *            the Number result of Date.prototype.getTime(), as those are already reliably
 *            in milliseconds.
 *
 * @param timestamp - A BigInt timestamp, potentially representing seconds or milliseconds.
 * @returns A BigInt timestamp normalized to represent milliseconds.
 *
 * @example
 * normalizeTimestamp(1700000000n) // Assumes seconds => returns 1700000000000n
 * normalizeTimestamp(1700000000000n) // Assumes milliseconds => returns 1700000000000n
 */
export const normalizeTimestamp = (timestamp: bigint): bigint => {
	if (timestamp.toString().length <= 11) {
		return timestamp * 1000n
	}
	return timestamp
}
