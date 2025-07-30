import Decimal from "decimal.js"

import { CumulativeVolumeUSD, DailyVolumeUSD } from "@/configs/src/types"
import { getDateFormatFromTimestamp, isWithin24Hours, timestampToDate } from "@/utils/date.helpers"
import { getHostStateMachine } from "@/utils/substrate.helpers"

export class VolumeService {
	/**
	 * Update cumulative volume for a given identifier
	 * @param baseId - The identifier for the cumulative volume record
	 * @param volumeUSD - The volume in USD to add
	 * @param timestamp - The timestamp of the transaction
	 */
	static async updateCumulativeVolume(baseId: string, volumeUSD: string, timestamp: bigint): Promise<void> {
		const id = this.getChainTypeId(baseId)
		let cumulativeVolumeUSD = await CumulativeVolumeUSD.get(id)

		if (!cumulativeVolumeUSD) {
			cumulativeVolumeUSD = CumulativeVolumeUSD.create({
				id,
				volumeUSD: new Decimal(volumeUSD).toFixed(18),
				lastUpdatedAt: timestamp,
			})
		}

		if (cumulativeVolumeUSD.lastUpdatedAt !== timestamp) {
			cumulativeVolumeUSD.volumeUSD = new Decimal(cumulativeVolumeUSD.volumeUSD)
				.plus(new Decimal(volumeUSD))
				.toFixed(18)
			cumulativeVolumeUSD.lastUpdatedAt = timestamp
		}

		await cumulativeVolumeUSD.save()
	}

	/**
	 * Update daily volume for a given identifier
	 * Creates a new record every 24 hours
	 * @param baseId - The base identifier for the daily volume record
	 * @param volumeUSD - The volume in USD to add
	 * @param timestamp - The timestamp of the transaction
	 */
	static async updateDailyVolume(baseId: string, volumeUSD: string, timestamp: bigint): Promise<void> {
		const id = this.getChainTypeId(baseId)
		const dailyRecordId = this.getDailyRecordId(id, timestamp)
		let dailyVolumeUSD = await DailyVolumeUSD.get(dailyRecordId)

		if (!dailyVolumeUSD) {
			dailyVolumeUSD = DailyVolumeUSD.create({
				id: dailyRecordId,
				last24HoursVolumeUSD: new Decimal(volumeUSD).toFixed(18),
				lastUpdatedAt: timestamp,
				createdAt: timestampToDate(timestamp),
			})
		}

		if (isWithin24Hours(dailyVolumeUSD.createdAt, timestamp) && dailyVolumeUSD.lastUpdatedAt !== timestamp) {
			dailyVolumeUSD.last24HoursVolumeUSD = new Decimal(dailyVolumeUSD.last24HoursVolumeUSD)
				.plus(new Decimal(volumeUSD))
				.toFixed(18)
			dailyVolumeUSD.lastUpdatedAt = timestamp
		}

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
