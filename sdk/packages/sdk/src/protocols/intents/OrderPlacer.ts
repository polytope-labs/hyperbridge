import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { SessionKeyData } from "@/storage"
import type { DecodedOrderPlacedLog, Order } from "@/types"
import type { HexString } from "@/types"
import { normalizeStateMachineId } from "@/utils"
import { type TransactionReceipt, encodeFunctionData, parseEventLogs } from "viem"
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts"
import type { IntentGatewayContext } from "./types"
import { DEFAULT_GRAFFITI } from "./types"
import { orderCommitment } from "./utils"
import { transformOrderForContract } from "./utils"

/**
 * Reconstructs the canonical order committed by the gateway from an
 * `OrderPlaced` event.
 *
 * The gateway stamps or normalises every field emitted by the event before it
 * hashes the order. The two call-data fields are not emitted, so they must be
 * retained from the submitted order.
 */
export function deriveCanonicalPlacedOrder(
	order: Order,
	args: DecodedOrderPlacedLog["args"],
): Order & { id: HexString } {
	const canonicalOrder: Order = {
		...order,
		user: args.user,
		source: args.source,
		destination: args.destination,
		deadline: args.deadline,
		nonce: args.nonce,
		fees: args.fees,
		session: args.session,
		predispatch: {
			assets: args.predispatch.map((asset) => ({ ...asset })),
			call: order.predispatch.call,
		},
		inputs: args.inputs.map((asset) => ({ ...asset })),
		output: {
			beneficiary: args.beneficiary,
			assets: args.outputs.map((asset) => ({ ...asset })),
			call: order.output.call,
		},
	}

	return { ...canonicalOrder, id: orderCommitment(canonicalOrder) }
}

/**
 * Handles order placement on the source chain for IntentGatewayV2.
 *
 * Generates a fresh ephemeral session key per order, encodes the
 * `placeOrder` calldata, yields it to the caller for signing and
 * submission, then waits for the on-chain `OrderPlaced` event to
 * reconstruct the canonical order before computing its final commitment.
 */
export class OrderPlacer {
	/**
	 * @param ctx - Shared IntentsV2 context providing the source chain client,
	 *   config service, and session-key storage.
	 */
	constructor(private readonly ctx: IntentGatewayContext) {}

	/**
	 * Bidirectional async generator that orchestrates order placement.
	 *
	 * **Yield/receive protocol:**
	 * 1. Yields `{ to, data, sessionPrivateKey }` — the caller must sign a
	 *    transaction sending `data` to `to` and pass the signed transaction back
	 *    via `gen.next(signedTx)`.
	 * 2. Returns `{ order, transactionHash }` — the finalized order rebuilt from
	 *    the canonical `OrderPlaced` fields with its computed `id`, plus the
	 *    placement transaction hash.
	 *
	 * A fresh ephemeral session key is generated for every call. The key is
	 * stored immediately (without a commitment) so it can be retrieved by
	 * address, then updated with the commitment once the `OrderPlaced` event
	 * confirms the canonical order.
	 *
	 * @param order - The order to place. It is not mutated.
	 * @param graffiti - Optional bytes32 tag for orderflow attribution /
	 *   revenue share. Defaults to {@link DEFAULT_GRAFFITI} (bytes32 zero).
	 * @yields `{ to, data, sessionPrivateKey }` — target contract address,
	 *   encoded `placeOrder` calldata, and the raw session private key.
	 * @returns `{ order, transactionHash }` after the transaction is confirmed
	 *   and the `OrderPlaced` event is parsed.
	 * @throws If the broadcast transaction receipt does not contain an
	 *   `OrderPlaced` event.
	 */
	async *placeOrder(
		order: Order,
		graffiti: HexString = DEFAULT_GRAFFITI,
	): AsyncGenerator<
		{ to: HexString; data: HexString; sessionPrivateKey: HexString },
		{ order: Order; receipt: TransactionReceipt },
		HexString
	> {
		const privateKey = generatePrivateKey()
		const account = privateKeyToAccount(privateKey)
		const sessionKeyAddress = account.address as HexString
		const createdAt = Date.now()
		const placementOrder: Order = { ...order, session: sessionKeyAddress }

		await this.ctx.sessionKeyStorage.setSessionKeyByAddress(sessionKeyAddress, {
			privateKey: privateKey as HexString,
			address: sessionKeyAddress,
			createdAt,
		})

		const data = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "placeOrder",
			args: [transformOrderForContract(placementOrder), graffiti],
		}) as HexString

		const intentGatewayAddress = this.ctx.source.configService.getIntentGatewayAddress(
			normalizeStateMachineId(order.source),
		)

		const signedTransaction = yield {
			to: intentGatewayAddress,
			data,
			sessionPrivateKey: privateKey as HexString,
		}

		const receipt =
			signedTransaction.length === 66
				? await this.ctx.source.getTransactionReceipt(signedTransaction)
				: await this.ctx.source.broadcastTransaction(signedTransaction)

		const events = parseEventLogs({
			abi: IntentGatewayV2ABI,
			logs: receipt.logs,
			eventName: "OrderPlaced",
		})

		const orderPlacedEvent = events[0] as DecodedOrderPlacedLog | undefined
		if (!orderPlacedEvent) {
			throw new Error("OrderPlaced event not found in transaction receipt")
		}

		const finalizedOrder = deriveCanonicalPlacedOrder(placementOrder, orderPlacedEvent.args)

		const sessionKeyData: SessionKeyData = {
			privateKey: privateKey as HexString,
			address: sessionKeyAddress,
			commitment: finalizedOrder.id,
			createdAt,
		}

		await this.ctx.sessionKeyStorage.setSessionKeyByAddress(sessionKeyAddress, sessionKeyData)

		return { order: finalizedOrder, receipt }
	}
}
