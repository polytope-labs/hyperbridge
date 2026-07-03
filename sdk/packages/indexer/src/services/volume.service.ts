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
