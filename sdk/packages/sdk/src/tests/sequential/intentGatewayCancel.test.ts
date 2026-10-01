import "log-timestamp"

import { strict as assert } from "node:assert"
import { describe, it } from "vitest"
import {
	type Account,
	type Chain,
	type Transport,
	type WalletClient,
	createWalletClient,
	decodeAbiParameters,
	decodeFunctionData,
	erc20Abi,
	hexToNumber,
	http,
	isAddressEqual,
	maxUint256,
	parseEventLogs,
	parseUnits,
	sliceHex,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet, polygonAmoy } from "viem/chains"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import EVM_HOST from "@/abis/evmHost"
import { EvmChain } from "@/chains/evm"
import { SubstrateChain } from "@/chains/substrate"
import { IsmpClient } from "@/client"
import { ChainConfigService } from "@/configs/ChainConfigService"
import { IntentGateway } from "@/protocols/intents/IntentGateway"
import { readLegEscrow } from "@/protocols/intents/escrowReads"
import { type CancelEvent, DEFAULT_GRAFFITI } from "@/protocols/intents/types"
import { createQueryClient } from "@/queryClient"
import { type HexString, type Order, RequestKind } from "@/types"
import { ADDRESS_ZERO, bytes20ToBytes32, postRequestCommitment } from "@/utils"

/**
 * Live cancellation of a cross-chain order from its destination: BSC Chapel to Polygon Amoy.
 *
 * Places an order no solver will bid on, cancels it on Amoy, which freezes the order there and posts
 * RefundEscrow back to Chapel, and follows that POST until Hyperbridge holds it. With
 * CANCEL_FULL_REFUND set to "true" or "1" it goes on to self-deliver the POST to the Chapel handler,
 * since the testnet relayer does not deliver to EVM chains, and checks the escrow is refunded.
 *
 * Needs PRIVATE_KEY, BSC_CHAPEL, POLYGON_AMOY and HYPERBRIDGE_GARGANTUA. GARGANTUA_INDEXER_URL
 * overrides the public gargantua indexer.
 */

const CHAPEL = "EVM-97"
const AMOY = "EVM-80002"
const FULL_REFUND = process.env.CANCEL_FULL_REFUND === "true" || process.env.CANCEL_FULL_REFUND === "1"

const ORDER_LIFETIME_BLOCKS = 43_200n
const STEP_TIMEOUT_MS = 5 * 60_000
const DELIVERED_TIMEOUT_MS = 15 * 60_000
const REFUND_TIMEOUT_MS = 90 * 60_000
const SETUP_TIMEOUT_MS = 10 * 60_000
const RECEIPT_POLL_MS = 15_000

const WITHDRAWAL_REQUEST = [
	{
		type: "tuple",
		components: [
			{ name: "commitment", type: "bytes32" },
			{ name: "beneficiary", type: "bytes32" },
			{
				name: "tokens",
				type: "tuple[]",
				components: [
					{ name: "token", type: "bytes32" },
					{ name: "amount", type: "uint256" },
				],
			},
		],
	},
] as const

describe("IntentGateway cancel from destination, BSC Chapel to Polygon Amoy (live)", () => {
	it(
		FULL_REFUND
			? "cancels an unfillable order on Amoy and refunds its escrow on Chapel"
			: "cancels an unfillable order on Amoy and delivers the refund to Hyperbridge",
		async () => {
			const env = readEnv()
			const account = privateKeyToAccount(env.privateKey)
			const configService = new ChainConfigService()
			const chapel = liveChain(CHAPEL, bscTestnet, env.chapelRpc, account, configService)
			const amoy = liveChain(AMOY, polygonAmoy, env.amoyRpc, account, configService)
			const hyperbridge = await SubstrateChain.connect({
				wsUrl: env.hyperbridgeWs,
				consensusStateId: "PAS0",
				hasher: "Keccak",
				stateMachineId: "KUSAMA-4009",
			})
			let stream: CancelStream | undefined

			try {
				const chapelFeeToken = await chapel.evm.getFeeTokenWithDecimals()
				const amoyFeeToken = await amoy.evm.getFeeTokenWithDecimals()
				const inputToken = configService.getUsdcAsset(CHAPEL)
				await approveGateway(chapel, inputToken)
				await approveGateway(chapel, chapelFeeToken.address)
				await approveGateway(amoy, amoyFeeToken.address)

				const user = bytes20ToBytes32(account.address)
				const order: Order = {
					user,
					source: CHAPEL,
					destination: AMOY,
					deadline: (await amoy.evm.client.getBlockNumber()) + ORDER_LIFETIME_BLOCKS,
					nonce: 0n,
					fees: parseUnits("0.01", chapelFeeToken.decimals),
					session: ADDRESS_ZERO,
					predispatch: { assets: [], call: "0x" },
					inputs: [
						{
							token: bytes20ToBytes32(inputToken),
							amount: parseUnits("0.01", configService.getUsdcDecimals(CHAPEL)),
						},
					],
					output: {
						beneficiary: user,
						assets: [
							{
								token: bytes20ToBytes32(configService.getUsdcAsset(AMOY)),
								amount: parseUnits("1000000", configService.getUsdcDecimals(AMOY)),
							},
						],
						call: "0x",
					},
				}

				const gateway = await IntentGateway.create(chapel.evm, amoy.evm)
				const placed = await placeOrder(gateway, chapel, order, chapelFeeToken.address)

				const escrowed = await readLegEscrow(
					chapel.evm.client,
					chapel.gateway,
					placed.id,
					0,
					placed.inputs[0].token,
				)
				assert.equal(escrowed, placed.inputs[0].amount, "Chapel escrow differs from the committed input")
				assert.equal(await gateway.isOrderFilled(placed), false, "A new order already reads as filled")
				assert.equal(await gateway.isOrderRefunded(placed), false, "A new order already reads as refunded")

				const ismpClient = new IsmpClient({
					queryClient: createQueryClient({ url: env.indexerUrl }),
					source: amoy.evm,
					dest: chapel.evm,
					hyperbridge,
					pollInterval: 5_000,
				})
				stream = new CancelStream(gateway.cancelOrder(placed, ismpClient, { from: "destination" }))

				const cancelTx = await advanceTo(stream, "AWAITING_CANCEL_TRANSACTION", Date.now() + STEP_TIMEOUT_MS)
				assert(isAddressEqual(cancelTx.to, amoy.gateway), `Cancel targets ${cancelTx.to}, not the Amoy gateway`)
				assert.equal(cancelTx.value, 0n, "Testnet hosts have no swap route, so the cancel must carry no value")
				const call = decodeFunctionData({ abi: IntentGatewayV2ABI, data: cancelTx.data })
				if (call.functionName !== "cancelOrder") throw new Error(`Cancel calldata calls ${call.functionName}`)
				const [, cancelOptions] = call.args
				assert(cancelOptions.relayerFee > 0n, "Cancel carries no relayer fee")
				assert.equal(cancelOptions.height, 0n, "A destination cancel needs no proof height")

				const started = await advanceTo(
					stream,
					"CANCEL_STARTED",
					Date.now() + STEP_TIMEOUT_MS,
					await signTransaction(amoy, cancelTx),
				)
				const { receipt } = started
				console.log(`[cancel] cancel tx ${receipt.transactionHash} on Amoy`)
				assert.equal(receipt.status, "success", "Cancel transaction reverted")

				const [cancelled] = parseEventLogs({
					abi: IntentGatewayV2ABI,
					logs: receipt.logs,
					eventName: "OrderCancelled",
				})
				assert(cancelled, "OrderCancelled missing from the cancel receipt")
				assert.equal(cancelled.args.commitment, placed.id)
				assert(isAddressEqual(cancelled.args.canceller, account.address))

				const [dispatched] = parseEventLogs({
					abi: EVM_HOST.ABI,
					logs: receipt.logs,
					eventName: "PostRequestEvent",
				})
				assert(dispatched, "PostRequestEvent missing from the cancel receipt")
				const post = dispatched.args
				assert.equal(post.source, AMOY)
				assert.equal(post.dest, CHAPEL)
				assert(isAddressEqual(post.from, amoy.gateway))
				assert.equal(post.to.toLowerCase(), chapel.gateway.toLowerCase())
				assert.equal(post.fee, cancelOptions.relayerFee)
				assert.equal(hexToNumber(sliceHex(post.body, 0, 1)), RequestKind.RefundEscrow)
				const [refund] = decodeAbiParameters(WITHDRAWAL_REQUEST, sliceHex(post.body, 1))
				assert.equal(refund.commitment, placed.id)
				assert.equal(refund.beneficiary.toLowerCase(), placed.user.toLowerCase())
				assert.equal(refund.tokens.length, 1)
				assert.equal(refund.tokens[0].token.toLowerCase(), placed.inputs[0].token.toLowerCase())
				assert.equal(refund.tokens[0].amount, escrowed)

				const { commitment: postCommitment } = postRequestCommitment({
					source: post.source,
					dest: post.dest,
					from: post.from,
					to: post.to,
					nonce: post.nonce,
					body: post.body,
					timeoutTimestamp: post.timeoutTimestamp,
				})
				console.log(`[cancel] refund POST ${postCommitment}`)

				assert.equal(await gateway.isOrderFilled(placed), true, "Amoy does not report the cancelled order")
				const finalizer = await amoy.evm.client.readContract({
					address: amoy.gateway,
					abi: IntentGatewayV2ABI,
					functionName: "_filled",
					args: [placed.id],
				})
				assert(isAddressEqual(finalizer, account.address), `Amoy froze the order to ${finalizer}`)

				const finalizedEarly = await awaitHyperbridgeDelivery(
					stream,
					hyperbridge,
					postCommitment,
					Date.now() + DELIVERED_TIMEOUT_MS,
				)
				assert(
					await hyperbridge.queryRequestReceipt(postCommitment),
					"Hyperbridge holds no refund POST receipt",
				)
				assert.equal(
					await readLegEscrow(chapel.evm.client, chapel.gateway, placed.id, 0, placed.inputs[0].token),
					escrowed,
					"Chapel released the escrow before the refund reached it",
				)
				assert.equal(await gateway.isOrderRefunded(placed), false)

				if (!FULL_REFUND) return

				const refundDeadline = Date.now() + REFUND_TIMEOUT_MS
				const finalized = finalizedEarly ?? (await advanceTo(stream, "HYPERBRIDGE_FINALIZED", refundDeadline))
				const { handler } = await chapel.evm.client.readContract({
					address: chapel.host,
					abi: EVM_HOST.ABI,
					functionName: "hostParams",
				})
				const deliveryHash = await chapel.wallet.sendTransaction({
					to: handler,
					data: finalized.metadata.calldata,
				})
				console.log(`[cancel] refund delivery tx ${deliveryHash} on Chapel`)
				const delivery = await chapel.evm.client.waitForTransactionReceipt({ hash: deliveryHash })
				assert.equal(delivery.status, "success", "Refund delivery reverted on the Chapel handler")
				assert(
					await chapel.evm.queryRequestReceipt(postCommitment),
					"The Chapel host did not accept the refund POST; the gateway's onAccept failed",
				)

				const complete = await advanceTo(stream, "CANCELLATION_COMPLETE", refundDeadline)
				assert.equal(complete.transactionHash.toLowerCase(), deliveryHash.toLowerCase())
				assert.equal(await gateway.isOrderRefunded(placed), true, "Chapel still holds the escrow")
			} finally {
				stream?.close()
				await hyperbridge.disconnect()
			}
		},
		SETUP_TIMEOUT_MS + DELIVERED_TIMEOUT_MS + (FULL_REFUND ? REFUND_TIMEOUT_MS : 0),
	)
})

interface LiveChain {
	evm: EvmChain
	wallet: WalletClient<Transport, Chain, Account>
	gateway: HexString
	host: HexString
}

type CancelStatus = CancelEvent["status"]
type CancelEventOf<S extends CancelStatus> = Extract<CancelEvent, { status: S }>

const TIMED_OUT = Symbol("timed out")

/**
 * Reads the cancel stream against deadlines. A read that times out stays pending, so the next read
 * returns its event instead of dropping it.
 */
class CancelStream {
	private pending?: Promise<IteratorResult<CancelEvent>>

	constructor(private readonly events: AsyncGenerator<CancelEvent>) {}

	async next(deadline: number, input?: HexString): Promise<CancelEvent | typeof TIMED_OUT> {
		if (!this.pending) {
			this.pending = this.events.next(input)
			this.pending.catch(() => {})
		}
		let timer: NodeJS.Timeout | undefined
		const expiry = new Promise<typeof TIMED_OUT>((resolve) => {
			timer = setTimeout(() => resolve(TIMED_OUT), Math.max(0, deadline - Date.now()))
		})
		try {
			const step = await Promise.race([this.pending, expiry])
			if (step === TIMED_OUT) return TIMED_OUT
			this.pending = undefined
			if (step.done) throw new Error("The cancel stream ended early")
			console.log(`[cancel] ${step.value.status}`)
			return step.value
		} finally {
			clearTimeout(timer)
		}
	}

	close(): void {
		void this.events.return(undefined).catch(() => {})
	}
}

async function advanceTo<S extends CancelStatus>(
	stream: CancelStream,
	status: S,
	deadline: number,
	input?: HexString,
): Promise<CancelEventOf<S>> {
	let send = input
	while (true) {
		const event = await stream.next(deadline, send)
		send = undefined
		if (event === TIMED_OUT) throw new Error(`Timed out waiting for ${status}`)
		if (event.status === status) return event as CancelEventOf<S>
	}
}

/**
 * Waits until Hyperbridge holds the refund POST. The status stream starts from the latest status the
 * indexer has, so a POST it first sees already delivered never yields HYPERBRIDGE_DELIVERED; Hyperbridge's
 * request receipt is read directly between stream reads. A HYPERBRIDGE_FINALIZED read on the way is
 * returned so its calldata is not lost.
 */
async function awaitHyperbridgeDelivery(
	stream: CancelStream,
	hyperbridge: SubstrateChain,
	commitment: HexString,
	deadline: number,
): Promise<CancelEventOf<"HYPERBRIDGE_FINALIZED"> | undefined> {
	while (Date.now() < deadline) {
		const event = await stream.next(Math.min(deadline, Date.now() + RECEIPT_POLL_MS))
		if (event !== TIMED_OUT) {
			if (event.status === "HYPERBRIDGE_DELIVERED") return undefined
			if (event.status === "HYPERBRIDGE_FINALIZED") return event
		}
		if (await hyperbridge.queryRequestReceipt(commitment)) return undefined
	}
	throw new Error(
		`Refund POST ${commitment} did not reach Hyperbridge within ${DELIVERED_TIMEOUT_MS / 60_000} minutes`,
	)
}

function readEnv() {
	const required = ["PRIVATE_KEY", "BSC_CHAPEL", "POLYGON_AMOY", "HYPERBRIDGE_GARGANTUA"] as const
	const missing = required.filter((name) => !process.env[name])
	if (missing.length > 0) throw new Error(`The live cancel test needs ${missing.join(", ")}`)

	const privateKey = process.env.PRIVATE_KEY as string
	return {
		privateKey: (privateKey.startsWith("0x") ? privateKey : `0x${privateKey}`) as HexString,
		chapelRpc: process.env.BSC_CHAPEL as string,
		amoyRpc: process.env.POLYGON_AMOY as string,
		hyperbridgeWs: process.env.HYPERBRIDGE_GARGANTUA as string,
		indexerUrl: process.env.GARGANTUA_INDEXER_URL || "https://gargantua.indexer.polytope.technology",
	}
}

function liveChain(
	id: string,
	chain: Chain,
	rpcUrl: string,
	account: Account,
	configService: ChainConfigService,
): LiveChain {
	const host = configService.getHostAddress(id)
	return {
		evm: EvmChain.fromParams({ chainId: chain.id, host, rpcUrl }),
		wallet: createWalletClient({ account, chain, transport: http(rpcUrl) }),
		gateway: configService.getIntentGatewayAddress(id),
		host,
	}
}

async function approveGateway(chain: LiveChain, token: HexString): Promise<void> {
	const allowance = await chain.evm.client.readContract({
		address: token,
		abi: erc20Abi,
		functionName: "allowance",
		args: [chain.wallet.account.address, chain.gateway],
	})
	if (allowance >= maxUint256 / 2n) return

	const hash = await chain.wallet.writeContract({
		address: token,
		abi: erc20Abi,
		functionName: "approve",
		args: [chain.gateway, maxUint256],
	})
	const receipt = await chain.evm.client.waitForTransactionReceipt({ hash })
	assert.equal(receipt.status, "success", `Approving ${token} for ${chain.gateway} reverted`)
	console.log(`[approve] ${token} for ${chain.gateway}`)
}

async function signTransaction(
	chain: LiveChain,
	tx: { to: HexString; data: HexString; value: bigint },
): Promise<HexString> {
	const request = await chain.wallet.prepareTransactionRequest({ to: tx.to, data: tx.data, value: tx.value })
	return chain.wallet.signTransaction(request)
}

async function placeOrder(
	gateway: IntentGateway,
	chapel: LiveChain,
	order: Order,
	feeToken: HexString,
): Promise<Order & { id: HexString }> {
	const placement = gateway.execute(order, DEFAULT_GRAFFITI, { auctionTimeMs: 1 })
	try {
		const awaiting = await placement.next()
		if (awaiting.done || awaiting.value.status !== "AWAITING_PLACE_ORDER") {
			throw new Error("Expected AWAITING_PLACE_ORDER first")
		}
		assert.equal(awaiting.value.value, 0n)
		assert.equal(awaiting.value.feeTokenAmount, order.fees)
		assert(isAddressEqual(awaiting.value.feeTokenAddress, feeToken))

		const placedUpdate = await placement.next(await signTransaction(chapel, awaiting.value))
		if (placedUpdate.done || placedUpdate.value.status !== "ORDER_PLACED") {
			throw new Error("Expected ORDER_PLACED after the placement transaction")
		}
		const { order: placed, receipt } = placedUpdate.value
		assert(placed.id, "ORDER_PLACED carries no order id")
		console.log(`[place] order ${placed.id} in tx ${receipt.transactionHash} on Chapel`)
		return { ...placed, id: placed.id as HexString }
	} finally {
		await placement.return(undefined)
	}
}
