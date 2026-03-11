import { normalizeTimestamp } from "@/utils/date.helpers"

describe("normalizeTimestamp", () => {
	test("should convert a typical 10-digit seconds timestamp to milliseconds", () => {
		const inputSeconds = 1713787200n
		const expectedMilliseconds = 1713787200000n
		expect(normalizeTimestamp(inputSeconds)).toBe(expectedMilliseconds)
	})

	test("should convert an 11-digit seconds timestamp (max seconds range) to milliseconds", () => {
		const inputSeconds = 99999999999n
		const expectedMilliseconds = 99999999999000n
		expect(normalizeTimestamp(inputSeconds)).toBe(expectedMilliseconds)
	})

	test("should convert a small seconds timestamp to milliseconds", () => {
		const inputSeconds = 123n
		const expectedMilliseconds = 123000n
		expect(normalizeTimestamp(inputSeconds)).toBe(expectedMilliseconds)
	})

	test("should handle zero timestamp correctly (treated as seconds)", () => {
		const inputZero = 0n
		const expectedZero = 0n
		expect(normalizeTimestamp(inputZero)).toBe(expectedZero)
	})

	test("should return a typical 13-digit milliseconds timestamp unchanged", () => {
		const inputMilliseconds = 1713787200000n
		const expectedMilliseconds = 1713787200000n
		expect(normalizeTimestamp(inputMilliseconds)).toBe(expectedMilliseconds)
	})

	test("should return a 12-digit timestamp unchanged (treated as milliseconds)", () => {
		const inputMilliseconds = 100000000000n
		const expectedMilliseconds = 100000000000n
		expect(normalizeTimestamp(inputMilliseconds)).toBe(expectedMilliseconds)
	})

	test("should return a very large (e.g., 15-digit) milliseconds timestamp unchanged", () => {
		const inputMilliseconds = 171378720012345n
		const expectedMilliseconds = 171378720012345n
		expect(normalizeTimestamp(inputMilliseconds)).toBe(expectedMilliseconds)
	})
})
