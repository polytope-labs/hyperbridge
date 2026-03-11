import { encodeFunctionData, parseEventLogs } from "viem"
import { generatePrivateKey, privateKeyToAccount } from "viem/accounts"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import { hexToString, orderV2Commitment } from "@/utils"
import type { OrderV2, DecodedOrderV2PlacedLog } from "@/types"
import type { HexString } from "@/types"
import type { SessionKeyData } from "@/storage"
import type { IntentsV2Context } from "./types"
import { DEFAULT_GRAFFITI } from "./types"
import { transformOrderForContract } from "./utils"

/**
 * Handles order placement on the source chain for IntentGatewayV2.
 *
 * Generates a fresh ephemeral session key per order, encodes the
 * `placeOrder` calldata, yields it to the caller for signing and
 * submission, then waits for the on-chain `OrderPlaced` event to
 * extract the canonical nonce and inputs before computing the final
 * order commitment.
 */
export class OrderPlacer {
	/**
	 * @param ctx - Shared IntentsV2 context providing the source chain client,
	 *   config service, and session-key storage.
	 */
	constructor(private readonly ctx: IntentsV2Context) {}

	/**
	 * Bidirectional async generator that orchestrates order placement.
	 *
	 * **Yield/receive protocol:**
	 * 1. Yields `{ to, data, sessionPrivateKey }` — the caller must sign a
	 *    transaction sending `data` to `to` and pass the signed transaction back
	 *    via `gen.next(signedTx)`.
	 * 2. Returns `{ order, transactionHash }` — the finalized order with its
	 *    on-chain `nonce`, `inputs`, and computed `id`, plus the placement
	 *    transaction hash.
	 *
	 * A fresh ephemeral session key is generated for every call. The key is
	 * stored immediately (without a commitment) so it can be retrieved by
	 * address, then updated with the commitment once the `OrderPlaced` event
	 * confirms the nonce.
	 *
	 * @param order - The order to place. `order.session` is mutated in-place
	 *   with the generated session key address before yielding.
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
		order: OrderV2,
		graffiti: HexString = DEFAULT_GRAFFITI,
	): AsyncGenerator<
		{ to: HexString; data: HexString; sessionPrivateKey: HexString },
		{ order: OrderV2; transactionHash: HexString },
		HexString
	> {
		const privateKey = generatePrivateKey()
		const account = privateKeyToAccount(privateKey)
		const sessionKeyAddress = account.address as HexString
		const createdAt = Date.now()

		order.session = sessionKeyAddress

		await this.ctx.sessionKeyStorage.setSessionKeyByAddress(sessionKeyAddress, {
			privateKey: privateKey as HexString,
			address: sessionKeyAddress,
			createdAt,
		})

		const data = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "placeOrder",
			args: [transformOrderForContract(order), graffiti],
		}) as HexString

		const intentGatewayAddress = this.ctx.source.configService.getIntentGatewayV2Address(
			hexToString(order.source as HexString),
		)

		const signedTransaction = yield {
			to: intentGatewayAddress,
			data,
			sessionPrivateKey: privateKey as HexString,
		}

		const receipt = await this.ctx.source.broadcastTransaction(signedTransaction)

		const events = parseEventLogs({
			abi: IntentGatewayV2ABI,
			logs: receipt.logs,
			eventName: "OrderPlaced",
		})

		const orderPlacedEvent = events[0] as DecodedOrderV2PlacedLog | undefined
		if (!orderPlacedEvent) {
			throw new Error("OrderPlaced event not found in transaction receipt")
		}

		order.nonce = orderPlacedEvent.args.nonce
		order.inputs = orderPlacedEvent.args.inputs.map((input) => ({
			token: input.token,
			amount: input.amount,
		}))

		order.id = orderV2Commitment(order)

		const sessionKeyData: SessionKeyData = {
			privateKey: privateKey as HexString,
			address: sessionKeyAddress,
			commitment: order.id as HexString,
			createdAt,
		}

		await this.ctx.sessionKeyStorage.setSessionKeyByAddress(sessionKeyAddress, sessionKeyData)

		return { order, transactionHash: receipt.transactionHash as HexString }
	}
}
