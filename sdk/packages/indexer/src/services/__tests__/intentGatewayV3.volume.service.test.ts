;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }
;(global as any).chainId = "137"

const records = new Map<string, any>()
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => {
		records.set(`${entity}:${id}`, { ...props })
	}),
}

jest.mock("@/services/orderbookRates.service", () => {
	const actual = jest.requireActual("@/services/orderbookRates.service")
	return { ...actual, fetchOrderbookUsdPrice: jest.fn(), fetchTokenUsdPrice: jest.fn(actual.fetchTokenUsdPrice) }
})

import { IntentGatewayV3Service } from "@/services/intentGatewayV3.service"
import { fetchTokenUsdPrice } from "@/services/orderbookRates.service"

const CHAIN = "EVM-137"
const USDT0 = "0xc2132d05d31c914a87c6611c10748aeb04b58e8f"
const CNGN = "0x52828daa48c1a9a06f37500882b42daf0be04c3b"

const SYMBOLS: Record<string, string> = { [USDT0]: "USDT0", [CNGN]: "cNGN" }

const priceMock = jest.mocked(fetchTokenUsdPrice)
const actualPrice = jest.requireActual("@/services/orderbookRates.service").fetchTokenUsdPrice

/** Whole tokens at 6 decimals, which every token here has. */
const units = (whole: number) => BigInt(Math.round(whole * 1e6))
const usd = (whole: number) => BigInt(Math.round(whole * 100)) * 10n ** 16n

const tokenVolume = (token: string, type = "PLACED") =>
	records.get(`IntentGatewayTokenVolume:${CHAIN}-${token}-${type}`)
const cumulativeUsd = (type = "PLACED") =>
	records.get(`CumulativeIntentGatewayVolumeUSD:${CHAIN}-${type}`)?.volumeUSD as bigint | undefined

const existingTokenVolume = (token: string, amount: bigint) =>
	records.set(`IntentGatewayTokenVolume:${CHAIN}-${token}-PLACED`, {
		id: `${CHAIN}-${token}-PLACED`,
		chain: CHAIN,
		tokenAddress: token,
		tokenSymbol: SYMBOLS[token],
		decimals: 6,
		volumeType: "PLACED",
		amount,
		lastUpdatedAt: 1n,
	})

const existingCumulative = (volumeUSD: bigint) =>
	records.set(`CumulativeIntentGatewayVolumeUSD:${CHAIN}-PLACED`, {
		id: `${CHAIN}-PLACED`,
		chain: CHAIN,
		volumeType: "PLACED",
		volumeUSD,
		lastUpdatedAt: 1n,
	})

const place = (token: string, whole: number, timestamp = 2n) =>
	IntentGatewayV3Service.recordOrderVolume("PLACED", [{ token, amount: units(whole) }], timestamp)

beforeEach(() => {
	records.clear()
	jest.clearAllMocks()
	priceMock.mockImplementation(actualPrice)
	jest.spyOn(IntentGatewayV3Service as any, "getTokenMetadata").mockImplementation(async (token: any) => ({
		symbol: SYMBOLS[token],
		decimals: 6,
	}))
})

describe("IntentGatewayV3Service.recordOrderVolume", () => {
	it("counts a USDT0 order at $1", async () => {
		await place(USDT0, 100)

		expect(tokenVolume(USDT0)).toMatchObject({ tokenSymbol: "USDT0", amount: units(100) })
		expect(cumulativeUsd()).toBe(usd(100))
	})

	it("adds only the order's own amount to an existing USDT0 row's USD rollup", async () => {
		existingTokenVolume(USDT0, units(249_301.98))
		existingCumulative(usd(97_841.76))

		await place(USDT0, 10)

		expect(tokenVolume(USDT0)).toMatchObject({ amount: units(249_311.98) })
		expect(cumulativeUsd()).toBe(usd(97_851.76))
	})

	it("keeps the raw amount of a token with no price and leaves it out of the USD rollup", async () => {
		priceMock.mockResolvedValueOnce(null)

		await place(CNGN, 3_000)

		expect(tokenVolume(CNGN)).toMatchObject({ amount: units(3_000) })
		expect(cumulativeUsd()).toBeUndefined()
	})
})
