;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }
;(global as any).chainId = "84532"

const records = new Map<string, any>()
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => {
		records.set(`${entity}:${id}`, { ...props })
	}),
	getByFields: jest.fn(async (entity: string, _filter: any[], options: any = {}) => {
		const all = [...records.entries()]
			.filter(([key]) => key.startsWith(`${entity}:`))
			.map(([, props]) => props)
			.sort((a, b) => (a.id < b.id ? -1 : 1))
		const offset = options.offset ?? 0
		return all.slice(offset, offset + (options.limit ?? all.length))
	}),
	remove: jest.fn(),
}

import { VolumeService, toScaledUsd } from "@/services/volume.service"

// 2023-11-14T22:13:20Z
const T1 = 1700000000n
const T2 = 1700000060n
const DAY = "2023-11-14"

describe("VolumeService.updateVolume", () => {
	beforeEach(() => records.clear())

	it("creates chain-scoped daily and cumulative records on first update", async () => {
		await VolumeService.updateVolume("IntentGatewayV3.FILLED", "2628.04532", T1)

		const daily = records.get(`DailyVolumeUSD:IntentGatewayV3.FILLED.EVM-84532.${DAY}`)
		expect(daily).toBeDefined()
		expect(daily.last24HoursVolumeUSD).toBe(toScaledUsd("2628.04532"))
		expect(daily.lastUpdatedAt).toBe(T1)

		const cumulative = records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLED.EVM-84532")
		expect(cumulative).toBeDefined()
		expect(cumulative.volumeUSD).toBe(toScaledUsd("2628.04532"))
	})

	it("accumulates into the same daily bucket across updates within a day", async () => {
		await VolumeService.updateVolume("IntentGatewayV3.FILLED", "100", T1)
		await VolumeService.updateVolume("IntentGatewayV3.FILLED", "50.5", T2)

		const daily = records.get(`DailyVolumeUSD:IntentGatewayV3.FILLED.EVM-84532.${DAY}`)
		expect(daily.last24HoursVolumeUSD).toBe(toScaledUsd("150.5"))
		expect(daily.lastUpdatedAt).toBe(T2)

		const cumulative = records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLED.EVM-84532")
		expect(cumulative.volumeUSD).toBe(toScaledUsd("150.5"))
	})

	it("keeps filler-scoped and gateway-scoped records independent", async () => {
		await VolumeService.updateVolume("IntentGatewayV3.FILLER.0xabc", "100", T1)
		await VolumeService.updateVolume("IntentGatewayV3.FILLED", "100", T1)

		expect(records.get(`DailyVolumeUSD:IntentGatewayV3.FILLER.0xabc.EVM-84532.${DAY}`)).toBeDefined()
		expect(records.get(`DailyVolumeUSD:IntentGatewayV3.FILLED.EVM-84532.${DAY}`)).toBeDefined()
	})
})

describe("VolumeService.seedAggregateVolume", () => {
	beforeEach(() => records.clear())

	const plantCumulative = (id: string, volumeUSD: bigint, lastUpdatedAt: bigint) =>
		records.set(`CumulativeVolumeUSD:${id}`, { id, volumeUSD, lastUpdatedAt })
	const plantDaily = (id: string, last24HoursVolumeUSD: bigint, lastUpdatedAt: bigint) =>
		records.set(`DailyVolumeUSD:${id}`, {
			id,
			last24HoursVolumeUSD,
			lastUpdatedAt,
			createdAt: new Date(`${id.slice(-10)}T00:00:00.000Z`),
		})

	const plantFillerHistory = () => {
		plantCumulative("IntentGatewayV3.FILLER.0xaaa.EVM-84532", toScaledUsd("100"), 1000n)
		plantCumulative("IntentGatewayV3.FILLER.0xbbb.EVM-84532", toScaledUsd("50"), 2000n)
		plantCumulative("IntentGatewayV3.FILLER.0xaaa.EVM-97", toScaledUsd("999"), 3000n)
		plantCumulative("Transfer.USDC.EVM-84532", toScaledUsd("777"), 3000n)
		plantDaily("IntentGatewayV3.FILLER.0xaaa.EVM-84532.2023-11-13", toScaledUsd("60"), 1000n)
		plantDaily("IntentGatewayV3.FILLER.0xbbb.EVM-84532.2023-11-13", toScaledUsd("40"), 900n)
		plantDaily(`IntentGatewayV3.FILLER.0xaaa.EVM-84532.${DAY}`, toScaledUsd("40"), 1000n)
		plantDaily("IntentGatewayV3.FILLER.0xaaa.EVM-97.2023-11-13", toScaledUsd("999"), 3000n)
		plantDaily("IntentGatewayV3.USER.EVM-84532.2023-11-13", toScaledUsd("888"), 3000n)
	}

	it("seeds per-day records and a cumulative derived from them, from this chain's filler dailies only", async () => {
		plantFillerHistory()

		await VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")

		// 140 is the daily sum; the planted filler cumulatives sum to 150 and must not be scanned
		const cumulative = records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLED.EVM-84532")
		expect(cumulative.volumeUSD).toBe(toScaledUsd("140"))
		expect(cumulative.lastUpdatedAt).toBe(1000n)

		expect(records.get("DailyVolumeUSD:IntentGatewayV3.FILLED.EVM-84532.2023-11-13").last24HoursVolumeUSD).toBe(
			toScaledUsd("100"),
		)
		expect(records.get(`DailyVolumeUSD:IntentGatewayV3.FILLED.EVM-84532.${DAY}`).last24HoursVolumeUSD).toBe(
			toScaledUsd("40"),
		)
	})

	it("does not swallow the triggering fill: update after seeding still counts, even same-day", async () => {
		plantFillerHistory()

		await VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")
		await VolumeService.updateVolume("IntentGatewayV3.FILLED", "10", T1)

		expect(records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLED.EVM-84532").volumeUSD).toBe(toScaledUsd("150"))
		expect(records.get(`DailyVolumeUSD:IntentGatewayV3.FILLED.EVM-84532.${DAY}`).last24HoursVolumeUSD).toBe(
			toScaledUsd("50"),
		)
	})

	it("is a no-op once the aggregate cumulative record exists", async () => {
		plantFillerHistory()

		await VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")
		plantDaily("IntentGatewayV3.FILLER.0xccc.EVM-84532.2023-11-13", toScaledUsd("1000"), 5000n)
		await VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")

		expect(records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLED.EVM-84532").volumeUSD).toBe(toScaledUsd("140"))
	})

	// Known divergence, pinned deliberately: the cumulative same-timestamp guard fires per record,
	// so the chain-wide FILLED aggregate drops the second of two same-block fills even when they
	// come from different fillers, while the daily series and the per-filler cumulatives count both.
	it("same-block fills by two fillers: FILLED daily counts both, FILLED cumulative drops the second", async () => {
		plantFillerHistory()
		await VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")

		await VolumeService.updateVolume("IntentGatewayV3.FILLER.0xaaa", "10", T1)
		await VolumeService.updateVolume("IntentGatewayV3.FILLED", "10", T1)
		await VolumeService.updateVolume("IntentGatewayV3.FILLER.0xbbb", "20", T1)
		await VolumeService.updateVolume("IntentGatewayV3.FILLED", "20", T1)

		expect(records.get(`DailyVolumeUSD:IntentGatewayV3.FILLED.EVM-84532.${DAY}`).last24HoursVolumeUSD).toBe(
			toScaledUsd("70"),
		)
		expect(records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLER.0xaaa.EVM-84532").volumeUSD).toBe(
			toScaledUsd("110"),
		)
		expect(records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLER.0xbbb.EVM-84532").volumeUSD).toBe(
			toScaledUsd("70"),
		)
		expect(records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLED.EVM-84532").volumeUSD).toBe(toScaledUsd("150"))
	})

	it("creates a zero marker when no filler history exists (fresh reindex)", async () => {
		await VolumeService.seedAggregateVolume("IntentGatewayV3.FILLED", "IntentGatewayV3.FILLER.")

		const cumulative = records.get("CumulativeVolumeUSD:IntentGatewayV3.FILLED.EVM-84532")
		expect(cumulative.volumeUSD).toBe(0n)
		expect([...records.keys()].filter((key) => key.startsWith("DailyVolumeUSD:"))).toHaveLength(0)
	})
})
