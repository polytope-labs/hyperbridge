import Decimal from "decimal.js"

import { CumulativeVolumeUSD, DailyVolumeUSD } from "@/configs/src/types"
import { getDateFormatFromTimestamp, timestampToDate } from "@/utils/date.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"

/** Decimal places used for the BigInt fixed-point representation of USD volumes. */
const USD_SCALE = 18
const USD_SCALE_FACTOR = new Decimal(10).pow(USD_SCALE)

/** Convert a decimal-string USD value (e.g. "1.5") to a scaled BigInt (e.g. 1_500_000_000_000_000_000n). */
export function toScaledUsd(volumeUSD: string): bigint {
	return BigInt(new Decimal(volumeUSD).mul(USD_SCALE_FACTOR).toFixed(0))
}

export class VolumeService {
	/**
	 * Update cumulative volume for a given identifier
	 * @param baseId - The identifier for the cumulative volume record
	 * @param volumeUSD - The volume in USD to add (decimal string)
	 * @param timestamp - The timestamp of the transaction
	 */
	static async updateCumulativeVolume(baseId: string, volumeUSD: string, timestamp: bigint): Promise<void> {
		const id = this.getChainTypeId(baseId)
		const scaled = toScaledUsd(volumeUSD)
		let cumulativeVolumeUSD = await CumulativeVolumeUSD.get(id)

		if (!cumulativeVolumeUSD) {
			cumulativeVolumeUSD = CumulativeVolumeUSD.create({
				id,
				volumeUSD: scaled,
				lastUpdatedAt: timestamp,
			})
		}

		if (cumulativeVolumeUSD.lastUpdatedAt !== timestamp) {
			cumulativeVolumeUSD.volumeUSD = cumulativeVolumeUSD.volumeUSD + scaled
			cumulativeVolumeUSD.lastUpdatedAt = timestamp
		}

		await cumulativeVolumeUSD.save()
	}

	/**
	 * Update daily volume for a given identifier
	 * Creates a new record every 24 hours
	 * @param baseId - The base identifier for the daily volume record
	 * @param volumeUSD - The volume in USD to add (decimal string)
	 * @param timestamp - The timestamp of the transaction
	 */
	static async updateDailyVolume(baseId: string, volumeUSD: string, timestamp: bigint): Promise<void> {
		const id = this.getChainTypeId(baseId)
		const day = timestampToDate(timestamp)
		day.setUTCHours(0, 0, 0, 0)
		const dateString = day.toISOString().slice(0, 10)
		const dailyRecordId = `${id}.${dateString}`

		let dailyVolumeUSD = await DailyVolumeUSD.get(dailyRecordId)

		if (!dailyVolumeUSD) {
			dailyVolumeUSD = DailyVolumeUSD.create({
				id: dailyRecordId,
				last24HoursVolumeUSD: 0n,
				lastUpdatedAt: timestamp,
				createdAt: day,
			})
		}

		dailyVolumeUSD.last24HoursVolumeUSD = dailyVolumeUSD.last24HoursVolumeUSD + toScaledUsd(volumeUSD)
		dailyVolumeUSD.lastUpdatedAt = timestamp

		await dailyVolumeUSD.save()
	}

	/**
	 * One-time initialization of an aggregate volume series from already-indexed
	 * component series, for aggregates introduced after their components had
	 * accumulated history on a resumed database. Sums every cumulative and daily
	 * record whose ID starts with `componentIdPrefix` (scoped to this chain) into
	 * records under `aggregateBaseId`. The aggregate's cumulative record doubles as
	 * the done-marker, so this is a cheap no-op on every call after the first.
	 *
	 * Must be called before the triggering event's own volume updates: the seed
	 * assumes component records contain only history, and the seeded cumulative
	 * record keeps the components' max `lastUpdatedAt` so the same-timestamp guard
	 * in `updateCumulativeVolume` does not swallow the triggering event's volume.
	 */
	static async seedAggregateVolume(aggregateBaseId: string, componentIdPrefix: string): Promise<void> {
		const aggregateCumulativeId = this.getChainTypeId(aggregateBaseId)
		if (await CumulativeVolumeUSD.get(aggregateCumulativeId)) return

		const chainSuffix = `.${getHostStateMachine(chainId)}`
		const PAGE_SIZE = 100

		let cumulativeTotal = 0n
		let cumulativeLastUpdatedAt = 0n
		for (let offset = 0; ; offset += PAGE_SIZE) {
			const page = await CumulativeVolumeUSD.getByFields([], {
				limit: PAGE_SIZE,
				offset,
				orderBy: "id",
				orderDirection: "ASC",
			})
			for (const record of page) {
				if (!record.id.startsWith(componentIdPrefix) || !record.id.endsWith(chainSuffix)) continue
				cumulativeTotal += record.volumeUSD
				if (record.lastUpdatedAt > cumulativeLastUpdatedAt) cumulativeLastUpdatedAt = record.lastUpdatedAt
			}
			if (page.length < PAGE_SIZE) break
		}

		const dailyTotals = new Map<string, { volume: bigint; lastUpdatedAt: bigint }>()
		for (let offset = 0; ; offset += PAGE_SIZE) {
			const page = await DailyVolumeUSD.getByFields([], {
				limit: PAGE_SIZE,
				offset,
				orderBy: "id",
				orderDirection: "ASC",
			})
			for (const record of page) {
				// Component daily IDs end with `.<chain>.YYYY-MM-DD`
				const date = record.id.slice(-10)
				if (
					!record.id.startsWith(componentIdPrefix) ||
					!record.id.slice(0, -11).endsWith(chainSuffix) ||
					!/^\d{4}-\d{2}-\d{2}$/.test(date)
				)
					continue
				const bucket = dailyTotals.get(date) ?? { volume: 0n, lastUpdatedAt: 0n }
				bucket.volume += record.last24HoursVolumeUSD
				if (record.lastUpdatedAt > bucket.lastUpdatedAt) bucket.lastUpdatedAt = record.lastUpdatedAt
				dailyTotals.set(date, bucket)
			}
			if (page.length < PAGE_SIZE) break
		}

		for (const [date, bucket] of dailyTotals) {
			await DailyVolumeUSD.create({
				id: `${aggregateCumulativeId}.${date}`,
				last24HoursVolumeUSD: bucket.volume,
				lastUpdatedAt: bucket.lastUpdatedAt,
				createdAt: new Date(`${date}T00:00:00.000Z`),
			}).save()
		}

		await CumulativeVolumeUSD.create({
			id: aggregateCumulativeId,
			volumeUSD: cumulativeTotal,
			lastUpdatedAt: cumulativeLastUpdatedAt,
		}).save()

		logger.info(
			`[VolumeService.seedAggregateVolume] seeded ${aggregateCumulativeId} from ${componentIdPrefix}*: cumulative=${cumulativeTotal}, dailyRecords=${dailyTotals.size}`,
		)
	}

	/**
	 * Update both cumulative and daily volume in a single call
	 * @param id - The identifier for the volume records
	 * @param volumeUSD - The volume in USD to add
	 * @param timestamp - The timestamp of the transaction
	 */
	static async updateVolume(id: string, volumeUSD: string, timestamp: bigint): Promise<void> {
		await Promise.all([
			this.updateCumulativeVolume(id, volumeUSD, timestamp),
			this.updateDailyVolume(id, volumeUSD, timestamp),
		])
	}

	/**
	 * Generate a entity record ID base on the base ID (getDailyRecordId inclusive) and chainId
	 * @param baseId - The identifier for the volume record
	 */
	static getChainTypeId(baseId: string): string {
		const stateMachineId = getHostStateMachine(chainId)
		return `${baseId}.${stateMachineId}`
	}

	/**
	 * Generate a daily record ID based on the base ID and timestamp
	 * @param baseId - The base identifier
	 * @param timestamp - The timestamp
	 * @returns The daily record ID
	 */
	private static getDailyRecordId(baseId: string, timestamp: bigint): string {
		const dateString = getDateFormatFromTimestamp(timestamp)
		return `${baseId}.${dateString}`
	}
}
