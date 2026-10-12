import "log-timestamp"

import { strict as assert } from "node:assert"
import { describe, it } from "vitest"
import {
	type Account,
	type Chain,
	type TransactionReceipt,
	type Transport,
	type WalletClient,
	concatHex,
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
import { partialFillSlot, readLegEscrow } from "@/protocols/intents/escrowReads"
import { type CancelEvent, DEFAULT_GRAFFITI } from "@/protocols/intents/types"
import { createQueryClient } from "@/queryClient"
import { type HexString, type Order, RequestKind, RequestStatus } from "@/types"
import { ADDRESS_ZERO, bytes20ToBytes32, getRequestCommitment, postRequestCommitment, sleep } from "@/utils"

/**
 * Live cross-chain cancellations between Polygon Amoy and BSC Chapel. Each test places an order no
 * solver can fill and runs its cancellation until the escrow is refunded on the order's source chain.
 *
 * From source: an Amoy to Chapel order expires on Chapel and is cancelled on Amoy. The SDK proves the
 * order's Chapel fill progress with a GET and delivers it to Hyperbridge; the test delivers the GET
 * response to the Amoy handler, which refunds the escrow.
 *
 * From destination: a Chapel to Amoy order is cancelled on Amoy, which freezes it there and posts
 * RefundEscrow back to Chapel. The test delivers that POST to the Chapel handler, which refunds the
 * escrow.
 *
 * The testnet relayer delivers Hyperbridge consensus updates to the EVM hosts but not messages, so each
 * test delivers the HYPERBRIDGE_FINALIZED calldata itself. When the host has not yet seen the Hyperbridge
 * height the message needs, that calldata batches the Hyperbridge consensus proofs the host is missing
 * with it. Both tests use the same wallet and run in order.
 *
 * Needs PRIVATE_KEY, BSC_CHAPEL, POLYGON_AMOY and HYPERBRIDGE_GARGANTUA. GARGANTUA_INDEXER_URL
 * overrides the public gargantua indexer.
 */

const CHAPEL = "EVM-97"
const AMOY = "EVM-80002"

const EXPIRED_ORDER_LIFETIME_BLOCKS = 40n
const OPEN_ORDER_LIFETIME_BLOCKS = 43_200n

const SETUP_TIMEOUT_MS = 10 * 60_000
const DESTINATION_PROOF_TIMEOUT_MS = 25 * 60_000
const CANCEL_TIMEOUT_MS = 5 * 60_000
const DELIVERED_TIMEOUT_MS = 15 * 60_000
const FINALIZED_TIMEOUT_MS = 40 * 60_000
const DELIVERY_TIMEOUT_MS = 5 * 60_000
const INDEXED_TIMEOUT_MS = 5 * 60_000

const FROM_SOURCE_TIMEOUT_MS =
	SETUP_TIMEOUT_MS +
	DESTINATION_PROOF_TIMEOUT_MS +
	CANCEL_TIMEOUT_MS +
	FINALIZED_TIMEOUT_MS +
	DELIVERY_TIMEOUT_MS +
	INDEXED_TIMEOUT_MS
const FROM_DESTINATION_TIMEOUT_MS =
	SETUP_TIMEOUT_MS + DELIVERED_TIMEOUT_MS + FINALIZED_TIMEOUT_MS + DELIVERY_TIMEOUT_MS + INDEXED_TIMEOUT_MS

const RECEIPT_POLL_MS = 15_000
const INDEXER_POLL_MS = 10_000

// The host swallows a failed app callback, so the estimate alone can starve it under the 63/64 rule.
const DELIVERY_GAS_HEADROOM = 2_000_000n
// Per-transaction gas cap on Chapel; Amoy allows 2^25.
const MAX_TX_GAS = 16_777_216n

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

describe("IntentGateway cross-chain cancellation between Polygon Amoy and BSC Chapel (live)", () => {
	it(
		"cancels an expired Amoy to Chapel order from its source and refunds its escrow on Amoy",
		async () => {
			const env = readEnv()
			const { account, configService, chapel, amoy, hyperbridge, ismpClient } = await connect(env)
			let stream: CancelStream | undefined

			try {
				const amoyFeeToken = await amoy.evm.getFeeTokenWithDecimals()
				await approveGateway(amoy, configService.getUsdcAsset(AMOY))
				await approveGateway(amoy, amoyFeeToken.address)

				const gateway = await IntentGateway.create(amoy.evm, chapel.evm)
				const { order: placed, receipt: placement } = await placeOrder(
					gateway,
					amoy,
					unfillableOrder(
						configService,
						account,
						AMOY,
						CHAPEL,
						parseUnits("0.01", amoyFeeToken.decimals),
						(await chapel.evm.client.getBlockNumber()) + EXPIRED_ORDER_LIFETIME_BLOCKS,
					),
					amoyFeeToken.address,
				)
				const escrowed = await assertOpen(gateway, amoy, placed)

				stream = new CancelStream(gateway.cancelOrder(placed, ismpClient, { from: "source" }))

				const { proof } = await advanceTo(
					stream,
					"DESTINATION_FINALIZED",
					Date.now() + DESTINATION_PROOF_TIMEOUT_MS,
				)
				assert.equal(proof.stateMachine, CHAPEL, "The destination proof is not for Chapel")
				assert(proof.height > placed.deadline, `Proof height ${proof.height} is not past the deadline`)

				const cancelDeadline = Date.now() + CANCEL_TIMEOUT_MS
				const cancelTx = await advanceTo(stream, "AWAITING_CANCEL_TRANSACTION", cancelDeadline)
				const cancelOptions = assertCancelTransaction(cancelTx, amoy.gateway)
				assert.equal(cancelOptions.height, proof.height, "The cancel does not carry the proof height")

				const { receipt } = await advanceTo(
					stream,
					"CANCEL_STARTED",
					cancelDeadline,
					await signTransaction(amoy, cancelTx),
				)
				console.log(`[cancel] cancel tx ${receipt.transactionHash} on Amoy`)
				assert.equal(receipt.status, "success", "Cancel transaction reverted")
				assertOrderCancelled(receipt, placed.id, account.address)

				const [dispatched] = parseEventLogs({
					abi: EVM_HOST.ABI,
					logs: receipt.logs,
					eventName: "GetRequestEvent",
				})
				assert(dispatched, "GetRequestEvent missing from the cancel receipt")
				const get = dispatched.args
				assert.equal(get.source, AMOY)
				assert.equal(get.dest, CHAPEL)
				assert.equal(get.from.toLowerCase(), amoy.gateway.toLowerCase())
				assert.equal(get.height, proof.height)
				assert.deepEqual(
					get.keys.map((key) => key.toLowerCase()),
					[concatHex([chapel.gateway, partialFillSlot(placed.id, 0)]).toLowerCase()],
					"The GET does not read the Chapel _partialFills slot of leg 0",
				)
				assert.equal(get.fee, cancelOptions.relayerFee)

				const getCommitment = getRequestCommitment({
					source: get.source,
					dest: get.dest,
					from: get.from,
					nonce: get.nonce,
					height: get.height,
					keys: [...get.keys],
					timeoutTimestamp: get.timeoutTimestamp,
					context: get.context,
				})
				console.log(`[cancel] GET ${getCommitment}`)

				const finalized = await advanceTo(stream, "HYPERBRIDGE_FINALIZED", Date.now() + FINALIZED_TIMEOUT_MS)
				assert(
					await hyperbridge.queryResponseReceipt(getCommitment),
					"Hyperbridge holds no response receipt for the GET",
				)
				assert.equal(
					await readLegEscrow(amoy.evm.client, amoy.gateway, placed.id, 0, placed.inputs[0].token),
					escrowed,
					"Amoy released the escrow before the GET response reached it",
				)
				assert.equal(await gateway.isOrderRefunded(placed), false)

				const delivery = await deliverToHandler(amoy, finalized.metadata.calldata)
				const handled = parseEventLogs({
					abi: EVM_HOST.ABI,
					logs: delivery.logs,
					eventName: "GetRequestHandled",
				}).find((log) => log.args.commitment === getCommitment)
				assert(handled, "GetRequestHandled missing from the delivery; the gateway's onGetResponse failed")
				assertEscrowRefunded(delivery, placed.id)
				assertRefundMatchesPlacement(placement, delivery, amoy.gateway, account.address)
				assert.equal(
					await readLegEscrow(amoy.evm.client, amoy.gateway, placed.id, 0, placed.inputs[0].token),
					0n,
					"Amoy still holds the leg escrow",
				)
				assert.equal(await gateway.isOrderRefunded(placed), true, "Amoy does not report the order refunded")

				await awaitIndexed(`cancel GET ${getCommitment} with its response`, env.indexerUrl, async () => {
					const statuses = indexedStatuses(await ismpClient.queryGetRequest(getCommitment))
					const response = await ismpClient.queryResponseByRequestId(getCommitment)
					return {
						done:
							statuses.includes(RequestStatus.HYPERBRIDGE_DELIVERED) &&
							statuses.includes(RequestStatus.DESTINATION) &&
							response !== undefined,
						indexed: `statuses ${statuses.join(", ") || "none"}; response ${response?.commitment ?? "none"}`,
					}
				})
			} finally {
				stream?.close()
				await hyperbridge.disconnect()
			}
		},
		FROM_SOURCE_TIMEOUT_MS,
	)

	it(
		"cancels a Chapel to Amoy order from its destination and refunds its escrow on Chapel",
		async () => {
			const env = readEnv()
			const { account, configService, chapel, amoy, hyperbridge, ismpClient } = await connect(env)
			let stream: CancelStream | undefined

			try {
				const chapelFeeToken = await chapel.evm.getFeeTokenWithDecimals()
				const amoyFeeToken = await amoy.evm.getFeeTokenWithDecimals()
				await approveGateway(chapel, configService.getUsdcAsset(CHAPEL))
				await approveGateway(chapel, chapelFeeToken.address)
				await approveGateway(amoy, amoyFeeToken.address)

				const gateway = await IntentGateway.create(chapel.evm, amoy.evm)
				const { order: placed, receipt: placement } = await placeOrder(
					gateway,
					chapel,
					unfillableOrder(
						configService,
						account,
						CHAPEL,
						AMOY,
						parseUnits("0.01", chapelFeeToken.decimals),
						(await amoy.evm.client.getBlockNumber()) + OPEN_ORDER_LIFETIME_BLOCKS,
					),
					chapelFeeToken.address,
				)
				const escrowed = await assertOpen(gateway, chapel, placed)

				stream = new CancelStream(gateway.cancelOrder(placed, ismpClient, { from: "destination" }))

				const deliveredDeadline = Date.now() + DELIVERED_TIMEOUT_MS
				const cancelTx = await advanceTo(stream, "AWAITING_CANCEL_TRANSACTION", deliveredDeadline)
				const cancelOptions = assertCancelTransaction(cancelTx, amoy.gateway)
				assert.equal(cancelOptions.height, 0n, "A destination cancel needs no proof height")

				const { receipt } = await advanceTo(
					stream,
					"CANCEL_STARTED",
					deliveredDeadline,
					await signTransaction(amoy, cancelTx),
				)
				console.log(`[cancel] cancel tx ${receipt.transactionHash} on Amoy`)
				assert.equal(receipt.status, "success", "Cancel transaction reverted")
				assertOrderCancelled(receipt, placed.id, account.address)

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
					deliveredDeadline,
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

				const finalized =
					finalizedEarly ??
					(await advanceTo(stream, "HYPERBRIDGE_FINALIZED", Date.now() + FINALIZED_TIMEOUT_MS))
				const delivery = await deliverToHandler(chapel, finalized.metadata.calldata)
				assert(
					await chapel.evm.queryRequestReceipt(postCommitment),
					"The Chapel host did not accept the refund POST; the gateway's onAccept failed",
				)
				assertEscrowRefunded(delivery, placed.id)
				assertRefundMatchesPlacement(placement, delivery, chapel.gateway, account.address)

				const complete = await advanceTo(stream, "CANCELLATION_COMPLETE", Date.now() + INDEXED_TIMEOUT_MS)
				assert.equal(complete.transactionHash.toLowerCase(), delivery.transactionHash.toLowerCase())
				assert.equal(await gateway.isOrderRefunded(placed), true, "Chapel still holds the escrow")
			} finally {
				stream?.close()
				await hyperbridge.disconnect()
			}
		},
		FROM_DESTINATION_TIMEOUT_MS,
	)
})

interface LiveChain {
	id: string
	evm: EvmChain
	wallet: WalletClient<Transport, Chain, Account>
	gateway: HexString
	host: HexString
}

type PlacedOrder = Order & { id: HexString }
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

/**
 * Polls the indexer until `probe` reports done. The source cancel stream ends at HYPERBRIDGE_FINALIZED,
 * before the test delivers the GET response, so this is what checks the indexer records that delivery.
 */
async function awaitIndexed(
	what: string,
	indexerUrl: string,
	probe: () => Promise<{ done: boolean; indexed: string }>,
): Promise<void> {
	const deadline = Date.now() + INDEXED_TIMEOUT_MS
	let indexed = "unknown"
	let lastError: string | undefined
	while (true) {
		try {
			const result = await probe()
			if (result.done) return
			indexed = result.indexed
		} catch (e) {
			lastError = e instanceof Error ? e.message : String(e)
		}
		if (Date.now() >= deadline) {
			throw new Error(
				`The indexer at ${indexerUrl} did not record ${what} within ${INDEXED_TIMEOUT_MS / 60_000} minutes (indexed: ${indexed}${lastError ? `; last query error: ${lastError}` : ""})`,
			)
		}
		await sleep(INDEXER_POLL_MS)
	}
}

function indexedStatuses(request: { statuses: Array<{ status: string }> } | undefined): string[] {
	return request?.statuses.map(({ status }) => status) ?? []
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

/** Both flows track requests from Amoy to Chapel: the cancel GET and the refund POST. */
async function connect(env: ReturnType<typeof readEnv>) {
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
	const ismpClient = new IsmpClient({
		queryClient: createQueryClient({ url: env.indexerUrl }),
		source: amoy.evm,
		dest: chapel.evm,
		hyperbridge,
		pollInterval: 5_000,
	})
	return { account, configService, chapel, amoy, hyperbridge, ismpClient }
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
		id,
		evm: EvmChain.fromParams({ chainId: chain.id, host, rpcUrl }),
		wallet: createWalletClient({ account, chain, transport: http(rpcUrl) }),
		gateway: configService.getIntentGatewayAddress(id),
		host,
	}
}

/** 0.01 USDC on the source for 1,000,000 USDC on the destination, so no solver can fill it. */
function unfillableOrder(
	configService: ChainConfigService,
	account: Account,
	source: string,
	destination: string,
	fees: bigint,
	deadline: bigint,
): Order {
	const user = bytes20ToBytes32(account.address)
	return {
		user,
		source,
		destination,
		deadline,
		nonce: 0n,
		fees,
		session: ADDRESS_ZERO,
		predispatch: { assets: [], call: "0x" },
		inputs: [
			{
				token: bytes20ToBytes32(configService.getUsdcAsset(source)),
				amount: parseUnits("0.01", configService.getUsdcDecimals(source)),
			},
		],
		output: {
			beneficiary: user,
			assets: [
				{
					token: bytes20ToBytes32(configService.getUsdcAsset(destination)),
					amount: parseUnits("1000000", configService.getUsdcDecimals(destination)),
				},
			],
			call: "0x",
		},
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
	console.log(`[approve] ${token} for ${chain.gateway} on ${chain.id}`)
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
	chain: LiveChain,
	order: Order,
	feeToken: HexString,
): Promise<{ order: PlacedOrder; receipt: TransactionReceipt }> {
	const placement = gateway.execute(order, DEFAULT_GRAFFITI, { auctionTimeMs: 1 })
	try {
		const awaiting = await placement.next()
		if (awaiting.done || awaiting.value.status !== "AWAITING_PLACE_ORDER") {
			throw new Error("Expected AWAITING_PLACE_ORDER first")
		}
		assert.equal(awaiting.value.value, 0n)
		assert.equal(awaiting.value.feeTokenAmount, order.fees)
		assert(isAddressEqual(awaiting.value.feeTokenAddress, feeToken))

		const placedUpdate = await placement.next(await signTransaction(chain, awaiting.value))
		if (placedUpdate.done || placedUpdate.value.status !== "ORDER_PLACED") {
			throw new Error("Expected ORDER_PLACED after the placement transaction")
		}
		const { order: placed, receipt } = placedUpdate.value
		assert(placed.id, "ORDER_PLACED carries no order id")
		console.log(`[place] order ${placed.id} in tx ${receipt.transactionHash} on ${chain.id}`)
		return { order: { ...placed, id: placed.id as HexString }, receipt }
	} finally {
		await placement.return(undefined)
	}
}

/** Checks a new order is escrowed on its source chain and neither filled nor refunded. */
async function assertOpen(gateway: IntentGateway, source: LiveChain, placed: PlacedOrder): Promise<bigint> {
	const escrowed = await readLegEscrow(source.evm.client, source.gateway, placed.id, 0, placed.inputs[0].token)
	assert.equal(escrowed, placed.inputs[0].amount, `${source.id} escrow differs from the committed input`)
	assert.equal(await gateway.isOrderFilled(placed), false, "A new order already reads as filled")
	assert.equal(await gateway.isOrderRefunded(placed), false, "A new order already reads as refunded")
	return escrowed
}

function assertCancelTransaction(tx: CancelEventOf<"AWAITING_CANCEL_TRANSACTION">, gateway: HexString) {
	assert(isAddressEqual(tx.to, gateway), `Cancel targets ${tx.to}, not the gateway ${gateway}`)
	assert.equal(tx.value, 0n, "Testnet hosts have no swap route, so the cancel must carry no value")
	const call = decodeFunctionData({ abi: IntentGatewayV2ABI, data: tx.data })
	if (call.functionName !== "cancelOrder") throw new Error(`Cancel calldata calls ${call.functionName}`)
	const [, options] = call.args
	assert(options.relayerFee > 0n, "Cancel carries no relayer fee")
	return options
}

function assertOrderCancelled(receipt: TransactionReceipt, orderId: HexString, canceller: HexString): void {
	const [cancelled] = parseEventLogs({
		abi: IntentGatewayV2ABI,
		logs: receipt.logs,
		eventName: "OrderCancelled",
	})
	assert(cancelled, "OrderCancelled missing from the cancel receipt")
	assert.equal(cancelled.args.commitment, orderId)
	assert(isAddressEqual(cancelled.args.canceller, canceller))
}

function assertEscrowRefunded(receipt: TransactionReceipt, orderId: HexString): void {
	const refunded = parseEventLogs({
		abi: IntentGatewayV2ABI,
		logs: receipt.logs,
		eventName: "EscrowRefunded",
	}).find((log) => log.args.commitment === orderId)
	assert(refunded, `EscrowRefunded missing from the delivery for order ${orderId}`)
}

/**
 * Per token, the gateway must pay the user back what the user paid it at placement: the escrow, its
 * protocol fee and the order fees. Transfers are compared rather than balances, since other jobs spend
 * from the same wallet.
 */
function assertRefundMatchesPlacement(
	placement: TransactionReceipt,
	refund: TransactionReceipt,
	gateway: HexString,
	user: HexString,
): void {
	const paid = transferTotals(placement, user, gateway)
	assert(paid.size > 0, "The placement moved no tokens to the gateway")
	assert.deepEqual(transferTotals(refund, gateway, user), paid, "The refund differs from what the placement paid")
}

function transferTotals(receipt: TransactionReceipt, from: HexString, to: HexString): Map<string, bigint> {
	const totals = new Map<string, bigint>()
	for (const log of parseEventLogs({ abi: erc20Abi, logs: receipt.logs, eventName: "Transfer" })) {
		if (!isAddressEqual(log.args.from, from) || !isAddressEqual(log.args.to, to)) continue
		const token = log.address.toLowerCase()
		totals.set(token, (totals.get(token) ?? 0n) + log.args.value)
	}
	return totals
}

async function deliverToHandler(chain: LiveChain, calldata: HexString): Promise<TransactionReceipt> {
	const { handler } = await chain.evm.client.readContract({
		address: chain.host,
		abi: EVM_HOST.ABI,
		functionName: "hostParams",
	})
	const estimate = await chain.evm.client.estimateGas({
		account: chain.wallet.account,
		to: handler,
		data: calldata,
	})
	const padded = estimate + DELIVERY_GAS_HEADROOM
	const gas = padded < MAX_TX_GAS ? padded : MAX_TX_GAS
	const hash = await chain.wallet.sendTransaction({ to: handler, data: calldata, gas })
	console.log(`[deliver] tx ${hash} to the ${chain.id} handler`)
	const receipt = await chain.evm.client.waitForTransactionReceipt({ hash })
	assert.equal(receipt.status, "success", `Delivery to the ${chain.id} handler reverted`)
	return receipt
}
