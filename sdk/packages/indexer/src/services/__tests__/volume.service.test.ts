;(global as any).logger = { debug: jest.fn(), info: jest.fn(), warn: jest.fn(), error: jest.fn() }
;(global as any).chainId = "84532"

const records = new Map<string, any>()
;(global as any).store = {
	get: jest.fn(async (entity: string, id: string) => records.get(`${entity}:${id}`)),
	set: jest.fn(async (entity: string, id: string, props: any) => {
		records.set(`${entity}:${id}`, { ...props })
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
