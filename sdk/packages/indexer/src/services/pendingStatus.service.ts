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
const KNOWN_ENTITY_TYPES = ["RequestV2", "GetRequestV2", "IOrderV3"] as const
type KnownEntityType = (typeof KNOWN_ENTITY_TYPES)[number]

function isKnownEntityType(value: string): value is KnownEntityType {
	return (KNOWN_ENTITY_TYPES as readonly string[]).includes(value)
}

export class PendingStatusService {
	/**
	 * Scan a small batch of pending status rows and materialize the real
	 * *StatusMetadata for any whose parent now exists, deleting the pending
	 * row. Rows whose parent is still missing are left untouched for a
	 * future tick.
	 *
	 * Fetches via `getByFields([], { limit })` so a stale or mismatched
	 * `entityType` index value can't hide rows from us — we dispatch on
	 * each row's own `entityType` field after the read.
	 */
	static async flushBatch(limit: number): Promise<void> {
		logger.info(`[PendingStatusService.flushBatch] starting, limit=${limit}`)

		// Use an indexed-field `in` filter so the query is non-empty and predictable.
		// `entityType` is `@index`-ed on the schema, so this passes the SubQuery
		// indexed-field assert in @subql/node-core store.js.
		const batch = await PendingStatusMetadata.getByFields(
			[["entityType", "in", [...KNOWN_ENTITY_TYPES]]],
			{ limit },
		)
		logger.info(
			`[PendingStatusService.flushBatch] fetched ${batch.length} pending row(s); ` +
				`sample=${batch
					.slice(0, 3)
					.map((p) => `${p.entityType}@${p.chain}:${p.id}`)
					.join(", ")}`,
		)

		for (const pending of batch) {
			if (!isKnownEntityType(pending.entityType)) {
				logger.warn(
					`[PendingStatusService] unknown entityType=${pending.entityType} on pending ${pending.id}, skipping`,
				)
				continue
			}
			await this.materialize(pending, pending.entityType)
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
