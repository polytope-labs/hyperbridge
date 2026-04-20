import type { OrderStatus, OrderWithStatus, TeleportStatus, HexString, TokenGatewayAssetTeleportedWithStatus } from "@/types"
import { _queryOrderInternal, _queryTokenGatewayAssetTeleportedInternal } from "@/query-client"

import { IsmpClient } from "./IsmpClient"
import { sleepForInterval } from "./utils"

/**
 * Backwards-compatible alias for {@link IsmpClient} that also retains the
 * order- and teleport-tracking methods originally shipped on the same class.
 *
 * @deprecated Prefer {@link IsmpClient} for ISMP request tracking and the
 * `IntentGateway` / `TokenGateway` classes for order- and teleport-status
 * tracking respectively. This alias will be removed in a future release.
 */
export class IndexerClient extends IsmpClient {
	/**
	 * @deprecated Use `IntentGateway.queryOrder()` instead.
	 */
	async queryOrder(commitment: HexString): Promise<OrderWithStatus | undefined> {
		return _queryOrderInternal({
			commitmentHash: commitment,
			queryClient: this.ctx.graphql,
			logger: this.ctx.logger,
		})
	}

	/**
	 * @deprecated Use `IntentGateway.orderStatusStream()` instead.
	 */
	async *orderStatusStream(commitment: HexString): AsyncGenerator<
		{
			status: OrderStatus
			metadata: {
				blockHash: string
				blockNumber: number
				transactionHash: string
				timestamp: bigint
				filler?: string
			}
		},
		void
	> {
		const logger = this.ctx.logger.withTag("[orderStatusStream]")

		const TERMINAL: OrderStatus[] = ["FILLED" as OrderStatus, "REDEEMED" as OrderStatus, "REFUNDED" as OrderStatus]
		let order: OrderWithStatus | undefined

		while (!order) {
			await sleepForInterval(this.ctx)
			order = await _queryOrderInternal({
				commitmentHash: commitment,
				queryClient: this.ctx.graphql,
				logger: this.ctx.logger,
			})
		}

		logger.trace("`Order` found")
		const latestStatus = order.statuses[order.statuses.length - 1]
		yield { status: latestStatus.status, metadata: latestStatus.metadata }

		if (TERMINAL.includes(latestStatus.status)) return

		while (true) {
			await sleepForInterval(this.ctx)
			const updatedOrder = await _queryOrderInternal({
				commitmentHash: commitment,
				queryClient: this.ctx.graphql,
				logger: this.ctx.logger,
			})
			if (!updatedOrder) continue

			const newLatestStatus = updatedOrder.statuses[updatedOrder.statuses.length - 1]
			if (newLatestStatus.status !== latestStatus.status) {
				yield { status: newLatestStatus.status, metadata: newLatestStatus.metadata }
				if (TERMINAL.includes(newLatestStatus.status)) return
			}
		}
	}

	/**
	 * @deprecated Use `TokenGateway.assetTeleportedStatusStream()` instead.
	 */
	async *tokenGatewayAssetTeleportedStatusStream(commitment: HexString): AsyncGenerator<
		{
			status: TeleportStatus
			metadata: { blockHash: string; blockNumber: number; transactionHash: string; timestamp: bigint }
		},
		void
	> {
		const logger = this.ctx.logger.withTag("[tokenGatewayAssetTeleportedStatusStream]")
		logger.trace(`Starting stream for token gateway asset teleported with commitment ${commitment}`)

		const TERMINAL: TeleportStatus[] = ["RECEIVED" as TeleportStatus, "REFUNDED" as TeleportStatus]
		let lastStatus: TeleportStatus | undefined
		let lastBlockNumber: number | undefined

		while (true) {
			try {
				const teleport = await this.queryTokenGatewayAssetTeleported(commitment)
				if (!teleport) {
					await sleepForInterval(this.ctx)
					continue
				}

				const statuses = teleport.statuses
				if (statuses.length === 0) {
					await sleepForInterval(this.ctx)
					continue
				}

				const latestStatus = statuses[statuses.length - 1]
				if (lastStatus === latestStatus.status && lastBlockNumber === latestStatus.metadata.blockNumber) {
					await sleepForInterval(this.ctx)
					continue
				}

				lastStatus = latestStatus.status
				lastBlockNumber = latestStatus.metadata.blockNumber

				yield latestStatus

				if (TERMINAL.includes(latestStatus.status)) break

				await sleepForInterval(this.ctx)
			} catch (error) {
				logger.error("Error in token gateway asset teleported status stream:", error)
				await sleepForInterval(this.ctx)
			}
		}
	}

	private async queryTokenGatewayAssetTeleported(
		commitment: HexString,
	): Promise<TokenGatewayAssetTeleportedWithStatus | undefined> {
		return _queryTokenGatewayAssetTeleportedInternal({
			commitmentHash: commitment,
			queryClient: this.ctx.graphql,
			logger: this.ctx.logger,
		})
	}
}
