import { isHex, hexToString } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { bytes32ToBytes20, normalizeAddressForEvmBytes32 } from "@/utils"
import { orderCommitment } from "./utils"
import type { Order, HexString, TokenInfo } from "@/types"
import type { IntentGatewayContext } from "./types"

/** Per-output-token fill progress read from the destination `_partialFills` mapping. */
export interface TokenFillProgress {
	/** Output token, as provided in `order.output.assets[i].token`. */
	token: TokenInfo["token"]
	/** Cumulative amount of this output filled so far across all (partial) fills. */
	filled: bigint
	/** Total amount required for this output (`order.output.assets[i].amount`). */
	total: bigint
}

/** Aggregate fill state of an order derived from its per-token progress. */
export interface OrderFillProgress {
	perToken: TokenFillProgress[]
	/** `unfilled` when nothing filled, `full` when every output is satisfied, otherwise `partial`. */
	status: "unfilled" | "partial" | "full"
}

/**
 * Checks the on-chain fill and refund status of IntentGatewayV2 orders.
 *
 * Reads contract storage directly rather than relying on events, so the
 * results are accurate even if the caller misses the confirmation window.
 */
export class OrderStatusChecker {
	/**
	 * @param ctx - Shared IntentsV2 context providing the source and destination
	 *   chain clients and config service.
	 */
	constructor(private readonly ctx: IntentGatewayContext) {}

	/**
	 * Checks if a V2 order has been filled by reading the commitment storage slot on the destination chain.
	 *
	 * Reads the storage slot returned by `calculateCommitmentSlotHash` on the IntentGatewayV2 contract.
	 * A non-zero value at that slot means the order has been finalized — either fully filled or
	 * cancelled. Note that partial fills clear this slot so the next solver can continue, so during an
	 * in-progress cross-chain partial fill this returns `false` until the order is completed. Use
	 * {@link getFillProgress} to observe intermediate per-token progress.
	 *
	 * @param order - The V2 order to check. `order.id` is used as the commitment; if not set it is computed.
	 * @returns True if the order has been finalized on the destination chain, false otherwise.
	 */
	async isOrderFilled(order: Order): Promise<boolean> {
		const commitment = (order.id ?? orderCommitment(order)) as HexString
		const destStateMachineId = isHex(order.destination)
			? hexToString(order.destination as HexString)
			: order.destination

		const intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayAddress(destStateMachineId)

		const filledSlot = await this.ctx.dest.client.readContract({
			abi: IntentGatewayV2ABI,
			address: intentGatewayV2Address,
			functionName: "calculateCommitmentSlotHash",
			args: [commitment],
		})

		const filledStatus = await this.ctx.dest.client.getStorageAt({
			address: intentGatewayV2Address,
			slot: filledSlot as HexString,
		})

		return filledStatus !== "0x0000000000000000000000000000000000000000000000000000000000000000"
	}

	/**
	 * Reads per-output-token fill progress from the destination `_partialFills` mapping.
	 *
	 * Unlike {@link isOrderFilled} (which reads the terminal `_filled` slot), this reflects
	 * intermediate progress across repeated partial fills, so callers can track a cross-chain order
	 * as multiple solvers fill successive slices.
	 *
	 * @param order - The V2 order to check. `order.id` is used as the commitment; if not set it is computed.
	 * @returns Per-token filled/total amounts and an aggregate `unfilled | partial | full` status.
	 */
	async getFillProgress(order: Order): Promise<OrderFillProgress> {
		const commitment = (order.id ?? orderCommitment(order)) as HexString
		const destStateMachineId = isHex(order.destination)
			? hexToString(order.destination as HexString)
			: order.destination

		const intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayAddress(destStateMachineId)

		const perToken = await Promise.all(
			order.output.assets.map(async (asset) => {
				const filled = (await this.ctx.dest.client.readContract({
					abi: IntentGatewayV2ABI,
					address: intentGatewayV2Address,
					functionName: "_partialFills",
					args: [commitment, normalizeAddressForEvmBytes32(asset.token)],
				})) as bigint

				return { token: asset.token, filled, total: asset.amount }
			}),
		)

		const anyFilled = perToken.some((t) => t.filled > 0n)
		const allFilled = perToken.every((t) => t.filled >= t.total)
		const status = allFilled ? "full" : anyFilled ? "partial" : "unfilled"

		return { perToken, status }
	}

	/**
	 * Checks if a V2 order has been refunded by reading the `_orders` mapping on the source chain.
	 *
	 * Calls `_orders(commitment, tokenAddress)` for each input token. When the order is placed the
	 * escrowed amounts are stored there. After a successful refund the contract zeroes them out.
	 * An order is considered refunded when all escrowed input amounts have been returned (i.e. are 0).
	 *
	 * @param order - The V2 order to check. `order.id` is used as the commitment; if not set it is computed.
	 * @returns True if all escrowed inputs have been returned to the user on the source chain, false otherwise.
	 */
	async isOrderRefunded(order: Order): Promise<boolean> {
		if (!order.inputs || order.inputs.length === 0) return false

		const commitment = (order.id ?? orderCommitment(order)) as HexString
		const sourceStateMachineId = isHex(order.source) ? hexToString(order.source as HexString) : order.source

		const intentGatewayV2Address = this.ctx.source.configService.getIntentGatewayAddress(sourceStateMachineId)

		for (const input of order.inputs) {
			const tokenAddress = bytes32ToBytes20(input.token)
			const escrowedAmount = await this.ctx.source.client.readContract({
				abi: IntentGatewayV2ABI,
				address: intentGatewayV2Address,
				functionName: "_orders",
				args: [commitment, tokenAddress],
			})

			if (escrowedAmount !== 0n) {
				return false
			}
		}

		return true
	}
}
