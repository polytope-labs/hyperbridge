/**
 * safeArray is a utility function that returns an array from an array-like object or an empty array if the input is undefined or null.
 * @param array - The array-like object to convert to an array.
 */
export const safeArray = <T>(array: T[] | undefined | null) => {
	return Array.isArray(array) ? array : []
}

/**
 * A type-safe utility function that unwraps the values from an array of PromiseSettledResults.
 * Supports full type inference.
 *
 * @param results - The array of PromiseSettledResults to unwrap
 * @returns An array of unwrapped values
 */
export const fulfilled = <T extends readonly PromiseSettledResult<any>[]>(
	results: T,
): Array<T[number] extends PromiseSettledResult<infer U> ? U : never> => {
	return results.reduce(
		(acc, result) => {
			if (result.status === "fulfilled") {
				return [...acc, result.value]
			}
			return acc
		},
		[] as Array<T[number] extends PromiseSettledResult<infer U> ? U : never>,
	)
}
