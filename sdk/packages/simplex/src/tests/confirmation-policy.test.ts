import { describe, it, expect } from "vitest"
import { ConfirmationPolicy } from "@/config/interpolated-curve"
import Decimal from "decimal.js"

describe("ConfirmationPolicy", () => {
	describe("constructor validation", () => {
		it("should throw error if less than 2 points provided", () => {
			expect(
				() =>
					new ConfirmationPolicy({
						"1": {
							points: [{ amount: "100", value: 1 }],
						},
					}),
			).toThrow("must have at least 2 points")
		})

		it("should throw error if points array is empty", () => {
			expect(
				() =>
					new ConfirmationPolicy({
						"1": {
							points: [],
						},
					}),
			).toThrow("must have at least 2 points")
		})

		it("should throw error for negative amount", () => {
			expect(
				() =>
					new ConfirmationPolicy({
						"1": {
							points: [
								{ amount: "-100", value: 1 },
								{ amount: "1000", value: 5 },
							],
						},
					}),
			).toThrow("invalid amount")
		})

		it("should throw error for negative value", () => {
			expect(
				() =>
					new ConfirmationPolicy({
						"1": {
							points: [
								{ amount: "100", value: -1 },
								{ amount: "1000", value: 5 },
							],
						},
					}),
			).toThrow("value must be a non-negative integer")
		})

		it("should throw error for non-integer value", () => {
			expect(
				() =>
					new ConfirmationPolicy({
						"1": {
							points: [
								{ amount: "100", value: 1.5 },
								{ amount: "1000", value: 5 },
							],
						},
					}),
			).toThrow("value must be a non-negative integer")
		})

		it("should accept valid configuration", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 5 },
					],
				},
			})
			expect(policy).toBeDefined()
		})

		it("should sort points by amount automatically", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "1000", value: 5 },
						{ amount: "100", value: 1 },
						{ amount: "500", value: 3 },
					],
				},
			})
			// Verify by checking boundary behavior
			expect(policy.getConfirmationBlocks(1, new Decimal(50))).toBe(1) // Below min returns first point's value
			expect(policy.getConfirmationBlocks(1, new Decimal(2000))).toBe(5) // Above max returns last point's value
		})
	})

	describe("getConfirmationBlocks", () => {
		it("should throw error for unknown chainId", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 5 },
					],
				},
			})
			expect(() => policy.getConfirmationBlocks(999, new Decimal(500))).toThrow(
				"No confirmation policy found for chainId 999",
			)
		})

		it("should return first point value for amounts below minimum", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 2 },
						{ amount: "1000", value: 10 },
					],
				},
			})
			expect(policy.getConfirmationBlocks(1, new Decimal(50))).toBe(2)
			expect(policy.getConfirmationBlocks(1, new Decimal(0))).toBe(2)
			expect(policy.getConfirmationBlocks(1, new Decimal(100))).toBe(2) // At minimum
		})

		it("should return last point value for amounts above maximum", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 2 },
						{ amount: "1000", value: 10 },
					],
				},
			})
			expect(policy.getConfirmationBlocks(1, new Decimal(1000))).toBe(10) // At maximum
			expect(policy.getConfirmationBlocks(1, new Decimal(5000))).toBe(10)
			expect(policy.getConfirmationBlocks(1, new Decimal(1000000))).toBe(10)
		})

		it("should return exact value at defined points", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 3 },
						{ amount: "10000", value: 6 },
						{ amount: "50000", value: 12 },
					],
				},
			})
			// At exact points, linear interpolation should return exact values
			expect(policy.getConfirmationBlocks(1, new Decimal(100))).toBe(1)
			expect(policy.getConfirmationBlocks(1, new Decimal(1000))).toBe(3)
			expect(policy.getConfirmationBlocks(1, new Decimal(10000))).toBe(6)
			expect(policy.getConfirmationBlocks(1, new Decimal(50000))).toBe(12)
		})

		it("should interpolate values between points using polynomial", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 3 },
						{ amount: "10000", value: 6 },
						{ amount: "50000", value: 12 },
					],
				},
			})
			// Values between points should be interpolated
			const midValue = policy.getConfirmationBlocks(1, new Decimal(5000))
			expect(midValue).toBeGreaterThanOrEqual(1)
			expect(midValue).toBeLessThanOrEqual(12)
		})

		it("should clamp results to min/max confirmation range", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 2 },
						{ amount: "500", value: 8 },
						{ amount: "1000", value: 4 },
					],
				},
			})
			// Even with polynomial oscillation, results should be clamped between 2 and 8
			for (let amount = 100; amount <= 1000; amount += 50) {
				const result = policy.getConfirmationBlocks(1, new Decimal(amount))
				expect(result).toBeGreaterThanOrEqual(2)
				expect(result).toBeLessThanOrEqual(8)
			}
		})
	})

	describe("multiple chains", () => {
		it("should handle different policies for different chains", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 5 },
					],
				},
				"56": {
					points: [
						{ amount: "100", value: 2 },
						{ amount: "1000", value: 10 },
					],
				},
				"137": {
					points: [
						{ amount: "100", value: 3 },
						{ amount: "1000", value: 15 },
					],
				},
			})

			// Same amount, different chains, different results
			expect(policy.getConfirmationBlocks(1, new Decimal(100))).toBe(1)
			expect(policy.getConfirmationBlocks(56, new Decimal(100))).toBe(2)
			expect(policy.getConfirmationBlocks(137, new Decimal(100))).toBe(3)
		})
	})

	describe("polynomial interpolation behavior", () => {
		it("should produce smooth curve through all points", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 2 },
						{ amount: "5000", value: 4 },
						{ amount: "10000", value: 10 },
					],
				},
			})

			// Test at exact defined points
			expect(policy.getConfirmationBlocks(1, new Decimal(100))).toBe(1)
			expect(policy.getConfirmationBlocks(1, new Decimal(10000))).toBe(10)

			// Collect values at regular intervals and verify they stay within bounds
			const values: number[] = []
			for (let amount = 100; amount <= 10000; amount += 500) {
				const conf = policy.getConfirmationBlocks(1, new Decimal(amount))
				values.push(conf)
				expect(conf).toBeGreaterThanOrEqual(1)
				expect(conf).toBeLessThanOrEqual(10)
			}

			// First value at 100 should be 1
			expect(values[0]).toBe(1)
		})

		it("should handle two-point curve (effectively linear)", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 10 },
					],
				},
			})

			// At midpoint (550): (550-100)/(1000-100) = 0.5, result = 1 + 0.5*9 = 5.5 -> rounds to 6
			const midpoint = policy.getConfirmationBlocks(1, new Decimal(550))
			expect(midpoint).toBe(6)

			// At 25% (325): (325-100)/(1000-100) = 0.25, result = 1 + 0.25*9 = 3.25 -> rounds to 3
			const quarterPoint = policy.getConfirmationBlocks(1, new Decimal(325))
			expect(quarterPoint).toBe(3)

			// At 75% (775): (775-100)/(1000-100) = 0.75, result = 1 + 0.75*9 = 7.75 -> rounds to 8
			const threeQuarterPoint = policy.getConfirmationBlocks(1, new Decimal(775))
			expect(threeQuarterPoint).toBe(8)
		})
	})

	describe("edge cases", () => {
		it("should handle very large amounts", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000000", value: 20 },
					],
				},
			})
			expect(policy.getConfirmationBlocks(1, new Decimal(1e12))).toBe(20)
		})

		it("should handle very small amounts", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "0.01", value: 1 },
						{ amount: "100", value: 10 },
					],
				},
			})
			expect(policy.getConfirmationBlocks(1, new Decimal(0.001))).toBe(1)
		})

		it("should handle zero value", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 0 },
						{ amount: "1000", value: 5 },
					],
				},
			})
			expect(policy.getConfirmationBlocks(1, new Decimal(100))).toBe(0)
		})

		it("should handle Decimal inputs correctly", () => {
			const policy = new ConfirmationPolicy({
				"1": {
					points: [
						{ amount: "100", value: 1 },
						{ amount: "1000", value: 10 },
					],
				},
			})

			// Test with various Decimal constructions
			expect(policy.getConfirmationBlocks(1, new Decimal("550.5"))).toBeDefined()
			expect(policy.getConfirmationBlocks(1, new Decimal(550.5))).toBeDefined()
			expect(policy.getConfirmationBlocks(1, Decimal.add(500, 50))).toBeDefined()
		})
	})
})
