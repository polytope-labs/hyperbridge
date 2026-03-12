import { fulfilled, safeArray } from "../data.helper"

describe("safeArray", () => {
	it("should return the array if input is an array", () => {
		const input = [1, 2, 3]
		expect(safeArray(input)).toEqual([1, 2, 3])
	})

	it("should return empty array if input is undefined", () => {
		expect(safeArray(undefined)).toEqual([])
	})

	it("should return empty array if input is null", () => {
		expect(safeArray(null)).toEqual([])
	})
})

describe("fulfilled", () => {
	it("should return only fulfilled values from promise settled results", async () => {
		const results = await Promise.allSettled([
			Promise.resolve(1),
			Promise.reject("error"),
			Promise.resolve("hello"),
		])

		const values = fulfilled(results)

		expect(values).toEqual([1, "hello"])
	})
})
