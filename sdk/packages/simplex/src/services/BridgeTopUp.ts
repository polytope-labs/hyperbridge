import {
	bytes20ToBytes32,
	type Bid,
	type DecodedOrderPlacedLog,
	deriveCanonicalPlacedOrder,
	IntentGatewayABI,
	createQueryClient,
	EvmChain,
	getChainId,
	getRequestCommitment,
	HyperFungibleTokenABI,
	IntentGateway,
	IsmpClient,
	SubstrateChain,
	type ERC7821Call,
	type HexString,
	type IntentsCoprocessor,
	type Order,
} from "@hyperbridge/sdk"
import { AsyncLocalStorage } from "node:async_hooks"
import { randomUUID } from "node:crypto"
import { setTimeout as sleep } from "node:timers/promises"
import {
	type AbiEvent,
	createPublicClient,
	decodeFunctionData,
	encodeAbiParameters,
	encodeFunctionData,
	fallback,
	formatUnits,
	http,
	type Log,
	parseEventLogs,
	parseUnits,
	type PublicClient,
	stringToHex,
	toHex,
	TransactionNotFoundError,
	TransactionReceiptNotFoundError,
} from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bsc as bscChain } from "viem/chains"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { ENTRYPOINT_ABI } from "@/config/abis/Entrypoint"
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
	StoredSubmission,
	TopUpBidPreview,
	TopUpExecutionEvent,
} from "@/data/types"
import type { ChainClientManager } from "./ChainClientManager"
import type { DelegationService } from "./DelegationService"
import type { FillerConfigService } from "./FillerConfigService"
import { moduleLogger, type Logger } from "./Logger"
import { paymasterReserveForToken } from "./paymaster"
import { PERMIT2_DEADLINE_SECONDS } from "./paymaster/types"
import { type SubmittedTx, SubmittedUnconfirmedError, type SubmittedUserOp } from "./submission"
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
 * How long a failed recovery attempt waits before the next: 1, 2, 4, 8 and 16 min. Spreading the
 * attempts over half an hour lets the budget outlast an RPC, indexer or relayer outage instead
 * of being spent in the first few minutes of one.
 */
const RECLAIM_RETRY_DELAYS_MS = [1, 2, 4, 8, 16].map((minutes) => minutes * 60_000)
/** Consecutive failed recovery attempts before a reclaim stops and waits for the operator: the first try and one per retry delay. */
const MAX_RECLAIM_ATTEMPTS = RECLAIM_RETRY_DELAYS_MS.length + 1
/** A recovery attempt that ran this long before failing was waiting on finality, not failing fast, so its failure starts a fresh count. */
const RECLAIM_LONG_ATTEMPT_MS = 10 * 60_000
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
/**
 * How long a submitted placement UserOp is still looked up before its permit must have expired.
 * Every sponsored top-up op authorizes through Permit2 with a deadline of at most
 * {@link PERMIT2_DEADLINE_SECONDS}; a margin covers the bundler's submission latency.
 */
const PLACEMENT_WINDOW_MS = (Number(PERMIT2_DEADLINE_SECONDS) + 10 * 60) * 1000
/** Chunk for the paying-chain placement scan; wider than BSC's fill search, which public BSC nodes cap. */
const PLACEMENT_CHUNK_BLOCKS = 2_000n
/** A chunk a public endpoint refuses is halved down to this before the attempt gives up. */
const MIN_PLACEMENT_CHUNK_BLOCKS = 100n
/**
 * Chain evidence is read this far below the height a scan reached, and a scan starts this far
 * below the block recorded before submission. Load-balanced endpoints disagree about the newest
 * blocks, and a reorg can rewrite them; state this deep does not change between backends.
 */
const REORG_MARGIN_BLOCKS = 64n
/** Every chain, indexer and Hyperbridge read is bounded, so a stalled endpoint cannot hang a step. */
const NETWORK_TIMEOUT_MS = 30_000
/** Rebuilding a refund response fetches Hyperbridge and consensus proofs, which takes longer than a read. */
const REBUILD_CLAIM_TIMEOUT_MS = 120_000
/** Public BSC endpoints serve logs over a few hundred blocks at a time; the search stays near the fill. */
const LOG_CHUNK_BLOCKS = 300n
const FILL_SEARCH_BEFORE_MS = 3 * 60_000
const FILL_SEARCH_AFTER_MS = 60_000
const ORDER_FILLED_EVENT = IntentGatewayABI.find(
	(item) => item.type === "event" && item.name === "OrderFilled",
) as AbiEvent
const ORDER_PLACED_EVENT = IntentGatewayABI.find(
	(item) => item.type === "event" && item.name === "OrderPlaced",
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

/**
 * The id of the record a background flow (placement recovery, tracking, delivery, reclaim,
 * sync) was started for. `update` drops a flow's writes once that record has been replaced: a
 * recovery still running for an old record must not land in the next top-up.
 */
const flowRecord = new AsyncLocalStorage<string>()

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
	/** Re-reads the filler's BRIDGE balance the moment a delivery lands, for the dashboard. */
	refreshBridgeBalance?: () => Promise<void>
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
	/** Consecutive failed recovery attempts; reset by recorded progress, an operator restart, a long attempt, and a fresh process. */
	private reclaimFailures = 0

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
			// The UserOp hash counts too: its receipt may arrive after the process restarted.
			if (
				record?.stage === "failed" &&
				(record.placeTxHash || record.placeUserOpHash) &&
				!record.commitment &&
				record.sessionPrivateKey
			) {
				void this.recoverPlacement()
				return
			}
			if (!record || !isBridgeTopUpPending(record.stage)) return
			if (record.stage === "placing" && !record.placeTxHash && !record.placeUserOpHash) {
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
		// An operator-started recovery gets a fresh budget.
		this.reclaimFailures = 0
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
			// The session key's address is what names this order's `OrderPlaced` log, and the
			// block it is placed in bounds the scan that finds it if no receipt ever arrives.
			const sessionKeyAddress = privateKeyToAccount(placement.sessionPrivateKey).address as HexString
			const fromBlock = await withTimeout(
				this.deps.clientManager.getPublicClient(this.record!.chain).getBlockNumber(),
				"the paying chain's height",
			)
			await this.update({
				sessionPrivateKey: placement.sessionPrivateKey,
				placeSession: sessionKeyAddress,
				placeFromBlock: fromBlock.toString(),
			})
			const calls = this.placementCalls(order, tokenIn, placement)
			let sent: Awaited<ReturnType<TokenSender["submitCalls"]>>
			try {
				sent = await this.deps.tokenSender.submitCalls(this.record!.chain, calls)
			} catch (err) {
				// The batch was broadcast before the failure, so it may yet place the order.
				// Record what was sent so drive() keeps the top-up pending and recovery can
				// find out whether anything was escrowed, then let drive() schedule that.
				if (!(err instanceof SubmittedUnconfirmedError)) throw err
				await this.update(placementEvidence(err.submission))
				throw err
			}
			// A receipt that cannot be read next still leaves a record that can age out and, for a
			// sponsored op, be settled by the UserOp's nonce when the bundle's tx hash later vanishes.
			await this.update({ ...placementEvidence(sent.submission), placeTxHash: sent.txHash })
			if (await this.recordPlacement(order, sent.txHash)) await this.trackOrder(gateway)
		})
	}

	/**
	 * Reads the placement receipt and stores the order as the gateway committed it, from the
	 * `OrderPlaced` log that names this top-up's session key. A UserOp placement's receipt is a
	 * bundle that can carry other users' placements, so the first log is not necessarily ours.
	 *
	 * A mined receipt with no such log, or one that reverted, is settled: the placement is final
	 * and nothing was escrowed, so it is written `failed` and false is returned. A receipt that
	 * cannot be read throws, so the caller retries or falls back to the chain scan.
	 */
	private async recordPlacement(order: Order, txHash: HexString): Promise<boolean> {
		const record = this.record!
		const receipt = await this.deps.clientManager
			.getPublicClient(record.chain)
			.waitForTransactionReceipt({ hash: txHash, timeout: RECEIPT_TIMEOUT_MS })
		if (this.superseded()) return false
		// Only the gateway's own events count: another op in the same bundle can emit a log with
		// the same shape, naming our session, from any contract.
		const gatewayAddress = this.deps.configService.getIntentGatewayAddress(record.chain).toLowerCase()
		const placements = parseEventLogs({
			abi: IntentGatewayABI,
			logs: receipt.logs.filter((log) => log.address.toLowerCase() === gatewayAddress),
			eventName: "OrderPlaced",
		})
		// The session address names ours; a record from before it was stored falls back to the
		// order's `user`, which is this filler's own account.
		const session = record.placeSession?.toLowerCase()
		const user = bytes20ToBytes32(this.deps.solver).toLowerCase()
		const placed = session
			? placements.find((log) => (log.args as { session?: string }).session?.toLowerCase() === session)
			: placements.find((log) => (log.args as { user?: string }).user?.toLowerCase() === user)
		if (receipt.status === "reverted" || !placed) {
			await this.update({
				stage: "failed",
				error: `Placement tx ${txHash} was mined but placed no order. Nothing was escrowed.`,
			})
			return false
		}
		const canonical = deriveCanonicalPlacedOrder(order, placed.args as DecodedOrderPlacedLog["args"])
		await this.update({
			stage: "awaiting_bids",
			commitment: canonical.id,
			order: serializeOrder(canonical),
			error: undefined,
		})
		return true
	}

	/**
	 * Continues a placement whose outcome was never recorded, using every piece of evidence the
	 * chain offers: the tx receipt if there is one, the bundler's receipt for a UserOp, and the
	 * paying chain's own `OrderPlaced` log for this session. It settles the record as failed only
	 * once the batch can no longer land; otherwise it throws so the caller retries.
	 */
	private async recoverPlacement(): Promise<void> {
		await this.drive(async (gateway) => {
			const record = this.record!
			const order = deserializeOrder(record.order)

			// (a) A known tx: its own receipt decides. A receipt that cannot be read yet falls
			// through, because the chain scan may still find the placement.
			if (record.placeTxHash) {
				try {
					if (await this.recordPlacement(order, record.placeTxHash)) await this.trackOrder(gateway)
					return
				} catch (err) {
					this.logger.debug(
						{ err, txHash: record.placeTxHash },
						"Placement receipt not readable yet; scanning the paying chain",
					)
				}
			}

			// (b) A submitted UserOp: the bundler's receipt is the fast path.
			if (!record.placeTxHash && record.placeUserOpHash) {
				const receipt = await this.deps.tokenSender.userOpReceipt(record.chain, record.placeUserOpHash).catch((err) => {
					this.logger.debug({ err }, "Could not ask the bundler for the placement's receipt")
					return null
				})
				if (receipt && !receipt.success) {
					// The batch reverted; ERC-7821 executeBatch is atomic, so nothing was placed.
					await this.update({
						stage: "failed",
						error: `Placement UserOp ${record.placeUserOpHash} reverted on-chain. Nothing was escrowed.`,
					})
					return
				}
				if (receipt) {
					await this.update({ placeTxHash: receipt.txHash })
					if (await this.recordPlacement(order, receipt.txHash)) await this.trackOrder(gateway)
					return
				}
			}

			// (c) Scan the paying chain for this session's `OrderPlaced`. A scan failure throws
			// here, so an unread range is never mistaken for "not found".
			const scan = await this.findPlacementLog(record)
			if (scan?.txHash) {
				await this.update({ placeTxHash: scan.txHash })
				if (await this.recordPlacement(order, scan.txHash)) await this.trackOrder(gateway)
				return
			}

			// (d) Nothing on chain: settle only once the batch can no longer land. `scannedTo`
			// pins the EntryPoint nonce read to a height the scan has actually covered.
			await this.settleUnplaceable(record, scan?.scannedTo)
		})
	}

	/**
	 * The transaction that placed this order, found on the paying chain rather than from a
	 * receipt: the `OrderPlaced` log whose `session` is this top-up's session key, from the scan
	 * cursor (or a reorg margin below the block stored before submission) up to the latest.
	 * Returns `undefined` when the record predates the scan; a read failure, or a node behind the
	 * range that must be read, throws, so an unread range is never counted as "not found".
	 * `scannedTo` is the height the scan reached, the reference for every later read of the chain.
	 */
	private async findPlacementLog(
		record: StoredBridgeTopUp,
	): Promise<{ txHash?: HexString; scannedTo: bigint } | undefined> {
		const session = record.placeSession?.toLowerCase()
		if (!session || !record.placeFromBlock) return undefined
		const client = this.deps.clientManager.getPublicClient(record.chain)
		// `placeFromBlock` came from one backend of a load-balanced endpoint, which can be ahead of
		// the one that took the broadcast; start a margin below it.
		const placeFrom = belowTip(BigInt(record.placeFromBlock))
		const cursor = record.placeScannedTo ? BigInt(record.placeScannedTo) : placeFrom
		const fromBlock = cursor > placeFrom ? cursor : placeFrom
		const latest = await withTimeout(client.getBlockNumber(), "the latest block")
		if (latest < fromBlock) {
			// A node behind the range we must read would return nothing for it; that is not evidence.
			throw new Error(`The paying chain is at ${latest}, behind the placement's range from ${fromBlock}`)
		}
		const gateway = this.deps.configService.getIntentGatewayAddress(record.chain)
		const txHash = await this.scanPlacementLogs(
			fromBlock,
			latest,
			async (from, to) => {
				const logs = await withTimeout(
					client.getLogs({ address: gateway, event: ORDER_PLACED_EVENT, fromBlock: from, toBlock: to }),
					"the placement on the paying chain",
				)
				const match = logs.find((log) => (log.args as { session?: string }).session?.toLowerCase() === session)
				return match?.transactionHash
			},
			// Saved per chunk, so a scan that fails late resumes where it got to. The cursor trails
			// the tip, so a placement just under it is re-read after a reorg; it never decides
			// anything on its own, the evidence was read up to `latest`.
			async (scanned) => {
				const next = scanned + 1n < belowTip(latest) ? scanned + 1n : belowTip(latest)
				if (next > cursor) await this.update({ placeScannedTo: next.toString() })
			},
		)
		return { txHash, scannedTo: latest }
	}

	/**
	 * Scans a block range in the small chunks public endpoints accept, handing each page to `read`
	 * and stopping at the first page that resolves a value. A failure propagates, so a caller
	 * retries rather than treating an unread range as empty.
	 */
	private async scanLogs<T>(
		fromBlock: bigint,
		toBlock: bigint,
		read: (fromBlock: bigint, toBlock: bigint) => Promise<T | undefined>,
	): Promise<T | undefined> {
		for (let start = fromBlock; start <= toBlock; start += LOG_CHUNK_BLOCKS) {
			const end = start + LOG_CHUNK_BLOCKS - 1n < toBlock ? start + LOG_CHUNK_BLOCKS - 1n : toBlock
			const found = await read(start, end)
			if (found !== undefined) return found
		}
		return undefined
	}

	/**
	 * The paying-chain scan, in chunks wider than BSC's fill search. A chunk a public endpoint
	 * refuses is halved and retried, down to a floor, before the attempt gives up and the caller
	 * retries. `onChunk` runs after each chunk read without a match. Kept separate from
	 * {@link scanLogs} so the BSC fill search is unchanged.
	 */
	private async scanPlacementLogs(
		fromBlock: bigint,
		toBlock: bigint,
		read: (fromBlock: bigint, toBlock: bigint) => Promise<HexString | undefined>,
		onChunk: (scannedTo: bigint) => Promise<void>,
	): Promise<HexString | undefined> {
		let start = fromBlock
		let chunk = PLACEMENT_CHUNK_BLOCKS
		while (start <= toBlock) {
			const end = start + chunk - 1n < toBlock ? start + chunk - 1n : toBlock
			let found: HexString | undefined
			try {
				found = await read(start, end)
			} catch (err) {
				if (chunk <= MIN_PLACEMENT_CHUNK_BLOCKS) throw err
				const halved = chunk / 2n
				chunk = halved < MIN_PLACEMENT_CHUNK_BLOCKS ? MIN_PLACEMENT_CHUNK_BLOCKS : halved
				continue
			}
			if (found !== undefined) return found
			await onChunk(end)
			start = end + 1n
		}
		return undefined
	}

	/**
	 * Settles a placement with no order on chain. "Nothing was escrowed" is written only on
	 * positive evidence that the batch can no longer land, read a reorg margin below the height
	 * the scan covered, so it is asserted against state every backend agrees on:
	 *
	 * - a native tx whose nonce the wallet has moved past: some other tx took that nonce, and had
	 *   it been ours the scan would have found its `OrderPlaced`. A node that no longer lists the
	 *   tx proves nothing, since one backend or mempool can lack a tx another still holds, so a
	 *   tx with no recorded nonce keeps waiting;
	 * - a UserOp whose EntryPoint nonce was consumed, or whose Permit2 permit expired by the
	 *   block timestamp at the height the scan reached, not by this host's clock.
	 *
	 * A tx hash that came from the bundler's receipt is never settled as a native tx: a bundle
	 * reorged out can be re-bundled under a different hash, so it falls to the UserOp rules (the
	 * scan would find a re-bundled placement). A tx the node reports with a block number has been
	 * mined, so its receipt will be readable. A bootstrap UserOp has no expiry and stays pending
	 * until its nonce moves or its log appears: it could still land. Inconclusive cases throw, so
	 * the caller retries.
	 */
	private async settleUnplaceable(record: StoredBridgeTopUp, scannedTo?: bigint): Promise<void> {
		// `updatedAt` is rewritten by every save, so it can never be a submission time.
		const submittedAt = record.placeSubmittedAt ?? record.stageTimes?.placing ?? record.createdAt
		const client = this.deps.clientManager.getPublicClient(record.chain)
		const evidenceBlock = scannedTo === undefined ? undefined : belowTip(scannedTo)

		const txHash = record.placeTxHash
		if (txHash) {
			let tx: { blockNumber: bigint | null; nonce: number } | undefined
			try {
				tx = await withTimeout(client.getTransaction({ hash: txHash }), "the placement tx")
			} catch (err) {
				if (!(err instanceof TransactionNotFoundError)) throw err
				// Not listed by this node: neither mined nor dropped, as far as this read shows.
				this.logger.debug({ txHash }, "The placement tx is not listed by the node")
			}
			if (tx && tx.blockNumber !== null) {
				// Mined: its receipt will be readable, so nothing is settled here.
				throw new Error(`Placement tx ${txHash} is mined; its receipt is being read`)
			}
			if (!record.placeUserOpHash) {
				const txNonce = record.placeTxNonce ?? tx?.nonce
				if (txNonce === undefined || evidenceBlock === undefined) {
					throw new Error(`Placement tx ${txHash} is not mined and its nonce cannot be checked yet; waiting`)
				}
				const accountNonce = await withTimeout(
					client.getTransactionCount({ address: this.deps.solver, blockNumber: evidenceBlock }),
					"the filler wallet's nonce",
				)
				if (accountNonce > txNonce) {
					await this.update({
						stage: "failed",
						error: `Placement tx ${txHash} was replaced or dropped; the filler wallet's nonce has moved past it. Nothing was escrowed.`,
					})
					return
				}
				throw new Error(`Placement tx ${txHash} is not mined yet; it is being checked`)
			}
			// The bundler's tx, still listed: it is the bundler's to mine.
			if (tx) throw new Error(`Placement tx ${txHash} is not mined yet; it is being checked`)
		}

		const userOpHash = record.placeUserOpHash
		if (!userOpHash) throw new Error("No placement tx or UserOp hash is recorded; waiting")
		if (record.placeUserOpExpires && scannedTo !== undefined) {
			// Permit2 mode: the permit's deadline has passed, by the chain's own clock at a height
			// the scan covered. Any later block is later still, so the op cannot validate.
			const block = await withTimeout(client.getBlock({ blockNumber: scannedTo }), "the scanned block")
			if (Number(block.timestamp) * 1000 >= submittedAt + PLACEMENT_WINDOW_MS) {
				await this.update({
					stage: "failed",
					error: `Placement UserOp ${userOpHash} did not land before its permit expired. Nothing was escrowed.`,
				})
				return
			}
		}
		const nonceCall = {
			abi: ENTRYPOINT_ABI,
			address: this.deps.configService.getEntryPointAddress(record.chain)!,
			functionName: "getNonce" as const,
			args: [this.deps.solver, BigInt(record.placeUserOpNonceKey ?? "0")] as const,
		}
		const nonce = (await withTimeout(
			evidenceBlock !== undefined
				? client.readContract({ ...nonceCall, blockNumber: evidenceBlock })
				: client.readContract(nonceCall),
			"the EntryPoint nonce",
		)) as bigint
		if (nonce > BigInt(record.placeUserOpNonce ?? "0")) {
			// The nonce was consumed as of a covered height; if it were ours, the scan would have found our log.
			await this.update({
				stage: "failed",
				error: `Placement UserOp ${userOpHash} was never placed and its EntryPoint nonce has moved on. Nothing was escrowed.`,
			})
			return
		}
		throw new Error(`Placement UserOp ${userOpHash} can still land; it is being checked`)
	}

	/**
	 * Whether the order's deadline has passed. The deadline is a block height on the order's
	 * destination (BSC), so it is read there; the gateway rejects a fill only once the height is
	 * strictly past it. An unreadable height is treated as not passed: an order must not be
	 * settled from a read that failed.
	 */
	private async deadlinePassed(order: Order): Promise<boolean> {
		try {
			return (await withTimeout(this.bsc().getBlockNumber(), "the latest BSC block")) > order.deadline
		} catch (err) {
			this.logger.debug({ err }, "Could not read BSC's height for the order's deadline")
			return false
		}
	}

	private async resumeDelivery(): Promise<void> {
		await this.drive(() => this.deliver())
	}

	private async resumeOrder(): Promise<void> {
		await this.drive((gateway) => this.trackOrder(gateway))
	}

	/** Follows a placed order through bids to a fill, or to its deadline. */
	private async trackOrder(gateway: IntentGateway): Promise<void> {
		// The record this flow was started for is gone; tracking the next one is not its job.
		if (this.superseded()) return
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
		if (delivered && record.stage !== "delivered") {
			await this.update({ stage: "delivered", error: undefined })
			// The balance moves only now. Refresh it before the sync returns, so the dashboard's
			// reload shows it; best-effort, the once-a-minute interval catches up regardless.
			const refreshBalance = this.deps.refreshBridgeBalance
			if (refreshBalance) {
				await refreshBalance().catch((err) =>
					this.logger.warn({ err }, "Failed to refresh the BRIDGE balance"),
				)
			}
		}
		return delivered
	}

	/**
	 * The HyperFungibleToken request the fill dispatched to this filler's Hyperbridge account.
	 *
	 * The fill's own receipt is read first and the `Sent` log it carries is taken, because it is
	 * the transaction this order's output call ran in. A bundled fill can carry several HFT sends,
	 * so only logs from the BRIDGE token addressed to this filler's substrate key count; when more
	 * than one matches, the amount the order's output call sends picks the right one. The indexer
	 * finds the request by the fill's transaction too, as the explorer's search does, and is the
	 * fallback only while the receipt cannot be read — and only when it is unambiguous.
	 */
	private async deliveryRequest(fillTxHash: HexString): Promise<HexString> {
		const receipt = await withTimeout(this.bsc().getTransactionReceipt({ hash: fillTxHash }), "the fill receipt").catch(
			(err) => {
				this.logger.debug({ err, fillTxHash }, "Could not read the fill receipt; falling back to the indexer")
				return undefined
			},
		)
		if (receipt) {
			const filler = this.deps.substratePublicKey.toLowerCase()
			const sends = parseEventLogs({ abi: HyperFungibleTokenABI, logs: receipt.logs, eventName: "Sent" })
				.filter((log) => log.address.toLowerCase() === this.bridgeToken.toLowerCase())
				.map((log) => log.args as { to: HexString; amount: bigint; commitment: HexString })
				.filter((args) => args.to.toLowerCase() === filler)
			if (sends.length === 0) {
				throw new Error(`The fill ${fillTxHash} sent no BRIDGE to this filler's Hyperbridge account`)
			}
			if (sends.length === 1) return sends[0].commitment

			// The order's own output amount names which of several sends is this order's.
			const amount = deserializeOrder(this.record!.order).output.assets[0]?.amount
			const matches = sends.filter((sent) => sent.amount === amount)
			if (matches.length !== 1) {
				throw new Error(
					`The fill ${fillTxHash} has ${sends.length} BRIDGE sends to this filler and ${matches.length} match the order's amount`,
				)
			}
			return matches[0].commitment
		}

		const response = await withTimeout(
			fetch(this.deps.indexerUrl, {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ query: DELIVERY_REQUEST_QUERY, variables: { hash: fillTxHash } }),
			}),
			"the indexer",
		)
		const body = (await response.json()) as { data?: { requestV2s?: { nodes: { commitment: HexString }[] } } }
		const nodes = body.data?.requestV2s?.nodes ?? []
		if (nodes.length !== 1) {
			throw new Error(
				`The fill ${fillTxHash}'s receipt could not be read and the indexer returned ${nodes.length} requests for it`,
			)
		}
		return nodes[0].commitment
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
		return this.scanLogs<HexString>(from, to, async (fromBlock, toBlock) => {
			const logs = await withTimeout(
				client.getLogs({
					address: this.deps.configService.getIntentGatewayAddress(BRIDGE_TOP_UP_CHAIN),
					event: ORDER_FILLED_EVENT,
					args: { commitment: record.commitment },
					fromBlock,
					toBlock,
				}),
				"the fill on BSC",
			)
			return logs[0]?.transactionHash
		})
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
		const id = this.record?.id
		if (id) await flowRecord.run(id, () => this.reconcileRecord())
	}

	private async reconcileRecord(): Promise<void> {
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
			const order = deserializeOrder(record.order)
			// An order reported unfilled may have been filled after all — by a late fill the
			// executor never saw. Check for one before the refund, or the reclaim would cancel
			// an order that already paid out.
			if (record.stage === "unfilled") {
				const fill = (await this.indexedOrder())?.find((status) => status.status === "FILLED")
				if (fill || (await gateway.isOrderFilled(order).catch(() => false))) {
					await this.update({
						stage: "delivering",
						fillTxHash: fill?.transactionHash ?? record.fillTxHash,
						filler: fill?.filler ?? record.filler,
						stageTimes: { ...record.stageTimes, delivering: fill?.timestamp ?? Date.now() },
					})
					void this.resumeDelivery()
					return
				}
			}
			await this.refunded(gateway, order)
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
				await this.update({ stage: "unfilled", error: value.error as string | undefined })
				return true
			case "FAILED": {
				// The executor reports transient faults — an unreadable destination chain, an
				// unexpected error — as FAILED. Before the deadline the order is still live, so
				// the failure is thrown for drive() to retry rather than settling as unfilled.
				const message = (value.error as string | undefined) ?? "The order's executor reported a failure"
				if (!(await this.deadlinePassed(deserializeOrder(this.record!.order)))) throw new Error(message)
				await this.update({ stage: "unfilled", error: message })
				return true
			}
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
	 * Hyperbridge finalizing, the filler submits Hyperbridge's response itself. The cancel pays
	 * the relayer fee in the source host's fee token, and repeated failures, retried on a
	 * backoff, stop the recovery and return the top-up to `unfilled`, where the operator restarts
	 * it from the SDK's saved progress. The refund claim is rebuilt when its calldata is missing or
	 * a claim reverted, since the SDK reports it only once.
	 */
	private async driveReclaim(): Promise<void> {
		await this.drive(async (gateway) => {
			const record = this.record!
			const order = deserializeOrder(record.order)
			if (await this.refunded(gateway, order)) return

			const sameChain = record.chain === BRIDGE_TOP_UP_CHAIN
			// Once Hyperbridge has finalized, the SDK has dropped its persisted progress, so running
			// the cancel again would send a second one; at that point only the refund is left.
			if (!this.hasRecoveryStep("HYPERBRIDGE_FINALIZED")) {
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
							// A cancel sent without a receipt is handed back only once it is known to have
							// landed; one that did not is cleared, and a new cancel goes out below.
							const earlier = this.record!.reclaimSubmission
							if (earlier) {
								const landed = await this.settleCancel(record.chain, earlier)
								if (landed) {
									await this.update({ reclaimTxHash: landed, reclaimSubmission: undefined })
									input = landed
									break
								}
								await this.update({ reclaimSubmission: undefined })
							}
							const calls = await this.cancelCalls(record.chain, value)
							let txHash: HexString
							try {
								;({ txHash } = await this.deps.tokenSender.submitCalls(record.chain, calls))
							} catch (err) {
								// The cancel was broadcast before the failure. Keep what it takes to check
								// it later, so a retry neither sends, and pays for, a second cancel while
								// this one can still land, nor hands the SDK one that never will.
								if (err instanceof SubmittedUnconfirmedError) {
									await this.update({ reclaimSubmission: storedSubmission(err.submission) })
								}
								throw err
							}
							await this.update({ reclaimTxHash: txHash })
							input = txHash
							break
						}
						case "CANCEL_STARTED":
							// Kept with the step: the SDK drops its own copy once Hyperbridge finalizes, and the
							// claim is rebuilt from this request.
							await this.recordRecoveryStep("ORDER_CANCELED", value.receipt.transactionHash as HexString, undefined, {
								getCommitment: this.cancelRequestCommitment(record.chain, value.receipt.logs),
							})
							break
						case "SOURCE_FINALIZED":
						case "HYPERBRIDGE_DELIVERED":
						case "HYPERBRIDGE_FINALIZED": {
							const metadata = value.metadata as { transactionHash?: string; timestamp?: number; calldata?: HexString }
							await this.recordRecoveryStep(
								value.status,
								metadata.transactionHash as HexString | undefined,
								metadata.timestamp ? toMs(metadata.timestamp) : undefined,
								// Written with the step: a restart between two writes would leave a finalized
								// recovery with no claim to submit, and the SDK will not yield it again.
								value.status === "HYPERBRIDGE_FINALIZED" ? { claimCalldata: metadata.calldata } : undefined,
							)
							break
						}
						case "CANCELLATION_COMPLETE":
							// The same-chain route: the cancel itself refunds the escrow.
							await this.recordRecoveryStep("ORDER_CANCELED", value.transactionHash as HexString)
							await this.recordRecoveryStep("FUNDS_CLAIMED", value.transactionHash as HexString)
							break
					}
				}
			}
			if (sameChain && this.hasRecoveryStep("FUNDS_CLAIMED")) {
				await this.update({ stage: "reclaimed", error: undefined })
				return
			}

			// A relayer delivers the refund to the source; past its window, the filler claims it.
			while (!(await this.refunded(gateway, order))) {
				const finalizedAt = this.record!.recoverySteps?.find((s) => s.step === "HYPERBRIDGE_FINALIZED")?.timestamp
				const claimable = finalizedAt && Date.now() - finalizedAt > RELAYER_DELIVERY_WAIT_MS
				if (claimable && !this.record!.claimTxHash) {
					await this.claimRefund(record.chain)
					continue
				}
				await sleep(REFUND_POLL_MS)
			}
		})
	}

	/**
	 * The calls that cancel an unfilled order's escrow from its source chain. The cancel carries
	 * no value: the source host pulls the relayer fee from the filler in its fee token, and the
	 * filler wallet holds stablecoins, not the native token the SDK's quote would spend. The fee
	 * is decoded from the SDK's own calldata so the approval matches what the gateway pulls.
	 */
	private async cancelCalls(chain: string, tx: { to: HexString; data: HexString }): Promise<ERC7821Call[]> {
		const { functionName, args } = decodeFunctionData({ abi: IntentGatewayABI, data: tx.data })
		if (functionName !== "cancelOrder") throw new Error(`Expected cancelOrder calldata, got ${functionName}`)
		const { relayerFee } = (args as readonly [unknown, { relayerFee: bigint }])[1]
		const cancel: ERC7821Call = { target: tx.to, value: 0n, data: tx.data }
		// A same-chain cancel carries no relayer fee, so it needs no approval.
		if (relayerFee === 0n) return [cancel]

		const feeToken = await this.feeToken(chain)
		const balance = await this.walletBalance(chain, feeToken)
		if (balance < relayerFee) {
			const { symbol, decimals } = this.feeTokenLabel(chain, feeToken)
			throw new Error(
				`Not enough ${symbol} on ${chain} to pay the cancel's relayer fee: have ${formatUnits(balance, decimals)}, need ${formatUnits(relayerFee, decimals)}`,
			)
		}
		// Reset to zero first: USDT-style tokens refuse a non-zero approval over a non-zero allowance.
		return [
			{
				target: feeToken,
				value: 0n,
				data: encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [tx.to, 0n] }),
			},
			{
				target: feeToken,
				value: 0n,
				data: encodeFunctionData({ abi: ERC20_ABI, functionName: "approve", args: [tx.to, relayerFee] }),
			},
			cancel,
		]
	}

	/**
	 * Whether a cancel that was submitted without a receipt landed. Returns its transaction hash
	 * when it succeeded, and undefined when it cannot land any more (it reverted, or its nonce
	 * was taken by another transaction), so a new cancel is safe to send. Throws while it could
	 * still land, which keeps the recovery waiting and retrying.
	 *
	 * Like the placement's evidence, the nonce is read a reorg margin below the tip, and the
	 * receipt is looked up before and after it, so a cancel mined in between is not discarded.
	 */
	private async settleCancel(chain: string, sent: StoredSubmission): Promise<HexString | undefined> {
		const client = this.deps.clientManager.getPublicClient(chain)
		const evidenceBlock = belowTip(await withTimeout(client.getBlockNumber(), "the latest block"))

		if (sent.kind === "tx") {
			const receiptOf = () =>
				withTimeout(client.getTransactionReceipt({ hash: sent.txHash }), "the cancel receipt").catch((err) => {
					if (err instanceof TransactionReceiptNotFoundError) return undefined
					throw err
				})
			let receipt = await receiptOf()
			if (!receipt) {
				const nonce = await withTimeout(
					client.getTransactionCount({ address: this.deps.solver, blockNumber: evidenceBlock }),
					"the filler wallet's nonce",
				)
				if (nonce <= sent.nonce) throw new Error(`The cancel ${sent.txHash} is not mined yet; waiting`)
				receipt = await receiptOf()
			}
			if (!receipt) return undefined
			return receipt.status === "success" ? sent.txHash : undefined
		}

		const userOp = await this.deps.tokenSender.userOpReceipt(chain, sent.userOpHash).catch((err) => {
			this.logger.debug({ err }, "Could not ask the bundler for the cancel's receipt")
			return null
		})
		if (userOp) return userOp.success ? userOp.txHash : undefined
		const nonce = (await withTimeout(
			client.readContract({
				abi: ENTRYPOINT_ABI,
				address: this.deps.configService.getEntryPointAddress(chain)!,
				functionName: "getNonce",
				args: [this.deps.solver, BigInt(sent.nonceKey)],
				blockNumber: evidenceBlock,
			}),
			"the EntryPoint nonce",
		)) as bigint
		if (nonce <= BigInt(sent.nonce)) throw new Error(`The cancel UserOp ${sent.userOpHash} can still land; waiting`)
		// The op's nonce was taken and the bundler has no receipt for it: it is not ours.
		return undefined
	}

	/** How the fee token reads in an error: USDC/USDT when configured, else its raw address in raw units. */
	private feeTokenLabel(chain: string, token: HexString): { symbol: string; decimals: number } {
		const { configService } = this.deps
		const address = token.toLowerCase()
		if (address === configService.getUsdcAsset(chain).toLowerCase()) {
			return { symbol: "USDC", decimals: configService.getUsdcDecimals(chain) }
		}
		if (address === configService.getUsdtAsset(chain).toLowerCase()) {
			return { symbol: "USDT", decimals: configService.getUsdtDecimals(chain) }
		}
		return { symbol: token, decimals: 0 }
	}

	/**
	 * Submits Hyperbridge's finalized response to the source chain's ISMP handler, which
	 * refunds the escrow, as the Hyperbridge app's "Claim funds" does.
	 */
	private async claimRefund(chain: string): Promise<void> {
		const calldata = this.record!.claimCalldata ?? (await this.rebuildClaim(chain))
		const params = (await withTimeout(
			this.deps.clientManager.getPublicClient(chain).readContract({
				abi: EVM_HOST,
				address: this.deps.configService.getHostAddress(chain),
				functionName: "hostParams",
			}),
			"the host's handler",
		)) as { handler: HexString }
		try {
			const { txHash } = await this.deps.tokenSender.submitCalls(chain, [
				{ target: params.handler, value: 0n, data: calldata },
			])
			await this.update({ claimTxHash: txHash })
		} catch (err) {
			// The response is proven at one Hyperbridge height. If the host's Hyperbridge client moved
			// past it, replaying the same bytes reverts every time; the next attempt builds them anew.
			await this.update({ claimCalldata: undefined })
			throw err
		}
	}

	/**
	 * Builds Hyperbridge's finalized response for the cancel's GET request again, from the SDK's
	 * own snapshot of it. The SDK reports the calldata once, proven at the height known then;
	 * this is what replaces calldata that reverted or was never saved.
	 */
	private async rebuildClaim(chain: string): Promise<HexString> {
		const commitment = await this.cancelCommitment(chain)
		const request = await withTimeout(
			(await this.cancelClient(chain)).queryGetRequestWithStatus(commitment),
			"the cancel's refund response",
			REBUILD_CLAIM_TIMEOUT_MS,
		)
		const finalized = request?.statuses.find((status) => status.status === "HYPERBRIDGE_FINALIZED")
		const calldata = (finalized?.metadata as { calldata?: HexString } | undefined)?.calldata
		// `buildFinalized` swallows proof and RPC errors, so an outage looks the same as "not yet".
		if (!calldata) {
			throw new Error(
				`No finalized refund response for ${commitment}: Hyperbridge has not finalized it yet, or its proof could not be read`,
			)
		}
		await this.update({ claimCalldata: calldata })
		return calldata
	}

	/** The cancel's GET request: as recorded, else read back from the cancel's receipt. */
	private async cancelCommitment(chain: string): Promise<HexString> {
		const record = this.record!
		if (record.getCommitment) return record.getCommitment
		const cancelTx = record.recoverySteps?.find((entry) => entry.step === "ORDER_CANCELED")?.txHash ?? record.reclaimTxHash
		if (!cancelTx) throw new Error("The cancel transaction is not recorded, so its refund response cannot be rebuilt")
		const receipt = await withTimeout(
			this.deps.clientManager.getPublicClient(chain).getTransactionReceipt({ hash: cancelTx }),
			"the cancel receipt",
		)
		const getCommitment = this.cancelRequestCommitment(chain, receipt.logs)
		if (!getCommitment) throw new Error(`The cancel ${cancelTx} dispatched no GET request`)
		await this.update({ getCommitment })
		return getCommitment
	}

	/** The GET request the IntentGateway on `chain` dispatched through its host, among a cancel receipt's logs. */
	private cancelRequestCommitment(chain: string, logs: Log[]): HexString | undefined {
		const { configService } = this.deps
		return cancelRequestCommitment(
			logs,
			configService.getHostAddress(chain),
			configService.getIntentGatewayAddress(chain),
			this.record!.commitment!,
		)
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

	/**
	 * Records a recovery step once, at the time it landed (or now). `also` is written in the same
	 * update, for what a restart must never see without the step.
	 */
	private async recordRecoveryStep(
		step: RecoveryStepRecord["step"],
		txHash?: HexString,
		timestamp?: number,
		also?: Partial<StoredBridgeTopUp>,
	) {
		if (this.hasRecoveryStep(step)) return
		// Progress was made: the budget for consecutive failures is fresh again.
		this.reclaimFailures = 0
		const steps = [...(this.record!.recoverySteps ?? []), { step, txHash, timestamp: timestamp ?? Date.now() }]
		await this.update({ ...also, recoverySteps: steps })
	}

	/**
	 * Schedules a retry in `delayMs` ({@link DELIVERY_RETRY_MS} unless given), but only while the
	 * record it was scheduled for is still the current one. A timer from a finished top-up must
	 * never drive the next one: a top-up the operator started after this record was replaced has
	 * its own state to protect.
	 */
	private scheduleRetry(id: string, run: () => void, delayMs = DELIVERY_RETRY_MS): void {
		setTimeout(() => {
			if (this.record?.id === id) run()
		}, delayMs)
	}

	/**
	 * Runs one background flow for the record; a failure is recorded rather than thrown. Every
	 * write the flow makes, its failure handling included, is bound to this record.
	 */
	private drive(flow: (gateway: IntentGateway) => Promise<void>): Promise<void> {
		const record = this.record
		if (!record) return Promise.resolve()
		return flowRecord.run(record.id, () => this.driveRecord(record, flow))
	}

	private async driveRecord(record: StoredBridgeTopUp, flow: (gateway: IntentGateway) => Promise<void>): Promise<void> {
		if (this.driving === record.id) return
		this.driving = record.id
		const startedAt = Date.now()
		try {
			await flow(await this.gateway(record.chain))
		} catch (err) {
			const error = err instanceof Error ? err.message : String(err)
			this.logger.warn({ err, id: record.id, stage: record.stage }, "BRIDGE top-up step failed")
			const id = record.id
			const submitted = !!record.placeTxHash || !!record.placeUserOpHash
			if (record.stage === "placing" && !submitted) {
				await this.update({ stage: "failed", error })
			} else if (submitted && (record.stage === "placing" || record.stage === "failed")) {
				// The batch went out but its receipt never arrived, so the order may be live.
				// Stay pending — start() must refuse another top-up — and check again; only a
				// read that settles the batch (dropped, reverted, expired) may mark it failed.
				await this.update({ error: `Placement is unconfirmed; still checking: ${error}` })
				this.scheduleRetry(id, () => void this.recoverPlacement())
			} else if (record.stage === "delivering") {
				// Filled and paid for: only the relay is outstanding, so it is retried, never abandoned.
				await this.update({ error: `Delivery to Hyperbridge is retrying: ${error}` })
				this.scheduleRetry(id, () => void this.resumeDelivery())
			} else if (record.stage === "reclaiming") {
				// A failure after a long finality wait is not a fast failure, so it starts a fresh count.
				if (Date.now() - startedAt >= RECLAIM_LONG_ATTEMPT_MS) this.reclaimFailures = 0
				this.reclaimFailures += 1
				const retryDelay = RECLAIM_RETRY_DELAYS_MS[this.reclaimFailures - 1]
				if (this.reclaimFailures >= MAX_RECLAIM_ATTEMPTS) {
					// Stopped, not abandoned: the operator restarts from the SDK's saved progress.
					this.reclaimFailures = 0
					await this.update({
						stage: "unfilled",
						error: `Recovery stopped after ${MAX_RECLAIM_ATTEMPTS} failed attempts: ${error}`,
					})
				} else {
					// The SDK keeps the cancellation's progress; the recovery resumes where it stopped.
					await this.update({ error: `Recovery is retrying: ${error}` })
					this.scheduleRetry(id, () => void this.driveReclaim(), retryDelay)
				}
			} else if (record.stage === "awaiting_bids" || record.stage === "filling") {
				// A live order's transient failure — an unreadable destination chain, a location
				// read that timed out — must not settle it as unfilled. Only its own deadline does.
				const order = deserializeOrder(record.order)
				if (await this.deadlinePassed(order)) {
					await this.update({ stage: "unfilled", error })
				} else {
					await this.update({ error: `Tracking is retrying: ${error}` })
					this.scheduleRetry(id, () => void this.resumeOrder())
				}
			} else {
				// Only an order past its auction is unfilled. A failure anywhere else (a legacy
				// failure, a stage a stale flow left behind) records the error and moves nothing.
				await this.update({ error })
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

	/** Whether the calling flow was started for a record that has since been replaced. */
	private superseded(): boolean {
		const id = flowRecord.getStore()
		return id !== undefined && this.record?.id !== id
	}

	private async update(patch: Partial<StoredBridgeTopUp>): Promise<void> {
		// A flow writes only to the record it was started for: a recovery that finishes after the
		// operator started the next top-up would otherwise land its stage, hashes and errors in it.
		if (!this.record || this.superseded()) return
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

/** `height` less the reorg margin, never below genesis: the newest height evidence is read at. */
function belowTip(height: bigint): bigint {
	return height > REORG_MARGIN_BLOCKS ? height - REORG_MARGIN_BLOCKS : 0n
}

/** What a placement submission leaves in the record for recovery to settle it by. */
function placementEvidence(submission: SubmittedUserOp | SubmittedTx): Partial<StoredBridgeTopUp> {
	if (submission.kind === "tx") {
		return { placeTxHash: submission.txHash, placeTxNonce: submission.nonce, placeSubmittedAt: submission.submittedAt }
	}
	return {
		placeUserOpHash: submission.userOpHash,
		placeUserOpNonce: submission.nonce.toString(),
		placeUserOpNonceKey: submission.nonceKey.toString(),
		placeUserOpExpires: submission.expires,
		placeSubmittedAt: submission.submittedAt,
	}
}

/** A submission as the JSON state keeps it. */
function storedSubmission(submission: SubmittedUserOp | SubmittedTx): StoredSubmission {
	if (submission.kind === "tx") return submission
	return {
		kind: "userOp",
		userOpHash: submission.userOpHash,
		nonce: submission.nonce.toString(),
		nonceKey: submission.nonceKey.toString(),
		expires: submission.expires,
		submittedAt: submission.submittedAt,
	}
}

/**
 * The commitment of the GET request a cross-chain cancel dispatched, from the cancel's receipt logs.
 * A sponsored cancel's receipt is its whole bundle, so another op's GET request can come first, and
 * so can another order's cancel through the same gateway. Only this chain's host counts, for a
 * request the IntentGateway sent (the host records the sender as its packed address) whose `context`
 * begins with `orderCommitment`: `_cancelFromSource` builds it as
 * `abi.encode(commitment, user, inputs, totalRequired)`, so its first word is the order's commitment.
 */
function cancelRequestCommitment(
	logs: Log[],
	host: HexString,
	gateway: HexString,
	orderCommitment: HexString,
): HexString | undefined {
	const own = logs.filter((log) => log.address.toLowerCase() === host.toLowerCase())
	const event = parseEventLogs({ abi: EVM_HOST, logs: own, eventName: "GetRequestEvent" }).find(
		(item) =>
			item.args.from.toLowerCase() === gateway.toLowerCase() &&
			item.args.context.toLowerCase().startsWith(orderCommitment.toLowerCase()),
	)
	if (!event) return undefined
	return getRequestCommitment({ ...event.args, keys: [...event.args.keys] })
}

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
