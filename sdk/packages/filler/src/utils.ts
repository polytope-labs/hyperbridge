// Returns true if candidate <= reference * (1 + thresholdBps/10000)
export function isWithinThreshold(candidate: bigint, reference: bigint, thresholdBps: bigint): boolean {
	const basisPoints = 10000n
	return candidate * basisPoints <= reference * (basisPoints + thresholdBps)
}

/**
 * Compares two BigInt values that represent the same logical amount but with different decimal precisions
 * @param value1 - First BigInt value
 * @param decimals1 - Decimal places for first value
 * @param value2 - Second BigInt value
 * @param decimals2 - Decimal places for second value
 * @returns true if the values represent the same amount, false otherwise
 */
export function compareDecimalValues(value1: bigint, decimals1: number, value2: bigint, decimals2: number): boolean {
	if (decimals1 === decimals2) {
		return value1 === value2
	}

	const maxDecimals = Math.max(decimals1, decimals2)

	const normalizedValue1 = decimals1 < maxDecimals ? value1 * 10n ** BigInt(maxDecimals - decimals1) : value1

	const normalizedValue2 = decimals2 < maxDecimals ? value2 * 10n ** BigInt(maxDecimals - decimals2) : value2

	return normalizedValue1 === normalizedValue2
}
