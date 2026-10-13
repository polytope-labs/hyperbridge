import {
	bytes20ToBytes32,
	type Bid,
	type DecodedOrderPlacedLog,
	deriveCanonicalPlacedOrder,
	IntentGatewayABI,
	createQueryClient,
	EvmChain,
	getChainId,
	HyperFungibleTokenABI,
	IntentGateway,
	IsmpClient,
	SubstrateChain,
	type ERC7821Call,
	type HexString,
	type IntentsCoprocessor,
	type Order,
} from "@hyperbridge/sdk"
import { randomUUID } from "node:crypto"
import { setTimeout as sleep } from "node:timers/promises"
import {
	type AbiEvent,
	createPublicClient,
	encodeAbiParameters,
	encodeFunctionData,
	fallback,
	formatUnits,
	http,
	parseEventLogs,
	parseUnits,
	type PublicClient,
	stringToHex,
	toHex,
} from "viem"
import { bsc as bscChain } from "viem/chains"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { hyperbridgeBundlerUrl } from "@/config/bundlers"
import { INTENT_GATEWAY_V2_ABI } from "@/config/abis/IntentGatewayV2"
import { EVM_HOST } from "@/config/abis/EvmHost"
import { chainByChainId } from "@/cli/init/chains"
import { patchRuntimeState } from "@/data/state"
import type {
	DeliveryStatus,
	RecoveryStepRecord,
	StateStore,
	StoredBridgeTopUp,
	TopUpBidPreview,
	TopUpExecutionEvent,
} from "@/data/types"
import type { ChainClientManager } from "./ChainClientManager"
import type { DelegationService } from "./DelegationService"
import type { FillerConfigService } from "./FillerConfigService"
import { moduleLogger, type Logger } from "./Logger"
import { paymasterReserveForToken } from "./paymaster"
import type { TokenSender } from "./TokenSender"

/**
 * The order's referrer: `placeOrder`'s graffiti, emitted in `OrderPlaced` and indexed as the
 * order's `referrer`. "Simplex" as UTF-8, right-padded to bytes32, as the HyperFX app tags its
 * own orders "HyperFX".
 */
export const SIMPLEX_REFERRER = stringToHex("Simplex", { size: 32 })

/** The chain BRIDGE is bought on: the only chain with BRIDGE books on the orderbook. */
export const BRIDGE_TOP_UP_CHAIN = "EVM-56"
/** BridgeToken on BSC, used when the SDK asset table does not name it. */
const BRIDGE_TOKEN_BSC = "0x5b0c50fDd52ECC0d4c682c441eaBaD41FfDEABBB" as HexString
const BRIDGE_DECIMALS = 18
/** Hyperbridge (nexus), where the HFT pallet credits the filler's native BRIDGE balance. */
const NEXUS_STATE_MACHINE = "POLKADOT-3367"
const NEXUS_CONSENSUS_STATE = "DOT0"
const NEXUS_WS_URL = "wss://nexus.rpc.polytope.technology"
/**
 * Nexus holds BRIDGE at 12 decimals and truncates the EVM amount when crediting it, so the
 * bought amount is rounded down to a whole nexus unit and nothing is lost in conversion.
 */
const NEXUS_UNIT = 10n ** 6n
/** Output kept below the quote so a bid still clears after a small price move. */
const QUOTE_SLIPPAGE_BPS = 100n
/** How long the order may wait for a fill, in seconds of BSC blocks. */
const ORDER_WINDOW_SECONDS = 30 * 60
const AUCTION_TIME_MS = 15_000
/** How often a cross-chain reclaim checks the source escrow after Hyperbridge finalizes it. */
const REFUND_POLL_MS = 10_000
const RECEIPT_TIMEOUT_MS = 120_000
const BSC_CONSENSUS_STATE = "BSC0"
/** Execution events kept per top-up: an order sees a handful of bids, not hundreds. */
const MAX_EVENTS = 50
const DELIVERY_POLL_MS = 15_000
const DELIVERY_RETRY_MS = 60_000
/**
 * Paid relayers deliver the zero-fee request to Hyperbridge when they get to it, which has
 * taken a minute and has also not happened at all. Once BSC is final on Hyperbridge the
 * filler gives them this long, then relays it itself.
 */
const SELF_RELAY_GRACE_MS = 90_000
/** How long a relayer has to deliver a recovery's refund before the filler claims it, as in the Hyperbridge app. */
const RELAYER_DELIVERY_WAIT_MS = 3 * 60_000
/** How long after delivery the request's last statuses are still read for the timeline. */
const STATUS_BACKFILL_MS = 30 * 60_000
/** Every chain, indexer and Hyperbridge read is bounded, so a stalled endpoint cannot hang a step. */
const NETWORK_TIMEOUT_MS = 30_000
/** Public BSC endpoints serve logs over a few hundred blocks at a time; the search stays near the fill. */
const LOG_CHUNK_BLOCKS = 300n
const FILL_SEARCH_BEFORE_MS = 3 * 60_000
const FILL_SEARCH_AFTER_MS = 60_000
const ORDER_FILLED_EVENT = IntentGatewayABI.find(
	(item) => item.type === "event" && item.name === "OrderFilled",
) as AbiEvent
const DELIVERY_REQUEST_QUERY = `query TopUpDelivery($hash: String!) {
	requestV2s(
		filter: {
			source: { equalTo: "EVM-56" }
			dest: { equalTo: "POLKADOT-3367" }
			statusMetadata: { some: { transactionHash: { equalTo: $hash } } }
		}
	) {
		nodes { commitment }
	}
}`
const ORDER_QUERY = `query TopUpOrder($commitment: String!) {
	iOrderV3s(filter: { commitment: { equalTo: $commitment } }) {
		nodes {
			status
			statusMetadata { nodes { status timestamp transactionHash filler } }
		}
	}
}`

const CALL_TUPLE = [
	{
		type: "tuple[]",
		components: [
			{ name: "to", type: "address" },
			{ name: "value", type: "uint256" },
			{ name: "data", type: "bytes" },
		],
	},
] as const

export type TopUpPayToken = "USDC" | "USDT"

/**
 * - `placing`: the placement batch is being sent.
 * - `awaiting_bids` / `filling`: the order is open on the gateway.
 * - `delivering`: filled on BSC; the BRIDGE is being relayed to Hyperbridge.
 * - `delivered`: the BRIDGE reached the filler's Hyperbridge account.
 * - `unfilled`: the order is past its auction without a fill; its input is still escrowed.
 * - `reclaiming` / `reclaimed`: the order is being, or has been, cancelled and refunded.
 * - `filled`: a record from before delivery was tracked; the BRIDGE was sent to Hyperbridge.
 * - `failed`: nothing was escrowed, or the outcome needs the operator's attention.
 */
export type BridgeTopUpStage =
	| "placing"
	| "awaiting_bids"
	| "filling"
	| "delivering"
	| "delivered"
	| "unfilled"
	| "reclaiming"
	| "reclaimed"
	| "filled"
	| "failed"

/** Stages that keep the operator from starting another top-up. */
const PENDING_STAGES: ReadonlySet<BridgeTopUpStage> = new Set([
	"placing",
	"awaiting_bids",
	"filling",
	"delivering",
	"unfilled",
	"reclaiming",
])

export function isBridgeTopUpPending(stage: BridgeTopUpStage): boolean {
	return PENDING_STAGES.has(stage)
}

export interface BridgeTopUpQuoteParams {
	/** Source state machine id the filler pays from, e.g. "EVM-8453". */
	chain: string
	token: TopUpPayToken
	/** Human-readable stablecoin amount. */
	amount: string
}

export interface BridgeTopUpQuote {
	chain: string
	token: TopUpPayToken
	tokenAddress: HexString
	amountIn: string
	/** BRIDGE the order requires, after slippage and nexus rounding. */
	bridgeOut: string
	/** BRIDGE per one stablecoin unit, from the quote. */
	rate: string
	fillable: boolean
	/** Largest stablecoin input the route can fill, human-readable. */
	maxFillableIn: string
	route: string
}

/** What the dashboard sees of a top-up: the stored record without its order or session key. */
export type BridgeTopUpState = Omit<StoredBridgeTopUp, "order" | "sessionPrivateKey"> & {
	/** True while the top-up blocks a new one. */
	pending: boolean
}

/** A stablecoin the filler wallet can pay a top-up with, on one network it fills on. */
export interface BridgeTopUpSource {
	chain: string
	chainId: number
	token: TopUpPayToken
	address: HexString
	/** Human-readable wallet balance, or null when the network could not be read. */
	balance: string | null
	/** What a top-up can spend: the balance less {@link reserve}. Null when unread. */
	spendable: string | null
	/** Held back for the paymaster, which pulls gas in this token. */
	reserve: string
}

export interface BridgeTopUpDeps {
	clientManager: ChainClientManager
	/**
	 * A top-up pays from a network the filler fills on: one whose account is delegated, so
	 * the placement batch runs through it with gas paid in stablecoins.
	 */
	delegation: Pick<DelegationService, "isDelegated">
	configService: FillerConfigService
	tokenSender: TokenSender
	/** Where the top-up record is kept, so an open order is tracked across restarts. */
	state: StateStore
	/** Directory for the SDK's order session keys and cancellation progress. */
	storagePath: string
	/** Indexer the cross-chain reclaim streams its GET request status from. */
	indexerUrl: string
	/** Filler EVM account that pays for the order. */
	solver: HexString
	/** sr25519 public key of the filler's Hyperbridge account, the BRIDGE recipient. */
	substratePublicKey: HexString
	/** Coprocessor the order's bids are read from. */
	hyperbridge?: Promise<IntentsCoprocessor>
	loggers?: Parameters<typeof moduleLogger>[0]
}

/**
 * Builds the `output.call` that forwards bought BRIDGE to Hyperbridge. The order pays its
 * output to the gateway's CallDispatcher, which runs this after the fill: HFT `send` burns
 * the dispatcher's BRIDGE and posts it to the nexus pallet, which credits `recipient`.
 *
 * `timeout` is 0 on purpose: a timed-out send refunds to its sender, the CallDispatcher,
 * which anyone can call. With no timeout there is no refund to strand there.
 */
export function encodeBridgeTopUpCall(bridgeToken: HexString, recipient: HexString, amount: bigint): HexString {
	const data = encodeFunctionData({
		abi: HyperFungibleTokenABI,
		functionName: "send",
		args: [
			{
				dest: toHex(NEXUS_STATE_MACHINE),
				to: recipient,
				amount,
				timeout: 0n,
				relayerFee: 0n,
				data: "0x",
			},
		],
	})
	return encodeAbiParameters(CALL_TUPLE, [[{ to: bridgeToken, value: 0n, data }]])
}

/** What the timeline shows of a bid: who offered, and what. Amounts are raw strings for JSON. */
function bidPreview(bid: Bid): TopUpBidPreview {
	return {
		solverAddress: bid.solverAddress,
		outputs: bid.outputs.map((output) => ({ token: output.token, amount: output.amount.toString() })),
	}
}

/** Output BRIDGE for a quoted amount: slippage taken off, rounded down to a whole nexus unit. */
export function bridgeTopUpOutput(quotedOut: bigint): bigint {
	const afterSlippage = (quotedOut * (10_000n - QUOTE_SLIPPAGE_BPS)) / 10_000n
	return (afterSlippage / NEXUS_UNIT) * NEXUS_UNIT
}

/** Orders carry bigints, which the JSON-encoded state store cannot hold. */
export function serializeOrder(order: Order): string {
	return JSON.stringify(order, (_key, value) => (typeof value === "bigint" ? { $bigint: value.toString() } : value))
}

export function deserializeOrder(json: string): Order {
	return JSON.parse(json, (_key, value) =>
		value && typeof value === "object" && typeof value.$bigint === "string" ? BigInt(value.$bigint) : value,
	) as Order
}

/**
 * Operator top-up of the filler's Hyperbridge BRIDGE balance. Places an IntentGateway order
 * from any running chain that sells USDC/USDT for BRIDGE filled on BSC, with an output call
 * that sends the BRIDGE to the filler's Hyperbridge account through the HyperFungibleToken.
 *
 * One top-up is open at a time. Its record, order and session key are persisted, so a
 * restart resumes tracking it, and an order that goes unfilled can be reclaimed.
 */
export class BridgeTopUp {
	private readonly logger: Logger
	private readonly gateways = new Map<string, Promise<IntentGateway>>()
	private dispatcher: Promise<HexString> | undefined
	private hyperbridgeChain: Promise<SubstrateChain> | undefined
	private record: StoredBridgeTopUp | undefined
	private loaded: Promise<void> | undefined
	/** Serializes state writes so a later stage never lands before an earlier one. */
	private writes: Promise<unknown> = Promise.resolve()
	private deliverySync: Promise<boolean> | undefined
	private bscClient: PublicClient | undefined
	private syncing: Promise<void> | undefined
	/**
	 * The top-up this process is driving, if any. Keyed by id: a finished top-up can still be
	 * reading its last statuses when the operator starts the next one, which must not wait.
	 */
	private driving: string | undefined

	constructor(private readonly deps: BridgeTopUpDeps) {
		this.logger = moduleLogger(deps.loggers, "bridge-top-up")
	}

	get bridgeToken(): HexString {
		return this.deps.configService.getAssetBySymbol(BRIDGE_TOP_UP_CHAIN, "BRIDGE") ?? BRIDGE_TOKEN_BSC
	}

	/** Loads the stored top-up and resumes driving it when it was left open. Never rejects. */
	restore(): Promise<void> {
		this.loaded ??= (async () => {
			try {
				this.record = (await this.deps.state.get()).bridgeTopUp
			} catch (err) {
				this.logger.warn({ err }, "Could not read the stored BRIDGE top-up")
				return
			}
			const record = this.record
			// A placement that failed only on reading its receipt back may still hold an order.
			if (record?.stage === "failed" && record.placeTxHash && !record.commitment && record.sessionPrivateKey) {
				void this.recoverPlacement()
				return
			}
			if (!record || !isBridgeTopUpPending(record.stage)) return
			if (record.stage === "placing" && !record.placeTxHash) {
				await this.update({
					stage: "failed",
					error: "Simplex restarted before the order was placed. Nothing was escrowed.",
				})
				return
			}
			if (record.stage === "placing") {
				// The batch was sent; its receipt says whether an order landed.
				void this.recoverPlacement()
				return
			}
			if (record.stage === "awaiting_bids" || record.stage === "filling") void this.resumeOrder()
			if (record.stage === "reclaiming") void this.driveReclaim()
			if (record.stage === "delivering") void this.resumeDelivery()
		})()
		return this.loaded
	}

	async current(): Promise<BridgeTopUpState | undefined> {
		await this.restore()
		return this.record ? publicState(this.record) : undefined
	}

	/** Networks the filler runs and has delegated its account on. */
	private async sourceChains(): Promise<number[]> {
		const chainIds = this.deps.configService.getConfiguredChainIds()
		const delegated = await Promise.all(
			chainIds.map((chainId) => this.deps.delegation.isDelegated(`EVM-${chainId}`).catch(() => false)),
		)
		return chainIds.filter((_, index) => delegated[index])
	}

	/** USDC and USDT wallet balances on every network a top-up can be paid from. */
	async sources(): Promise<BridgeTopUpSource[]> {
		const rows = (await this.sourceChains()).flatMap((chainId) =>
			(["USDC", "USDT"] as const).map(async (token): Promise<BridgeTopUpSource | undefined> => {
				const chain = `EVM-${chainId}`
				let tokenAddress: HexString
				let decimals: number
				try {
					;({ tokenAddress, decimals } = this.payToken(chain, token))
				} catch {
					return undefined
				}
				const reserve = this.reserve(chain, tokenAddress)
				const row = { chain, chainId, token, address: tokenAddress, reserve: formatUnits(reserve, decimals) }
				try {
					const balance = await this.walletBalance(chain, tokenAddress)
					return {
						...row,
						balance: formatUnits(balance, decimals),
						spendable: formatUnits(balance > reserve ? balance - reserve : 0n, decimals),
					}
				} catch (err) {
					this.logger.debug({ err, chain, token }, "Could not read a top-up source balance")
					return { ...row, balance: null, spendable: null }
				}
			}),
		)
		return (await Promise.all(rows)).filter((row): row is BridgeTopUpSource => row !== undefined)
	}

	async quote(params: BridgeTopUpQuoteParams): Promise<BridgeTopUpQuote> {
		const { tokenAddress, decimals } = this.payToken(params.chain, params.token)
		const amountIn = parseUnits(params.amount, decimals)
		if (amountIn <= 0n) throw new Error("Amount must be greater than 0")

		const gateway = await this.gateway(params.chain)
		const quote = await gateway.quoteIntent({ tokenIn: tokenAddress, tokenOut: this.bridgeToken, amountIn })
		const bridgeOut = quote.fillable ? bridgeTopUpOutput(quote.amountOut) : 0n
		const rate =
			quote.amountIn > 0n
				? Number(formatUnits(quote.amountOut, BRIDGE_DECIMALS)) / Number(formatUnits(quote.amountIn, decimals))
				: 0

		return {
			chain: params.chain,
			token: params.token,
			tokenAddress,
			amountIn: formatUnits(amountIn, decimals),
			bridgeOut: formatUnits(bridgeOut, BRIDGE_DECIMALS),
			rate: rate.toString(),
			fillable: quote.fillable && bridgeOut > 0n,
			maxFillableIn: formatUnits(quote.maxFillableIn, decimals),
			route: quote.route,
		}
	}

	/** Re-quotes, then places and drives the order in the background. Returns its state. */
	async start(params: BridgeTopUpQuoteParams): Promise<BridgeTopUpState> {
		if (!(await this.sourceChains()).includes(getChainId(params.chain) ?? -1)) {
			throw new Error(`${params.chain} is not a network the filler fills on`)
		}
		await this.restore()
		if (this.record && isBridgeTopUpPending(this.record.stage)) {
			throw new Error("A BRIDGE top-up is already pending")
		}

		const quote = await this.quote(params)
		if (!quote.fillable) {
			throw new Error(`The orderbook cannot fill ${quote.amountIn} ${params.token} (max ${quote.maxFillableIn})`)
		}

		const { tokenAddress, decimals } = this.payToken(params.chain, params.token)
		const amountIn = parseUnits(quote.amountIn, decimals)
		const balance = await this.balanceOf(params.chain, tokenAddress)
		if (balance < amountIn) {
			const reserve = formatUnits(this.reserve(params.chain, tokenAddress), decimals)
			throw new Error(
				`Not enough ${params.token}: ${formatUnits(balance, decimals)} spendable after the ${reserve} ${params.token} kept for gas, need ${quote.amountIn}`,
			)
		}

		const order = await this.buildOrder(params.chain, tokenAddress, amountIn, parseUnits(quote.bridgeOut, BRIDGE_DECIMALS))
		// Checked again after the awaits above: two clicks must not place two orders.
		if (this.record && isBridgeTopUpPending(this.record.stage)) {
			throw new Error("A BRIDGE top-up is already pending")
		}
		const now = Date.now()
		this.record = {
			id: randomUUID(),
			chain: params.chain,
			token: params.token,
			amountIn: quote.amountIn,
			bridgeOut: quote.bridgeOut,
			stage: "placing",
			createdAt: now,
			updatedAt: now,
			stageTimes: { placing: now },
			order: serializeOrder(order),
		}
		await this.persist()
		void this.place(order, tokenAddress)
		return publicState(this.record)
	}

	/** Cancels an unfilled order and refunds its escrowed input to the filler wallet. */
	async reclaim(): Promise<BridgeTopUpState> {
		await this.restore()
		const record = this.record
		if (!record || record.stage !== "unfilled") throw new Error("There is no unfilled top-up to reclaim")
		await this.update({ stage: "reclaiming", error: undefined })
		void this.driveReclaim()
		return publicState(this.record!)
	}

	/**
	 * Places the order, then tracks it. The SDK builds the `placeOrder` calldata and the
	 * session key; the receipt is read back here, through the filler's own multi-endpoint
	 * client, rather than by the SDK from one endpoint, where a node that has not caught up
	 * with the new block fails the read and loses track of an order that did land.
	 */
	private async place(order: Order, tokenIn: HexString): Promise<void> {
		await this.drive(async (gateway) => {
			const gen = gateway.executeBest(order, SIMPLEX_REFERRER, { auctionTimeMs: AUCTION_TIME_MS })
			const first = await gen.next()
			// Placement is finished here; stopping the generator leaves nothing running.
			await gen.return()
			if (first.done || first.value.status !== "AWAITING_PLACE_ORDER") {
				throw new Error("The SDK did not produce a placement transaction")
			}
			const placement = first.value
			await this.update({ sessionPrivateKey: placement.sessionPrivateKey })
			const calls = this.placementCalls(order, tokenIn, placement)
			const { txHash } = await this.deps.tokenSender.submitCalls(this.record!.chain, calls)
			await this.update({ placeTxHash: txHash })
			await this.recordPlacement(order, txHash)
			await this.trackOrder(gateway)
		})
	}

	/**
	 * Reads the placement receipt and stores the order as the gateway committed it. Throws
	 * when the transaction placed no order, so the top-up is marked failed.
	 */
	private async recordPlacement(order: Order, txHash: HexString): Promise<void> {
		const receipt = await this.deps.clientManager
			.getPublicClient(this.record!.chain)
			.waitForTransactionReceipt({ hash: txHash, timeout: RECEIPT_TIMEOUT_MS })
		const [placed] = parseEventLogs({ abi: IntentGatewayABI, logs: receipt.logs, eventName: "OrderPlaced" })
		if (!placed) throw new Error(`Transaction ${txHash} placed no order`)
		const canonical = deriveCanonicalPlacedOrder(order, placed.args as DecodedOrderPlacedLog["args"])
		await this.update({
			stage: "awaiting_bids",
			commitment: canonical.id,
			order: serializeOrder(canonical),
			error: undefined,
		})
	}

	/** Continues an order whose placement landed but was not recorded. */
	private async recoverPlacement(): Promise<void> {
		await this.drive(async (gateway) => {
			const record = this.record!
			await this.recordPlacement(deserializeOrder(record.order), record.placeTxHash!)
			await this.trackOrder(gateway)
		})
	}

	private async resumeDelivery(): Promise<void> {
		await this.drive(() => this.deliver())
	}

	private async resumeOrder(): Promise<void> {
		await this.drive((gateway) => this.trackOrder(gateway))
	}

	/** Follows a placed order through bids to a fill, or to its deadline. */
	private async trackOrder(gateway: IntentGateway): Promise<void> {
		const record = this.record!
		const order = deserializeOrder(record.order)
		if (await gateway.isOrderFilled(order)) {
			await this.update({ stage: "delivering" })
			await this.deliver()
			return
		}
		if (!record.sessionPrivateKey) {
			await this.update({ stage: "unfilled", error: "The order's session key was not saved, so it cannot be resumed" })
			return
		}
		const gen = gateway.resumeBest(order, {
			sessionPrivateKey: record.sessionPrivateKey,
			auctionTimeMs: AUCTION_TIME_MS,
		})
		for await (const value of gen) {
			if (await this.applyExecutionUpdate(value)) break
		}
		if (this.record!.stage === "delivering") await this.deliver()
	}

	/**
	 * Carries the bought BRIDGE to Hyperbridge. The order's output call dispatched it from
	 * BSC with no relayer fee. Relayers may still deliver it; when none has shortly after BSC
	 * is final on Hyperbridge, the filler relays it itself: a request into Hyperbridge goes
	 * in as an unsigned extrinsic and costs nothing. Runs until the request is delivered.
	 */
	private async deliver(): Promise<void> {
		let relayedAt = 0
		while (!(await this.syncDelivery())) {
			const finalized = this.record!.deliveryStatuses?.find((status) => status.status === "SOURCE_FINALIZED")
			const due = finalized && Date.now() - finalized.timestamp > SELF_RELAY_GRACE_MS
			if (due && Date.now() - relayedAt > DELIVERY_RETRY_MS) {
				relayedAt = Date.now()
				await this.selfRelay(this.record!.deliveryCommitment!).catch((err) => {
					this.logger.warn({ err }, "Self-relay of the BRIDGE to Hyperbridge failed; retrying")
				})
			}
			await sleep(DELIVERY_POLL_MS)
		}
		// Hyperbridge can confirm the delivery before the indexer records the request's last
		// statuses; keep reading until it has, so the timeline gets their times and links.
		const id = this.record!.id
		// Stops when a new top-up replaces this one: the record is then someone else's.
		while (this.record?.id === id && this.needsStatusBackfill()) {
			await sleep(DELIVERY_POLL_MS)
			await this.syncDelivery().catch((err) => this.logger.debug({ err }, "BRIDGE delivery backfill failed"))
		}
	}

	/** Delivered, but the indexer has yet to record the delivery, and not for too long. */
	private needsStatusBackfill(): boolean {
		const record = this.record
		if (!record || record.stage !== "delivered" || !record.deliveryCommitment) return false
		if (record.deliveryStatuses?.some((status) => status.status === "DESTINATION")) return false
		return Date.now() - (record.stageTimes?.delivered ?? 0) < STATUS_BACKFILL_MS
	}

	/**
	 * Reads where the order and its BRIDGE are, from the indexer and the chains, and records
	 * it. Shared by the driver and {@link sync}, and never runs twice at once. True once the
	 * BRIDGE has reached Hyperbridge.
	 */
	private syncDelivery(): Promise<boolean> {
		this.deliverySync ??= this.readDelivery().finally(() => {
			this.deliverySync = undefined
		})
		return this.deliverySync
	}

	private async readDelivery(): Promise<boolean> {
		const record = this.record!
		if (record.stage !== "delivering" && record.stage !== "delivered") return false
		if (!record.fillTxHash) {
			const fillTxHash = await this.findFill()
			if (!fillTxHash) return false
			await this.update({ fillTxHash })
		}
		if (!record.deliveryCommitment) {
			await this.update({ deliveryCommitment: await this.deliveryRequest(record.fillTxHash!) })
		}

		const hyperbridge = await this.hyperbridge()
		const request = await withTimeout(
			this.ismpClient(hyperbridge).queryRequestWithStatus(record.deliveryCommitment!),
			"the BRIDGE request status",
		)
		const statuses = (request?.statuses ?? [])
			.filter((status) => DELIVERY_STATUSES.has(status.status))
			.map((status) => ({
				status: status.status as DeliveryStatus["status"],
				timestamp: toMs((status.metadata as { timestamp?: number }).timestamp),
				txHash: status.metadata.transactionHash as HexString,
			}))
		if (statuses.length > 0) await this.update({ deliveryStatuses: statuses })

		const delivered =
			statuses.some((status) => status.status === "DESTINATION") ||
			!!(await withTimeout(hyperbridge.queryRequestReceipt(record.deliveryCommitment!), "the request receipt"))
		if (delivered && record.stage !== "delivered") await this.update({ stage: "delivered", error: undefined })
		return delivered
	}

	/**
	 * The HyperFungibleToken request the fill dispatched to Hyperbridge. The indexer finds it by
	 * the fill's transaction, as the explorer's search does; the fill's own `Sent` log is the
	 * fallback while the indexer catches up.
	 */
	private async deliveryRequest(fillTxHash: HexString): Promise<HexString> {
		const response = await withTimeout(
			fetch(this.deps.indexerUrl, {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ query: DELIVERY_REQUEST_QUERY, variables: { hash: fillTxHash } }),
			}),
			"the indexer",
		)
		const body = (await response.json()) as { data?: { requestV2s?: { nodes: { commitment: HexString }[] } } }
		const indexed = body.data?.requestV2s?.nodes[0]?.commitment
		if (indexed) return indexed

		const fill = await withTimeout(this.bsc().getTransactionReceipt({ hash: fillTxHash }), "the fill receipt")
		const [sent] = parseEventLogs({ abi: HyperFungibleTokenABI, logs: fill.logs, eventName: "Sent" })
		if (!sent) throw new Error(`The fill ${fillTxHash} sent no BRIDGE to Hyperbridge`)
		return (sent.args as { commitment: HexString }).commitment
	}

	/**
	 * BSC, read through every endpoint there is: the filler need not run it, and a single public
	 * endpoint refuses receipts it considers old.
	 */
	private bsc(): PublicClient {
		this.bscClient ??= createPublicClient({
			chain: bscChain,
			transport: fallback(this.bscProofRpcUrls().map((url) => http(url, { timeout: 10_000, retryCount: 0 }))),
		}) as PublicClient
		return this.bscClient
	}

	/**
	 * The transaction that filled the order. The executor reports it only for a fill this
	 * process selected, so it comes from the indexer, or, while the indexer has yet to reach
	 * BSC's latest blocks, from the gateway's `OrderFilled` log around when the fill was seen.
	 */
	private async findFill(): Promise<HexString | undefined> {
		const record = this.record!
		const indexed = (await this.indexedOrder())?.find((status) => status.status === "FILLED")
		if (indexed) {
			if (indexed.filler && !record.filler) await this.update({ filler: indexed.filler })
			return indexed.transactionHash
		}
		const seenAt = record.events?.find((event) => event.status === "FILLED")?.timestamp ?? record.stageTimes?.delivering
		if (!seenAt) return undefined

		const client = this.bsc()
		const [latest, blockTime] = await Promise.all([
			withTimeout(client.getBlock(), "the latest BSC block"),
			this.blockTimeSeconds(),
		])
		const blockAt = (ms: number) => latest.number - BigInt(Math.ceil((Number(latest.timestamp) - ms / 1000) / blockTime))
		const from = blockAt(seenAt - FILL_SEARCH_BEFORE_MS)
		const to = (() => {
			const end = blockAt(seenAt + FILL_SEARCH_AFTER_MS)
			return end < latest.number ? end : latest.number
		})()
		for (let start = from; start <= to; start += LOG_CHUNK_BLOCKS) {
			const end = start + LOG_CHUNK_BLOCKS - 1n < to ? start + LOG_CHUNK_BLOCKS - 1n : to
			const logs = await withTimeout(
				client.getLogs({
					address: this.deps.configService.getIntentGatewayAddress(BRIDGE_TOP_UP_CHAIN),
					event: ORDER_FILLED_EVENT,
					args: { commitment: record.commitment },
					fromBlock: start,
					toBlock: end,
				}),
				"the fill on BSC",
			)
			if (logs[0]) return logs[0].transactionHash
		}
		return undefined
	}

	/** The order's statuses as the indexer has them: placed, filled, redeemed or refunded. */
	private async indexedOrder(): Promise<{ status: string; timestamp: number; transactionHash: HexString; filler?: HexString }[] | undefined> {
		const response = await withTimeout(
			fetch(this.deps.indexerUrl, {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ query: ORDER_QUERY, variables: { commitment: this.record!.commitment } }),
			}),
			"the indexer",
		)
		const body = (await response.json()) as {
			data?: { iOrderV3s?: { nodes: { statusMetadata: { nodes: { status: string; timestamp: string; transactionHash: HexString; filler?: HexString | null }[] } }[] } }
		}
		return body.data?.iOrderV3s?.nodes[0]?.statusMetadata.nodes.map((node) => ({
			status: node.status,
			timestamp: toMs(node.timestamp),
			transactionHash: node.transactionHash,
			filler: node.filler ?? undefined,
		}))
	}

	/**
	 * Brings a pending top-up up to date from the chains and the indexer, for a dashboard that
	 * is showing it: someone may have filled, delivered or refunded it while this process was
	 * not looking. One sync at a time; a call made while one runs waits for that one.
	 */
	sync(): Promise<void> {
		this.syncing ??= this.reconcile()
			.catch((err) => this.logger.debug({ err }, "BRIDGE top-up sync failed"))
			.finally(() => {
				this.syncing = undefined
			})
		return this.syncing
	}

	private async reconcile(): Promise<void> {
		await this.restore()
		const record = this.record
		if (this.needsStatusBackfill()) {
			await this.syncDelivery()
			return
		}
		if (!record || !isBridgeTopUpPending(record.stage) || !record.commitment) return
		if (record.stage === "awaiting_bids" || record.stage === "filling") {
			const statuses = await this.indexedOrder()
			const fill = statuses?.find((status) => status.status === "FILLED")
			if (fill) {
				await this.update({
					stage: "delivering",
					fillTxHash: fill.transactionHash,
					filler: fill.filler ?? record.filler,
					stageTimes: { ...record.stageTimes, delivering: fill.timestamp },
				})
				void this.resumeDelivery()
			} else if (statuses?.some((status) => status.status === "REFUNDED")) {
				await this.update({ stage: "reclaimed" })
			}
			return
		}
		if (record.stage === "delivering") {
			await this.syncDelivery()
			return
		}
		if (record.stage === "unfilled" || record.stage === "reclaiming") {
			const gateway = await this.gateway(record.chain)
			await this.refunded(gateway, deserializeOrder(record.order))
		}
	}

	/**
	 * Submits the request to Hyperbridge with a proof from BSC state. Public BSC endpoints
	 * differ in whether they serve `eth_getProof` for recent blocks, so each is tried in turn.
	 */
	private async selfRelay(commitment: HexString): Promise<void> {
		const hyperbridge = await this.hyperbridge()
		const failures: string[] = []
		for (const rpcUrl of this.bscProofRpcUrls()) {
			if (await withTimeout(hyperbridge.queryRequestReceipt(commitment), "the request receipt")) return
			try {
				const client = this.ismpClient(hyperbridge, rpcUrl)
				const receipt = await withTimeout(client.aggregateTransactionWithCommitment(commitment), "the relay", 120_000)
				await this.update({ deliveryTxHash: receipt.transactionHash as HexString })
				return
			} catch (err) {
				const reason = err instanceof Error ? err.message.split("\n")[0] : String(err)
				failures.push(`${new URL(rpcUrl).host}: ${reason}`)
			}
		}
		throw new Error(`Could not relay the BRIDGE to Hyperbridge (${failures.join("; ")})`)
	}

	/** The operator's BSC endpoints first, then the free public ones. */
	private bscProofRpcUrls(): string[] {
		const urls = [
			...this.deps.configService.getRpcUrls(BRIDGE_TOP_UP_CHAIN),
			...(chainByChainId(getChainId(BRIDGE_TOP_UP_CHAIN)!)?.defaultRpcUrls ?? []),
		]
		return urls.filter((url, index) => url && urls.indexOf(url) === index)
	}

	/** Records an executor update. True when the order reached a final state. */
	private async applyExecutionUpdate(value: { status: string; [key: string]: unknown }): Promise<boolean> {
		switch (value.status) {
			case "NEW_BID":
			case "BIDS_RECEIVED": {
				const bids = (value.status === "NEW_BID" ? [value.bid] : (value.bids as unknown[])) as Bid[]
				const bidCount = value.status === "BIDS_RECEIVED" ? Number(value.bidCount ?? bids.length) : undefined
				await this.recordEvent({ status: value.status, bidCount, bids: bids.map(bidPreview) })
				if (bidCount !== undefined) await this.update({ bidCount: Math.max(this.record!.bidCount ?? 0, bidCount) })
				return false
			}
			case "BID_SELECTED":
				await this.recordEvent({
					status: "BID_SELECTED",
					selectedSolver: value.selectedSolver as HexString,
					userOpHash: value.userOpHash as HexString,
					txHash: value.transactionHash as HexString | undefined,
				})
				await this.update({ stage: "filling", fillTxHash: value.transactionHash as HexString | undefined })
				return false
			case "FILLED":
				await this.recordEvent({
					status: "FILLED",
					selectedSolver: value.selectedSolver as HexString | undefined,
					userOpHash: value.userOpHash as HexString | undefined,
					txHash: value.transactionHash as HexString | undefined,
				})
				await this.update({
					stage: "delivering",
					fillTxHash: (value.transactionHash as HexString | undefined) ?? this.record!.fillTxHash,
					error: undefined,
				})
				return true
			case "EXPIRED":
			case "FAILED":
				await this.update({ stage: "unfilled", error: value.error as string | undefined })
				return true
			case "CANCELLED":
				await this.update({ stage: "reclaimed" })
				return true
			default:
				return false
		}
	}

	/**
	 * Recovers an unfilled order's escrow, following the Hyperbridge app's recovery: the SDK
	 * cancel flow from the paying chain (BSC's expiry height finalized, the cancel sent, the
	 * cancellation finalized on the source, verified and finalized on Hyperbridge), then the
	 * refund. A relayer normally delivers that refund; if none has within a few minutes of
	 * Hyperbridge finalizing, the filler submits Hyperbridge's response itself. Each step is
	 * recorded as it lands, and a restart resumes from the SDK's persisted progress.
	 */
	private async driveReclaim(): Promise<void> {
		await this.drive(async (gateway) => {
			const record = this.record!
			const order = deserializeOrder(record.order)
			if (await this.refunded(gateway, order)) return

			const sameChain = record.chain === BRIDGE_TOP_UP_CHAIN
			const ismp = await this.cancelClient(record.chain)
			const gen = gateway.cancelOrder(order, ismp)
			let input: HexString | undefined
			while (true) {
				const { value, done } = await gen.next(input as HexString)
				input = undefined
				if (done) break
				switch (value.status) {
					case "DESTINATION_FINALIZED":
						await this.recordRecoveryStep("DESTINATION_FINALIZED")
						break
					case "AWAITING_CANCEL_TRANSACTION": {
						// A cancel already sent (before a restart) is handed back rather than sent twice.
						if (this.record!.reclaimTxHash && !this.hasRecoveryStep("ORDER_CANCELED")) {
							input = this.record!.reclaimTxHash
							break
						}
						const calls: ERC7821Call[] = []
						const quote = await gateway.quoteCancelOrder(order)
						// Chains whose host cannot swap native for the relayer fee take it in the fee token.
						if (value.value === 0n && quote.relayerFee > 0n) {
							const feeToken = await this.feeToken(record.chain)
							calls.push({
								target: feeToken,
								value: 0n,
								data: encodeFunctionData({
									abi: ERC20_ABI,
									functionName: "approve",
									args: [value.to, quote.relayerFee],
								}),
							})
						}
						calls.push({ target: value.to, value: value.value, data: value.data })
						const { txHash } = await this.deps.tokenSender.submitCalls(record.chain, calls)
						await this.update({ reclaimTxHash: txHash })
						input = txHash
						break
					}
					case "CANCEL_STARTED":
						await this.recordRecoveryStep("ORDER_CANCELED", value.receipt.transactionHash as HexString)
						break
					case "SOURCE_FINALIZED":
					case "HYPERBRIDGE_DELIVERED":
					case "HYPERBRIDGE_FINALIZED": {
						const metadata = value.metadata as { transactionHash?: string; timestamp?: number; calldata?: HexString }
						await this.recordRecoveryStep(
							value.status,
							metadata.transactionHash as HexString | undefined,
							metadata.timestamp ? toMs(metadata.timestamp) : undefined,
						)
						if (value.status === "HYPERBRIDGE_FINALIZED" && metadata.calldata) {
							await this.update({ claimCalldata: metadata.calldata })
						}
						break
					}
					case "CANCELLATION_COMPLETE":
						// The same-chain route: the cancel itself refunds the escrow.
						await this.recordRecoveryStep("ORDER_CANCELED", value.transactionHash as HexString)
						await this.recordRecoveryStep("FUNDS_CLAIMED", value.transactionHash as HexString)
						break
				}
			}
			if (sameChain && this.hasRecoveryStep("FUNDS_CLAIMED")) {
				await this.update({ stage: "reclaimed", error: undefined })
				return
			}

			// A relayer delivers the refund to the source; past its window, the filler claims it.
			while (!(await this.refunded(gateway, order))) {
				const finalizedAt = this.record!.recoverySteps?.find((s) => s.step === "HYPERBRIDGE_FINALIZED")?.timestamp
				const claimable = this.record!.claimCalldata && finalizedAt && Date.now() - finalizedAt > RELAYER_DELIVERY_WAIT_MS
				if (claimable && !this.record!.claimTxHash) {
					await this.claimRefund(record.chain, this.record!.claimCalldata!)
					continue
				}
				await sleep(REFUND_POLL_MS)
			}
		})
	}

	/**
	 * Submits Hyperbridge's finalized response to the source chain's ISMP handler, which
	 * refunds the escrow, as the Hyperbridge app's "Claim funds" does.
	 */
	private async claimRefund(chain: string, calldata: HexString): Promise<void> {
		const params = (await withTimeout(
			this.deps.clientManager.getPublicClient(chain).readContract({
				abi: EVM_HOST,
				address: this.deps.configService.getHostAddress(chain),
				functionName: "hostParams",
			}),
			"the host's handler",
		)) as { handler: HexString }
		const { txHash } = await this.deps.tokenSender.submitCalls(chain, [
			{ target: params.handler, value: 0n, data: calldata },
		])
		await this.update({ claimTxHash: txHash })
	}

	/**
	 * Whether the escrow is back with the filler, from the source gateway or the indexer's
	 * REFUNDED status; records the refund as the recovery's last step when it is.
	 */
	private async refunded(gateway: IntentGateway, order: Order): Promise<boolean> {
		const onChain = await withTimeout(gateway.isOrderRefunded(order), "the refund").catch(() => false)
		const indexed = (await this.indexedOrder().catch(() => undefined))?.find((s) => s.status === "REFUNDED")
		if (!onChain && !indexed) return false
		if (!this.hasRecoveryStep("FUNDS_CLAIMED")) {
			await this.recordRecoveryStep(
				"FUNDS_CLAIMED",
				this.record!.claimTxHash ?? indexed?.transactionHash,
				indexed?.timestamp,
			)
		}
		await this.update({ stage: "reclaimed", error: undefined })
		return true
	}

	private hasRecoveryStep(step: RecoveryStepRecord["step"]): boolean {
		return !!this.record?.recoverySteps?.some((entry) => entry.step === step)
	}

	/** Records a recovery step once, at the time it landed (or now). */
	private async recordRecoveryStep(step: RecoveryStepRecord["step"], txHash?: HexString, timestamp?: number) {
		if (this.hasRecoveryStep(step)) return
		const steps = [...(this.record!.recoverySteps ?? []), { step, txHash, timestamp: timestamp ?? Date.now() }]
		await this.update({ recoverySteps: steps })
	}

	/** Runs one background flow for the record; a failure is recorded rather than thrown. */
	private async drive(flow: (gateway: IntentGateway) => Promise<void>): Promise<void> {
		const record = this.record
		if (!record || this.driving === record.id) return
		this.driving = record.id
		try {
			await flow(await this.gateway(record.chain))
		} catch (err) {
			const error = err instanceof Error ? err.message : String(err)
			this.logger.warn({ err, id: record.id, stage: record.stage }, "BRIDGE top-up step failed")
			if (record.stage === "placing" && !record.placeTxHash) {
				await this.update({ stage: "failed", error })
			} else if (record.stage === "delivering") {
				// Filled and paid for: only the relay is outstanding, so it is retried, never abandoned.
				await this.update({ error: `Delivery to Hyperbridge is retrying: ${error}` })
				setTimeout(() => void this.resumeDelivery(), DELIVERY_RETRY_MS)
			} else if (record.stage === "reclaiming") {
				// The SDK keeps the cancellation's progress; the recovery resumes where it stopped.
				await this.update({ error: `Recovery is retrying: ${error}` })
				setTimeout(() => void this.driveReclaim(), DELIVERY_RETRY_MS)
			} else if (record.stage === "placing") {
				// The batch went out; whether it placed an order is settled by reading its
				// receipt again, at the next start.
				await this.update({ stage: "failed", error: `${error} (placement tx ${record.placeTxHash})` })
			} else {
				await this.update({ stage: "unfilled", error })
			}
		} finally {
			if (this.driving === record.id) this.driving = undefined
			this.logger.info(publicState(record), "BRIDGE top-up step finished")
		}
	}

	/** Keeps what the executor reported, for the timeline's offers and its links. */
	private async recordEvent(event: Omit<TopUpExecutionEvent, "timestamp">): Promise<void> {
		const events = [...(this.record!.events ?? []), { ...event, timestamp: Date.now() }]
		await this.update({ events: events.slice(-MAX_EVENTS) })
	}

	private async update(patch: Partial<StoredBridgeTopUp>): Promise<void> {
		if (!this.record) return
		const now = Date.now()
		if (patch.stage && patch.stage !== this.record.stage) {
			patch = { ...patch, stageTimes: { ...this.record.stageTimes, [patch.stage]: now } }
		}
		Object.assign(this.record, patch, { updatedAt: now })
		await this.persist()
	}

	private persist(): Promise<void> {
		const snapshot = this.record ? { ...this.record } : undefined
		const write = this.writes.then(() => patchRuntimeState(this.deps.state, { bridgeTopUp: snapshot }))
		this.writes = write.catch((err) => this.logger.warn({ err }, "Could not save the BRIDGE top-up"))
		return write.then(
			() => undefined,
			() => undefined,
		)
	}

	private async buildOrder(chain: string, tokenIn: HexString, amountIn: bigint, bridgeOut: bigint): Promise<Order> {
		const dispatcher = await this.callDispatcher()
		const bsc = this.bsc()
		const [block, blockTime] = await Promise.all([bsc.getBlockNumber(), this.blockTimeSeconds()])

		return {
			user: bytes20ToBytes32(this.deps.solver),
			source: chain,
			destination: BRIDGE_TOP_UP_CHAIN,
			deadline: block + BigInt(Math.ceil(ORDER_WINDOW_SECONDS / blockTime)),
			nonce: 0n,
			fees: 0n,
			session: "0x0000000000000000000000000000000000000000",
			predispatch: { assets: [], call: "0x" },
			inputs: [{ token: bytes20ToBytes32(tokenIn), amount: amountIn }],
			output: {
				beneficiary: bytes20ToBytes32(dispatcher),
				assets: [{ token: bytes20ToBytes32(this.bridgeToken), amount: bridgeOut }],
				call: encodeBridgeTopUpCall(this.bridgeToken, this.deps.substratePublicKey, bridgeOut),
			},
		}
	}

	/**
	 * Approvals for the gateway to pull the input and the order fee, then `placeOrder`.
	 * The fee token is often the input token itself; one approval then covers both.
	 */
	private placementCalls(
		order: Order,
		tokenIn: HexString,
		placement: { to: HexString; data: HexString; value: bigint; feeTokenAmount: bigint; feeTokenAddress: HexString },
	): ERC7821Call[] {
		const amountIn = order.inputs[0].amount
		const approvals = new Map<string, bigint>([[tokenIn.toLowerCase(), amountIn]])
		if (placement.feeTokenAmount > 0n) {
			const fee = placement.feeTokenAddress.toLowerCase()
			approvals.set(fee, (approvals.get(fee) ?? 0n) + placement.feeTokenAmount)
		}
		const calls: ERC7821Call[] = [...approvals].map(([token, amount]) => ({
			target: token as HexString,
			value: 0n,
			data: encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [placement.to, amount] }),
		}))
		calls.push({ target: placement.to, value: placement.value, data: placement.data })
		return calls
	}

	private payToken(chain: string, token: TopUpPayToken): { tokenAddress: HexString; decimals: number } {
		const { configService } = this.deps
		const tokenAddress = token === "USDC" ? configService.getUsdcAsset(chain) : configService.getUsdtAsset(chain)
		if (!tokenAddress || /^0x0+$/.test(tokenAddress)) throw new Error(`${token} is not configured on ${chain}`)
		const decimals = token === "USDC" ? configService.getUsdcDecimals(chain) : configService.getUsdtDecimals(chain)
		return { tokenAddress, decimals }
	}

	private async walletBalance(chain: string, token: HexString): Promise<bigint> {
		return (await this.deps.clientManager.getPublicClient(chain).readContract({
			abi: ERC20_ABI,
			address: token,
			functionName: "balanceOf",
			args: [this.deps.solver],
		})) as bigint
	}

	/** What the paymaster pulls for gas on the sponsored path, which a top-up must leave. */
	private reserve(chain: string, token: HexString): bigint {
		return paymasterReserveForToken(chain, token.toLowerCase(), this.deps.configService)
	}

	/** Spendable wallet balance: the paymaster's gas reserve is held back. */
	private async balanceOf(chain: string, token: HexString): Promise<bigint> {
		const balance = await this.walletBalance(chain, token)
		const reserve = this.reserve(chain, token)
		return balance > reserve ? balance - reserve : 0n
	}

	private async feeToken(chain: string): Promise<HexString> {
		return (await this.deps.clientManager.getPublicClient(chain).readContract({
			abi: [{ type: "function", name: "feeToken", stateMutability: "view", inputs: [], outputs: [{ type: "address" }] }],
			address: this.deps.configService.getHostAddress(chain),
			functionName: "feeToken",
		})) as HexString
	}

	/** The CallDispatcher the BSC gateway runs output calls through, read from the gateway itself. */
	private callDispatcher(): Promise<HexString> {
		this.dispatcher ??= (async () => {
			const params = (await this.bsc().readContract({
				abi: INTENT_GATEWAY_V2_ABI,
				address: this.deps.configService.getIntentGatewayAddress(BRIDGE_TOP_UP_CHAIN),
				functionName: "params",
			})) as { dispatcher: HexString }
			return params.dispatcher
		})().catch((err) => {
			this.dispatcher = undefined
			throw err
		})
		return this.dispatcher
	}

	private async blockTimeSeconds(): Promise<number> {
		const client = this.bsc()
		const latest = await client.getBlock()
		const earlier = await client.getBlock({ blockNumber: latest.number - 100n })
		const seconds = Number(latest.timestamp - earlier.timestamp) / 100
		return seconds > 0 ? seconds : 1
	}

	private evmChain(chain: string, rpcUrl?: string): EvmChain {
		const { configService } = this.deps
		// BSC need not be a chain the filler runs; its bundler is then Hyperbridge's own.
		const bundlerUrl =
			chain === BRIDGE_TOP_UP_CHAIN
				? (configService.getBundlerUrl(chain) ?? hyperbridgeBundlerUrl(getChainId(chain)!))
				: undefined
		return EvmChain.fromParams({
			chainId: getChainId(chain)!,
			host: configService.getHostAddress(chain),
			rpcUrl: rpcUrl ?? configService.getRpcUrl(chain),
			bundlerUrl,
		})
	}

	private gateway(source: string): Promise<IntentGateway> {
		let gateway = this.gateways.get(source)
		if (!gateway) {
			gateway = (async () =>
				IntentGateway.create(
					this.evmChain(source),
					this.evmChain(BRIDGE_TOP_UP_CHAIN),
					await this.deps.hyperbridge,
					{ storagePath: this.deps.storagePath },
				))().catch((err) => {
				this.gateways.delete(source)
				throw err
			})
			this.gateways.set(source, gateway)
		}
		return gateway
	}

	private hyperbridge(): Promise<SubstrateChain> {
		this.hyperbridgeChain ??= withTimeout(SubstrateChain.connect({
			wsUrl: this.deps.configService.getHyperbridgeWsUrl() ?? NEXUS_WS_URL,
			consensusStateId: NEXUS_CONSENSUS_STATE,
			hasher: "Keccak",
			stateMachineId: NEXUS_STATE_MACHINE,
		}), "Hyperbridge").catch((err) => {
			this.hyperbridgeChain = undefined
			throw err
		})
		return this.hyperbridgeChain
	}

	/** Tracks requests from BSC into Hyperbridge, the route the bought BRIDGE takes. */
	private ismpClient(hyperbridge: SubstrateChain, rpcUrl?: string): IsmpClient {
		return new IsmpClient({
			queryClient: createQueryClient({ url: this.deps.indexerUrl }),
			source: this.evmChain(BRIDGE_TOP_UP_CHAIN, rpcUrl),
			dest: hyperbridge,
			hyperbridge,
		})
	}

	/** Tracks the reclaim's cancellation, which runs between the paying chain and BSC. */
	private async cancelClient(source: string): Promise<IsmpClient> {
		return new IsmpClient({
			queryClient: createQueryClient({ url: this.deps.indexerUrl }),
			source: this.evmChain(source),
			dest: this.evmChain(BRIDGE_TOP_UP_CHAIN),
			hyperbridge: await this.hyperbridge(),
		})
	}

}

function publicState(record: StoredBridgeTopUp): BridgeTopUpState {
	const { order: _order, sessionPrivateKey: _key, ...rest } = record
	return { ...rest, pending: isBridgeTopUpPending(record.stage) }
}

const DELIVERY_STATUSES = new Set(["SOURCE", "SOURCE_FINALIZED", "DESTINATION"])

/** The indexer stamps some statuses in seconds and others in milliseconds. */
function toMs(timestamp: number | string | bigint | undefined): number {
	const value = Number(timestamp ?? 0)
	return value > 0 && value < 1e12 ? value * 1000 : value
}

function withTimeout<T>(promise: Promise<T>, what: string, ms = NETWORK_TIMEOUT_MS): Promise<T> {
	let timer: NodeJS.Timeout | undefined
	const timeout = new Promise<never>((_, reject) => {
		timer = setTimeout(() => reject(new Error(`Timed out reading ${what}`)), ms)
	})
	return Promise.race([promise, timeout]).finally(() => clearTimeout(timer))
}
