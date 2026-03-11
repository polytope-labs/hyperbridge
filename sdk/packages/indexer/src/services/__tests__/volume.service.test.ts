import Decimal from "decimal.js"

import { CumulativeVolumeUSD, DailyVolumeUSD } from "@/configs/src/types"
import { timestampToDate } from "@/utils/date.helpers"

import { VolumeService } from "../volume.service"

const mockStore = new Map<string, any>()

// Mock the global store
;(global as any).store = {
	get: jest.fn().mockImplementation((entityName: string, id: string) => {
		const key = `${entityName}:${id}`
		return Promise.resolve(mockStore.get(key))
	}),
	set: jest.fn().mockImplementation((entityName: string, id: string, entity: any) => {
		const key = `${entityName}:${id}`
		mockStore.set(key, entity)
		return Promise.resolve()
	}),
	remove: jest.fn().mockImplementation((entityName: string, id: string) => {
		const key = `${entityName}:${id}`
		mockStore.delete(key)
		return Promise.resolve()
	}),
}

// Mock the model classes
jest.mock("@/configs/src/types", () => ({
	CumulativeVolumeUSD: {
		get: jest.fn().mockImplementation((id: string) => {
			const key = `CumulativeVolumeUSD:${id}`
			const data = mockStore.get(key)
			if (data) {
				return Promise.resolve({
					id: data.id,
					volumeUSD: data.volumeUSD,
					lastUpdatedAt: data.lastUpdatedAt,
					save: jest.fn().mockImplementation(function (this: any) {
						const key = `CumulativeVolumeUSD:${this.id}`
						mockStore.set(key, this)
						return Promise.resolve()
					}),
				})
			}
			return Promise.resolve(undefined)
		}),
		create: jest.fn().mockImplementation((data: any) => {
			const entity = {
				id: data.id,
				volumeUSD: data.volumeUSD,
				lastUpdatedAt: data.lastUpdatedAt,
				save: jest.fn().mockImplementation(function (this: any) {
					const key = `CumulativeVolumeUSD:${this.id}`
					mockStore.set(key, this)
					return Promise.resolve()
				}),
			}
			return entity
		}),
	},
	DailyVolumeUSD: {
		get: jest.fn().mockImplementation((id: string) => {
			const key = `DailyVolumeUSD:${id}`
			const data = mockStore.get(key)
			if (data) {
				return Promise.resolve({
					id: data.id,
					last24HoursVolumeUSD: data.last24HoursVolumeUSD,
					lastUpdatedAt: data.lastUpdatedAt,
					createdAt: data.createdAt,
					save: jest.fn().mockImplementation(function (this: any) {
						const key = `DailyVolumeUSD:${this.id}`
						mockStore.set(key, this)
						return Promise.resolve()
					}),
				})
			}
			return Promise.resolve(undefined)
		}),
		create: jest.fn().mockImplementation((data: any) => {
			const entity = {
				id: data.id,
				last24HoursVolumeUSD: data.last24HoursVolumeUSD,
				lastUpdatedAt: data.lastUpdatedAt,
				createdAt: data.createdAt,
				save: jest.fn().mockImplementation(function (this: any) {
					const key = `DailyVolumeUSD:${this.id}`
					mockStore.set(key, this)
					return Promise.resolve()
				}),
			}
			return entity
		}),
	},
}))

describe("VolumeService", () => {
	beforeAll(() => {
		;(global as any).chainId = "97"
	})

	beforeEach(() => {
		// Clear the mock store before each test
		mockStore.clear()
		jest.clearAllMocks()
	})

	afterAll(() => {
		mockStore.clear()
		jest.clearAllMocks()
	})

	describe("updateCumulativeVolume", () => {
		it("should create a new cumulative volume record when none exists", async () => {
			const id = "TokenGateway"
			const volume = "1000.50"
			const timestamp = BigInt(1700000000)

			await VolumeService.updateCumulativeVolume(id, volume, timestamp)

			expect(CumulativeVolumeUSD.get).toHaveBeenCalledWith(VolumeService.getChainTypeId(id))
			expect(CumulativeVolumeUSD.create).toHaveBeenCalledWith({
				id: VolumeService.getChainTypeId(id),
				volumeUSD: new Decimal(volume).toFixed(18),
				lastUpdatedAt: timestamp,
			})
		})

		it("should update existing cumulative volume record", async () => {
			const id = "TokenGateway"
			const volume = "500.25"
			const additionalVolume = "200.75"
			const timestamp = BigInt(1700000000)
			const updatedTimestamp = BigInt(1700000100)

			await VolumeService.updateCumulativeVolume(id, volume, timestamp)
			await VolumeService.updateCumulativeVolume(id, additionalVolume, updatedTimestamp)

			const stored = mockStore.get(`CumulativeVolumeUSD:${VolumeService.getChainTypeId(id)}`)
			expect(stored).toBeDefined()
			expect(stored.volumeUSD).toBe("701.000000000000000000")
			expect(stored.lastUpdatedAt).toBe(updatedTimestamp)
		})

		it("should handle decimal precision correctly", async () => {
			const id = "TokenGateway"
			await VolumeService.updateCumulativeVolume(id, "0.123456789012345678", BigInt(1700000000))

			const stored = mockStore.get(`CumulativeVolumeUSD:${VolumeService.getChainTypeId(id)}`)
			expect(stored).toBeDefined()
			expect(stored.volumeUSD).toBe("0.123456789012345678")
		})
	})

	describe("updateDailyVolume", () => {
		it("should create a new daily volume record when none exists", async () => {
			const id = "TokenGateway"
			const volume = "1000.50"
			const timestamp = BigInt(1752145126274)

			await VolumeService.updateDailyVolume(id, volume, timestamp)

			const expectedId = `${VolumeService.getChainTypeId(id)}.2025-07-10`
			expect(DailyVolumeUSD.get).toHaveBeenCalledWith(expectedId)
			expect(DailyVolumeUSD.create).toHaveBeenCalledWith({
				id: expectedId,
				last24HoursVolumeUSD: new Decimal(volume).toFixed(18),
				lastUpdatedAt: timestamp,
				createdAt: timestampToDate(timestamp),
			})
		})

		it("should update existing daily volume record within 24 hours", async () => {
			const id = "TokenGateway"
			const volume = "500.25"
			const additionalVolume = "200.75"
			const timestamp = BigInt(1752145126274)
			const updatedTimestamp = timestamp + BigInt(3600)

			await VolumeService.updateDailyVolume(id, volume, timestamp)
			await VolumeService.updateDailyVolume(id, additionalVolume, updatedTimestamp)

			const stored = mockStore.get(`DailyVolumeUSD:${VolumeService.getChainTypeId(id)}.2025-07-10`)
			expect(stored).toBeDefined()
			expect(stored.last24HoursVolumeUSD).toBe(new Decimal(volume).plus(additionalVolume).toFixed(18))
			expect(stored.lastUpdatedAt).toBe(updatedTimestamp)
		})

		it("should create new daily volume record after 24 hours", async () => {
			const id = "TokenGateway"
			const volume = "5000.25"
			const additionalVolume = "2100.75"
			const firstDayTimestamp = BigInt(1752495664445)
			const secondDayTimestamp = firstDayTimestamp + BigInt(60 * 60 * 25 * 1000) // 25 hours later

			await VolumeService.updateDailyVolume(id, volume, firstDayTimestamp)
			await VolumeService.updateDailyVolume(id, additionalVolume, secondDayTimestamp)

			const firstDayStored = mockStore.get(`DailyVolumeUSD:${VolumeService.getChainTypeId(id)}.2025-07-14`)
			expect(firstDayStored).toBeDefined()
			expect(firstDayStored.last24HoursVolumeUSD).toBe(new Decimal(volume).toFixed(18))

			const secondDayStored = mockStore.get(`DailyVolumeUSD:${VolumeService.getChainTypeId(id)}.2025-07-15`)
			expect(secondDayStored).toBeDefined()
			expect(secondDayStored.last24HoursVolumeUSD).toBe(new Decimal(additionalVolume).toFixed(18))
		})

		it("should generate correct daily record ID based on timestamp", async () => {
			const id = "TokenGateway"
			await VolumeService.updateDailyVolume(id, "1000.50", BigInt(1752145126274))
			expect(DailyVolumeUSD.get).toHaveBeenCalledWith(`${VolumeService.getChainTypeId(id)}.2025-07-10`)
		})
	})

	describe("updateVolume", () => {
		it("should update both cumulative and daily volume", async () => {
			const id = "TokenGateway"
			const volume = "1000.50"
			const timestamp = BigInt(1752497885694)

			await VolumeService.updateVolume(id, volume, timestamp)

			expect(CumulativeVolumeUSD.get).toHaveBeenCalledWith(VolumeService.getChainTypeId(id))
			expect(CumulativeVolumeUSD.create).toHaveBeenCalledWith({
				id: VolumeService.getChainTypeId(id),
				volumeUSD: new Decimal(volume).toFixed(18),
				lastUpdatedAt: timestamp,
			})

			const expectedDailyId = `${VolumeService.getChainTypeId(id)}.2025-07-14`
			expect(DailyVolumeUSD.get).toHaveBeenCalledWith(expectedDailyId)
			expect(DailyVolumeUSD.create).toHaveBeenCalledWith({
				id: expectedDailyId,
				last24HoursVolumeUSD: new Decimal(volume).toFixed(18),
				lastUpdatedAt: timestamp,
				createdAt: timestampToDate(timestamp),
			})
		})

		it("should handle parallel updates correctly", async () => {
			const id = "TokenGateway"
			const volume = "1000.50"
			const timestamp = BigInt(1700000000)

			const promises = [
				VolumeService.updateVolume(id, volume, timestamp),
				VolumeService.updateVolume(id, volume, timestamp),
				VolumeService.updateVolume(id, volume, timestamp),
			]

			await Promise.all(promises)

			expect(CumulativeVolumeUSD.get).toHaveBeenCalledTimes(3)
			expect(DailyVolumeUSD.get).toHaveBeenCalledTimes(3)
		})
	})

	describe("edge cases", () => {
		it("should handle very large volume amounts", async () => {
			const id = "TokenGateway"
			const volume = "999999999999999999.999999999999999999"
			const timestamp = BigInt(1700000000)

			await VolumeService.updateCumulativeVolume(id, volume, timestamp)
			const stored = mockStore.get(`CumulativeVolumeUSD:${VolumeService.getChainTypeId(id)}`)

			expect(stored).toBeDefined()
			expect(stored.volumeUSD).toBe(new Decimal(volume).toFixed(18))
		})

		it("should handle zero volume amounts", async () => {
			const id = "TokenGateway"
			const volume = "0"
			const timestamp = BigInt(1700000000)

			await VolumeService.updateCumulativeVolume(id, volume, timestamp)
			const stored = mockStore.get(`CumulativeVolumeUSD:${VolumeService.getChainTypeId(id)}`)

			expect(stored).toBeDefined()
			expect(stored.volumeUSD).toBe("0.000000000000000000")
		})

		it("should handle very small volume amounts", async () => {
			const id = "TokenGateway"
			const volume = "0.000000000000000001"

			await VolumeService.updateCumulativeVolume(id, volume, BigInt(1700000000))
			const stored = mockStore.get(`CumulativeVolumeUSD:${VolumeService.getChainTypeId(id)}`)

			expect(stored).toBeDefined()
			expect(stored.volumeUSD).toBe(new Decimal(volume).toFixed(18))
		})

		it("should handle different ID formats", async () => {
			const ids = ["TokenGateway", "IntentGateway.USER", "IntentGateway.FILLER"]
			const volume = "100.50"
			const timestamp = BigInt(1700000000)

			for (const id of ids) {
				await VolumeService.updateCumulativeVolume(id, volume, BigInt(1700000000))
				const stored = mockStore.get(`CumulativeVolumeUSD:${VolumeService.getChainTypeId(id)}`)

				expect(stored).toBeDefined()
				expect(stored.volumeUSD).toBe(new Decimal(volume).toFixed(18))
			}
		})
	})
})
