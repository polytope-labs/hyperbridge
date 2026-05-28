import {
	GetRequestStatusMetadata,
	GetRequestV2,
	OrderStatus,
	PendingStatusMetadata,
	RequestStatusMetadata,
	RequestV2,
	Status,
} from "@/configs/src/types"
import { IOrderV3 } from "@/configs/src/types/models/IOrderV3"
import { IOrderV3StatusMetadata } from "@/configs/src/types/models/IOrderV3StatusMetadata"

// Entity types we know how to materialize a real *StatusMetadata for.
// Must match the `ENTITY_TYPE` constants in the per-service handlers that
// write PendingStatusMetadata rows.
const ENTITY_TYPES = ["RequestV2", "GetRequestV2", "IOrderV3"] as const
type KnownEntityType = (typeof ENTITY_TYPES)[number]

export class PendingStatusService {
	/**
	 * Scan a small batch of pending status rows across the known entity types
	 * and materialize the real *StatusMetadata for any whose parent now exists,
	 * deleting the pending row. Rows whose parent is still missing are left
	 * untouched for a future tick.
	 *
	 * `limit` caps the total rows examined per call, distributed across the
	 * entity types so a backlog on one type cannot starve the others. We
	 * cannot order by `createdAt` because that column isn't indexed on the
	 * existing table; rows we successfully flush are deleted, so even with
	 * arbitrary ordering the queue drains over successive blocks.
	 */
	static async flushBatch(limit: number): Promise<void> {
		const perType = Math.max(1, Math.ceil(limit / ENTITY_TYPES.length))
		logger.info(
			`[PendingStatusService.flushBatch] starting, limit=${limit} perType=${perType} entityTypes=${ENTITY_TYPES.join(",")}`,
		)

		for (const entityType of ENTITY_TYPES) {
			const batch = await PendingStatusMetadata.getByEntityType(entityType, {
				limit: perType,
			})
			logger.info(
				`[PendingStatusService.flushBatch] fetched ${batch.length} pending row(s) for entityType=${entityType}`,
			)

			for (const pending of batch) {
				await this.materialize(pending, entityType)
			}
		}

		logger.info(`[PendingStatusService.flushBatch] finished`)
	}

	private static async materialize(
		pending: PendingStatusMetadata,
		entityType: KnownEntityType,
	): Promise<void> {
		switch (entityType) {
			case "RequestV2": {
				const parent = await RequestV2.get(pending.commitment)
				if (!parent) {
					logger.info(
						`[PendingStatusService] RequestV2 ${pending.commitment} not yet present, leaving pending`,
					)
					return
				}
				const statusMetadata = RequestStatusMetadata.create({
					id: `${pending.commitment}.${pending.status}`,
					requestId: pending.commitment,
					status: pending.status as Status,
					chain: pending.chain,
					timestamp: pending.timestamp,
					blockNumber: pending.blockNumber,
					blockHash: pending.blockHash,
					transactionHash: pending.transactionHash,
					createdAt: pending.createdAt,
				})
				await statusMetadata.save()
				await PendingStatusMetadata.remove(pending.id)
				logger.info(
					`[PendingStatusService] Flushed RequestV2 ${pending.commitment} status ${pending.status}`,
				)
				return
			}
			case "GetRequestV2": {
				const parent = await GetRequestV2.get(pending.commitment)
				if (!parent) {
					logger.info(
						`[PendingStatusService] GetRequestV2 ${pending.commitment} not yet present, leaving pending`,
					)
					return
				}
				const statusMetadata = GetRequestStatusMetadata.create({
					id: `${pending.commitment}.${pending.status}`,
					requestId: pending.commitment,
					status: pending.status as Status,
					chain: pending.chain,
					timestamp: pending.timestamp,
					blockNumber: pending.blockNumber,
					blockHash: pending.blockHash,
					transactionHash: pending.transactionHash,
					createdAt: pending.createdAt,
				})
				await statusMetadata.save()
				await PendingStatusMetadata.remove(pending.id)
				logger.info(
					`[PendingStatusService] Flushed GetRequestV2 ${pending.commitment} status ${pending.status}`,
				)
				return
			}
			case "IOrderV3": {
				const parent = await IOrderV3.get(pending.commitment)
				if (!parent) {
					logger.info(
						`[PendingStatusService] IOrderV3 ${pending.commitment} not yet present, leaving pending`,
					)
					return
				}
				const statusMetadata = IOrderV3StatusMetadata.create({
					id: `${pending.commitment}.${pending.status}`,
					orderId: pending.commitment,
					status: pending.status as OrderStatus,
					chain: pending.chain,
					timestamp: pending.timestamp,
					blockNumber: pending.blockNumber,
					filler: pending.filler,
					transactionHash: pending.transactionHash,
					createdAt: pending.createdAt,
				})
				await statusMetadata.save()
				await PendingStatusMetadata.remove(pending.id)
				logger.info(
					`[PendingStatusService] Flushed IOrderV3 ${pending.commitment} status ${pending.status}`,
				)
				return
			}
		}
	}
}
