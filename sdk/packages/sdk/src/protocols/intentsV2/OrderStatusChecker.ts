import { isHex, hexToString } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { orderV2Commitment, bytes32ToBytes20 } from "@/utils"
import type { OrderV2, HexString } from "@/types"
import type { IntentsV2Context } from "./types"

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
	constructor(private readonly ctx: IntentsV2Context) {}

	/**
	 * Checks if a V2 order has been filled by reading the commitment storage slot on the destination chain.
	 *
	 * Reads the storage slot returned by `calculateCommitmentSlotHash` on the IntentGatewayV2 contract.
	 * A non-zero value at that slot means the solver has called `fillOrder` and the order is complete
	 * from the user's perspective (the beneficiary has received their tokens).
	 *
	 * @param order - The V2 order to check. `order.id` is used as the commitment; if not set it is computed.
	 * @returns True if the order has been filled on the destination chain, false otherwise.
	 */
	async isOrderFilled(order: OrderV2): Promise<boolean> {
		const commitment = (order.id ?? orderV2Commitment(order)) as HexString
		const destStateMachineId = isHex(order.destination)
			? hexToString(order.destination as HexString)
			: order.destination

		const intentGatewayV2Address = this.ctx.dest.configService.getIntentGatewayV2Address(destStateMachineId)

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
	 * Checks if a V2 order has been refunded by reading the `_orders` mapping on the source chain.
	 *
	 * Calls `_orders(commitment, tokenAddress)` for each input token. When the order is placed the
	 * escrowed amounts are stored there. After a successful refund the contract zeroes them out.
	 * An order is considered refunded when all escrowed input amounts have been returned (i.e. are 0).
	 *
	 * @param order - The V2 order to check. `order.id` is used as the commitment; if not set it is computed.
	 * @returns True if all escrowed inputs have been returned to the user on the source chain, false otherwise.
	 */
	async isOrderRefunded(order: OrderV2): Promise<boolean> {
		if (!order.inputs || order.inputs.length === 0) return false

		const commitment = (order.id ?? orderV2Commitment(order)) as HexString
		const sourceStateMachineId = isHex(order.source)
			? hexToString(order.source as HexString)
			: order.source

		const intentGatewayV2Address = this.ctx.source.configService.getIntentGatewayV2Address(sourceStateMachineId)

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
