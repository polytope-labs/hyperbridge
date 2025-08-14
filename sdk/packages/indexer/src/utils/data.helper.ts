/**
 * safeArray is a utility function that returns an array from an array-like object or an empty array if the input is undefined or null.
 * @param array - The array-like object to convert to an array.
 */
export const safeArray = <T>(array: T[] | undefined | null) => {
	return Array.isArray(array) ? array : []
}
