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

/**
 * Creates a Date object from a timestamp (either number or bigint).
 * Ensures the timestamp is in milliseconds before creating the Date object.
 *
 * @param timestamp - A number or BigInt timestamp that might be in seconds or milliseconds
 * @returns A Date object representing the timestamp
 *
 * @example
 * timestampToDate(1700000000) // Converts seconds to ms, returns Date object
 * timestampToDate(1700000000000) // Already in ms, returns Date object
 * timestampToDate(1700000000n) // Converts BigInt seconds to ms, returns Date object
 */
export const timestampToDate = (timestamp: number | bigint): Date => {
	if (typeof timestamp === "bigint") {
		const normalizedTimestamp = normalizeTimestamp(timestamp)
		return new Date(Number(normalizedTimestamp))
	}

	const normalizedTimestamp = normalizeTimestamp(BigInt(timestamp))
	return new Date(Number(normalizedTimestamp))
}
