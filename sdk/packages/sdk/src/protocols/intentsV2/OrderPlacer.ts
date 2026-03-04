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

export class OrderPlacer {
	constructor(private readonly ctx: IntentsV2Context) {}

	async *placeOrder(
		order: OrderV2,
		graffiti: HexString = DEFAULT_GRAFFITI,
	): AsyncGenerator<{ calldata: HexString; sessionPrivateKey: HexString }, OrderV2, any> {
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

		const calldata = encodeFunctionData({
			abi: IntentGatewayV2ABI,
			functionName: "placeOrder",
			args: [transformOrderForContract(order), graffiti],
		}) as HexString

		const signedTransaction = yield { calldata, sessionPrivateKey: privateKey as HexString }

		const receipt = await this.ctx.source.broadcastTransaction(signedTransaction)

		console.log(
			`Place order transaction sent to source chain ${hexToString(order.source as HexString)} with hash: ${receipt.transactionHash}`,
		)

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

		return order
	}
}
