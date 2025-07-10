import PriceHelper from "../price.helpers"

describe.skip("PriceHelper.getTokenPriceInUSDCoingecko", () => {
	beforeAll(() => {
		;(global as any).logger = console
	})

	it("should return correct price and amount value for CERE token", async () => {
		const result = await PriceHelper.getTokenPriceInUSDCoingecko("CERE", BigInt("50000000000"), 10)

		expect(result).toHaveProperty("priceInUSD")
		expect(result).toHaveProperty("amountValueInUSD")

		expect(parseFloat(result.priceInUSD)).toBeTruthy()
		expect(parseFloat(result.amountValueInUSD)).toBeTruthy()
	})

	it("should return correct price and amount value for DOT token", async () => {
		const result = await PriceHelper.getTokenPriceInUSDCoingecko("DOT", BigInt("100000000000"), 10)

		expect(result).toHaveProperty("priceInUSD")
		expect(result).toHaveProperty("amountValueInUSD")

		expect(parseFloat(result.priceInUSD)).toBeTruthy()
		expect(parseFloat(result.amountValueInUSD)).toBeTruthy()
	})

	it("should return correct price and amount value for ETH token", async () => {
		const result = await PriceHelper.getTokenPriceInUSDCoingecko("eth", BigInt("1000000000000000000"), 18)

		expect(result).toHaveProperty("priceInUSD")
		expect(result).toHaveProperty("amountValueInUSD")

		expect(parseFloat(result.priceInUSD)).toBeGreaterThan(1000)
		expect(parseFloat(result.amountValueInUSD)).toBeGreaterThan(1000)
	})

	it("should return correct price and amount value for xDai", async () => {
		const result = await PriceHelper.getTokenPriceInUSDCoingecko("xdai", BigInt("1000000000000000000"), 18)

		expect(result).toHaveProperty("priceInUSD")
		expect(result).toHaveProperty("amountValueInUSD")
	})

	it("should return zero values when symbol empty string", async () => {
		const result = await PriceHelper.getTokenPriceInUSDCoingecko("", BigInt("200000000000"), 18)
		expect(result).toEqual({ priceInUSD: "0", amountValueInUSD: "0" })
	})

	it("should handle invalid symbols properly", async () => {
		const result = await PriceHelper.getTokenPriceInUSDCoingecko("+++", BigInt("100"), 18)
		expect(result).toEqual({ priceInUSD: "0", amountValueInUSD: "0" })
	})
})
