import { isHex, hexToString } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { orderCommitment } from "./utils"
import { readLegEscrow, readLegPartialFill } from "./escrowReads"
import type { Order, HexString, TokenInfo } from "@/types"
import type { IntentGatewayContext } from "./types"

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
	 * Whether the order is finalized on the destination chain, read from the gateway's `_filled`
	 * getter. It is set by the completing fill, and by a cancellation, which records the refund
	 * beneficiary there. A partially filled order reads false; see {@link getFillProgress}.
	 *
	 * @param order - The V2 order to check. `order.id` is used as the commitment; if not set it is computed.
	 */
	async isOrderFilled(order: Order): Promise<boolean> {
		const commitment = (order.id ?? orderCommitment(order)) as HexString
		const gateway = this.ctx.dest.configService.getIntentGatewayAddress(destinationStateMachine(order))
		const finalizer = (await this.ctx.dest.client.readContract({
			abi: IntentGatewayV2ABI,
			address: gateway,
			functionName: "_filled",
			args: [commitment],
		})) as HexString
		return !/^0x0{40}$/i.test(finalizer)
	}

	/**
	 * Output credited to the order so far, one entry per leg, read from `_partialFills` on the
	 * destination chain. Several solvers can each fill a slice of a same-chain or cross-chain order,
	 * so this, not {@link isOrderFilled}, says how far along an open order is. Surplus paid above
	 * the order's rate is not counted.
	 *
	 * @param order - The V2 order to check. `order.id` is used as the commitment; if not set it is computed.
	 */
	async getFillProgress(order: Order): Promise<TokenInfo[]> {
		const commitment = (order.id ?? orderCommitment(order)) as HexString
		const gateway = this.ctx.dest.configService.getIntentGatewayAddress(destinationStateMachine(order))
		return Promise.all(
			order.output.assets.map(async (asset, index) => ({
				token: asset.token,
				amount: await readLegPartialFill(this.ctx.dest.client, gateway, commitment, index, asset.token),
			})),
		)
	}

	/**
	 * Checks if a V2 order has been refunded by reading the `_orders` mapping on the source chain.
	 *
	 * Calls `_orders(commitment, index)` for each input: escrow is held per leg, keyed by the input's
	 * index, so inputs that repeat a token are read separately. A gateway not yet upgraded to per-leg
	 * escrow is read by token instead, see {@link readLegEscrow}. When the order is placed the
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

		for (let index = 0; index < order.inputs.length; index++) {
			const escrowedAmount = await readLegEscrow(
				this.ctx.source.client,
				intentGatewayV2Address,
				commitment,
				index,
				order.inputs[index].token,
			)

			if (escrowedAmount !== 0n) {
				return false
			}
		}

		return true
	}
}

function destinationStateMachine(order: Order): string {
	return isHex(order.destination) ? hexToString(order.destination as HexString) : order.destination
}
