import { randomBytes, randomUUID } from "node:crypto"
import { getChainId, MAX_DECLARED_ENTRIES, type HexString } from "@hyperbridge/sdk"
import { type AssetRegistry, normalizeSymbol } from "@/config/asset-registry"
import type { LimitOrder, LimitOrderFilter, LimitOrderInsert, LimitOrderStore } from "@/data/types"
import type { ContractInteractionService } from "@/services/ContractInteractionService"
import type { DelegationService } from "@/services/DelegationService"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { VaultBalancePosition } from "@/funding/types"
import type { Signer } from "@/services/wallet"
import { fromHuman, ORDERBOOK_SCALE, rateFrom, signedAmounts, toHuman, toScaled } from "./amounts"
import { OrderbookClient, OrderbookRequestError } from "./client"
import type {
	Book,
	CancelOrderResult,
	ChainInfo,
	HeartbeatResult,
	OrderbookLimits,
	PostedOrder,
	SubmitOrderResult,
	TokenMinSize,
} from "./types"

/** How long a read of `serverInfo` and `books` is reused before being refreshed. */
const LIMITS_TTL_MS = 5 * 60 * 1000

/**
 * The statuses a posting may be written onto: the row still expects one. A
 * posting is a slow round trip, and an operator's cancel or the expiry sweep
 * can land while one is in flight.
 */
const POSTABLE = ["open", "resizing"] as const

/**
 * How long a row must have sat untouched before reconciliation treats a missing
 * entry as stranded by a crash rather than as a posting still in flight.
 *
 * Three paths leave a live row with no entry to find for as long as a round trip
 * takes: a create between the insert and the post, a resize, and a renewal
 * between the cancel and the post. All three write `updatedAt` as they start, so
 * one rule covers them.
 */
const POST_GRACE_MS = 2 * 60 * 1000

/** EIP-712 types for the signed messages the orderbook accepts. */
const EIP712_DOMAIN = [
	{ name: "name", type: "string" },
	{ name: "version", type: "string" },
] as const

const CANCEL_ORDER_TYPES = {
	EIP712Domain: EIP712_DOMAIN,
	CancelOrder: [
		{ name: "solver", type: "address" },
		{ name: "commitment", type: "bytes32" },
		{ name: "timestamp", type: "uint64" },
	],
} as const

const HEARTBEAT_TYPES = {
	EIP712Domain: EIP712_DOMAIN,
	Heartbeat: [
		{ name: "solver", type: "address" },
		{ name: "timestamp", type: "uint64" },
	],
} as const

/**
 * An operator error: the request itself is wrong, and retrying it unchanged will
 * fail the same way. The HTTP layer turns these into a 400.
 */
export class LimitOrderValidationError extends Error {}

/**
 * One limit order as the operator states it: what simplex takes in, and what it
 * pays out for that.
 *
 * "10000 USDC for 139000000 CNGN" is `tokenIn: "USDC", amountIn: 10000e18,
 * tokenOut: "CNGN", amountOut: 139000000e18`. The rate and the side of the book
 * follow from those, so the order is directional by construction: it prices
 * USDC to CNGN swaps and never the reverse.
 */
export interface CreateLimitOrderRequest {
	fillChain: string
	/** The symbol simplex takes in. */
	tokenIn: string
	/** Whole tokens taken in, as a decimal string: "1000", "1500.25". */
	amountIn: string
	/** The symbol simplex pays out. */
	tokenOut: string
	/** Whole tokens paid out, as a decimal string. */
	amountOut: string
	acceptedSources: string[]
	/**
	 * How long the order lives, in seconds. Optional: without it the order lives
	 * `[orderbook] defaultTtlSecs`, or 365 days when that is unset too. It is the
	 * TTL of the posting and the life of the order itself: one clock, derived into
	 * `expiresAt` on the row, and nothing renews it. When it runs out the posting
	 * lapses and the order is done.
	 */
	ttlSecs?: number
}

/**
 * What the service reports about an order nobody asked it about: a fill worked it
 * down, or took it under the dust floor. Operator-initiated changes are reported
 * by the controller that took the request.
 */
export type LimitOrderEvent =
	| { kind: "resized"; order: LimitOrder; delivered: bigint }
	| { kind: "filled"; order: LimitOrder }

/**
 * What came of an order being created: the orderbook's answer, or `unposted` for
 * a same-asset quote, which has no book to sit on and is never sent anywhere.
 */
export type PostingOutcome = SubmitOrderResult | { kind: "unposted" }

/** The stored limit order, and what became of its posting. */
export interface PostedLimitOrder {
	order: LimitOrder
	result: PostingOutcome
}

export interface CancelledLimitOrder {
	order: LimitOrder
	result: CancelOrderResult
}

/** What one reconciliation pass put right. */
export interface ReconcileReport {
	/** Orderbook entries no limit order here owns, now withdrawn. */
	cancelled: number
	/** Limit orders whose entry had gone, now posted again. */
	reposted: number
	/** Postings the orderbook is not backing in full, left alone and surfaced. */
	underFunded: number
}

/**
 * Creates the operator's limit orders and keeps the orderbook's copy of them.
 *
 * A limit order is stored before it is posted, so a posting that fails leaves a
 * row the operator can see and act on rather than a request that vanished. The
 * orderbook's answer is then written back onto that row: an accepted order
 * carries its commitment, a rejected one carries the reason.
 */
/**
 * The configured ERC-4626 vaults' holdings, as `VaultFundingPlanner` reports them. A fill sources
 * a payout shortfall from these, so they back a limit order as much as the wallet does.
 */
export interface VaultHoldings {
	getBalanceSnapshot(chain?: string): Promise<VaultBalancePosition[]>
}

export class LimitOrderService {
	private logger: Logger
	private cachedLimits?: { limits: OrderbookLimits; readAt: number }
	private onEvent?: (event: LimitOrderEvent) => void

	constructor(
		private readonly store: LimitOrderStore,
		private readonly client: OrderbookClient,
		private readonly contractService: ContractInteractionService,
		private readonly configService: FillerConfigService,
		private readonly assetRegistry: AssetRegistry,
		private readonly signer: Signer,
		private readonly defaultTtlSecs: number,
		private readonly delegationService?: DelegationService,
		loggers: LoggerContext = defaultLoggerContext(),
		/** Where a new order's nonce starts; tests pin it, production draws it at random. */
		private readonly startingNonce: () => bigint = initialOrderNonce,
		/** The configured ERC-4626 vaults, when the operator runs any. */
		private readonly vaultBalances?: VaultHoldings,
	) {
		this.logger = loggers.get("limit-orders")
	}

	/**
	 * Attaches the listener fill-driven changes are reported to. `Simplex` does
	 * this after boot, the same way the filler is handed the service itself.
	 */
	listen(onEvent: (event: LimitOrderEvent) => void): void {
		this.onEvent = onEvent
	}

	list(filter?: LimitOrderFilter): Promise<LimitOrder[]> {
		return this.store.list(filter)
	}

	get(id: string): Promise<LimitOrder | null> {
		return this.store.get(id)
	}

	/**
	 * Validates, stores and posts one limit order.
	 *
	 * Everything the orderbook would refuse for is checked first, so a rejection
	 * that does come back is either a race with the server's own limits or a bug
	 * in what we encode. The one check that can cost a transaction, 7702
	 * delegation, runs before the row is written: the orderbook deletes an
	 * undelegated solver's orders outright, so posting without it achieves nothing.
	 */
	async create(given: CreateLimitOrderRequest): Promise<PostedLimitOrder> {
		const limits = await this.limits()
		// No book trades a symbol against itself, so a same-asset quote is ours
		// alone: it prices swaps here and is never advertised.
		const sameAsset = normalizeSymbol(given.tokenIn) === normalizeSymbol(given.tokenOut)
		const book = sameAsset
			? { id: given.tokenIn, base: given.tokenIn, quote: given.tokenIn }
			: this.resolveBook(limits, given.tokenIn, given.tokenOut)
		// Symbols are matched however they were cased (the asset registry spells cNGN
		// "CNGN") and carried on as the book spells them, which is how the rate, the
		// dust floor and the published decimals below look them up.
		const request = sameAsset
			? given
			: { ...given, tokenIn: spelledAs(book, given.tokenIn), tokenOut: spelledAs(book, given.tokenOut) }
		const ttlSecs = request.ttlSecs ?? this.defaultTtlSecs
		const { amountIn, amountOut } = this.validate(request, book, limits, ttlSecs)

		const { side, price } = rateFrom({
			base: book.base,
			quote: book.quote,
			tokenIn: request.tokenIn,
			amountIn,
			amountOut,
		})

		// A same-asset fill hands back the same token it took in, so anything above
		// par pays out more than it receives. That is the rule the curves expressed
		// as ask-only and priced below par.
		if (sameAsset && price > ORDERBOOK_SCALE) {
			throw new LimitOrderValidationError(
				`A ${request.tokenIn} for ${request.tokenIn} order must pay out no more than it takes in`,
			)
		}

		await this.assertWalletCanPay(request.tokenOut, request.fillChain, amountOut)

		if (!sameAsset && this.delegationService && !(await this.delegationService.setupDelegation(request.fillChain))) {
			throw new LimitOrderValidationError(
				`The solver is not 7702-delegated on ${request.fillChain}, and the orderbook deletes an undelegated solver's orders`,
			)
		}

		const insert: LimitOrderInsert = {
			id: randomUUID(),
			book: book.id,
			base: book.base,
			quote: book.quote,
			side,
			fillChain: request.fillChain,
			price: price.toString(),
			size: amountOut.toString(),
			acceptedSources: request.acceptedSources,
			ttlSecs,
			expiresAt: new Date(Date.now() + ttlSecs * 1000).toISOString(),
			orderNonce: this.startingNonce().toString(),
		}
		const stored = await this.store.create(insert)
		if (sameAsset) {
			this.logger.info({ id: stored.id, symbol: book.base }, "Same-asset limit order stored; nothing to post")
			return { order: stored, result: { kind: "unposted" } }
		}
		return this.post(stored)
	}

	/**
	 * Withdraws a limit order.
	 *
	 * Local first: once the status is no longer `open` nothing new can draw on the
	 * order, which matters more than the orderbook entry going away promptly. A
	 * cancel the orderbook refuses still leaves the order cancelled here, with the
	 * refusal on the row for the operator to see.
	 */
	async cancel(id: string): Promise<CancelledLimitOrder> {
		const existing = await this.store.get(id)
		if (!existing) throw new LimitOrderValidationError(`No limit order with id '${id}'`)

		const order = (await this.store.setStatus(id, "cancelled"))!
		if (!existing.commitment) {
			return { order, result: { kind: "cancelled", commitment: "0x" as HexString } }
		}

		const result = await this.withdraw(existing.commitment as HexString)
		if (result.kind === "cancelled" || (result.kind === "rejected" && result.code === "UNKNOWN_ORDER")) {
			// UNKNOWN_ORDER means the entry is already gone, whether it expired, was
			// swept, or the orderbook deleted it over balance or delegation.
			return {
				order: (await this.store.setPosting(id, {
					commitment: null,
					bookExpiresAt: null,
					bookPrice: null,
					orderNonce: order.orderNonce,
					status: "cancelled",
					lastError: null,
				}))!,
				result,
			}
		}

		// The commitment stays on the row. A refusal leaves the entry live, and a
		// request that never got an answer may have left it live: either way
		// something still owns it, and clearing it here is how an entry is orphaned.
		const message = result.kind === "rejected" ? `${result.code}: ${result.message}` : result.message
		this.logger.error({ id, err: message }, "Could not clear the limit order's entry on the orderbook")
		return { order: (await this.store.setStatus(id, "cancelled", message))!, result }
	}

	/**
	 * Signs and sends one `cancelOrder`, retrying once on a timestamp the server
	 * would not take. Both retryable codes are about the timestamp alone, so a
	 * later one is the whole fix: `SIGNATURE_EXPIRED` means our clock has drifted
	 * past the skew the server allows, `SIGNATURE_REUSED` that a previous cancel
	 * already used this second.
	 */
	private async withdraw(commitment: HexString): Promise<CancelOrderResult> {
		const first = await this.signAndCancel(commitment, nowSecs())
		if (first.kind !== "rejected") return first
		if (first.code !== "SIGNATURE_EXPIRED" && first.code !== "SIGNATURE_REUSED") return first

		if (first.code === "SIGNATURE_EXPIRED") {
			this.logger.warn({ commitment }, "Orderbook rejected the cancel timestamp as expired; check this host's clock")
		}
		return this.signAndCancel(commitment, nowSecs() + 1)
	}

	private async signAndCancel(commitment: HexString, timestamp: number): Promise<CancelOrderResult> {
		const { eip712DomainName, eip712DomainVersion } = (await this.limits()).serverInfo
		const solver = this.signer.address
		// `solver` is part of what is signed, not merely an argument alongside it:
		// it is what stops two solvers ever signing the same digest.
		const signature = await this.signer.signTypedData({
			domain: { name: eip712DomainName, version: eip712DomainVersion },
			types: CANCEL_ORDER_TYPES,
			primaryType: "CancelOrder",
			message: { solver, commitment, timestamp },
		})
		try {
			return await this.client.cancelOrder({ solver, commitment, timestamp, signature })
		} catch (err) {
			// Never `UNKNOWN_ORDER`: that is the orderbook's considered answer that the
			// entry is gone, and this is the request not getting one at all.
			if (err instanceof OrderbookRequestError) return { kind: "failed", message: err.message }
			throw err
		}
	}

	/**
	 * The pairs an order can be written against, the smallest payout each token may carry, and
	 * the tokens the orderbook registers on each chain.
	 *
	 * The orderbook lists the books; `resolveBook` refuses a request naming anything else, so the
	 * operator is offered exactly these and no combination of symbols they could not post. The
	 * chains do the same for the source chains an order can accept.
	 */
	async books(): Promise<{ books: Book[]; minOrderSizes: TokenMinSize[]; chains: ChainInfo[] }> {
		const limits = await this.limits()
		return { books: limits.books, minOrderSizes: limits.serverInfo.minOrderSizes, chains: limits.chains ?? [] }
	}

	/**
	 * Reads `serverInfo` and `books`, reusing the last read for {@link LIMITS_TTL_MS}.
	 *
	 * The limits change rarely and every create needs them, so re-reading per
	 * request would put a round trip in front of an operator action for nothing.
	 */
	private async limits(): Promise<OrderbookLimits> {
		const cached = this.cachedLimits
		if (cached && Date.now() - cached.readAt < LIMITS_TTL_MS) return cached.limits
		try {
			const limits = await this.client.limits()
			this.cachedLimits = { limits, readAt: Date.now() }
			return limits
		} catch (err) {
			// A stale read still describes the server's limits better than nothing,
			// and the posting itself is about to find out whether it is still right.
			if (cached) {
				this.logger.warn({ err }, "Could not refresh the orderbook's limits; using the last read")
				return cached.limits
			}
			throw err
		}
	}

	/** The book that trades this pair of symbols, whichever way round and however cased they were given. */
	private resolveBook(limits: OrderbookLimits, tokenIn: string, tokenOut: string): Book {
		const tin = normalizeSymbol(tokenIn)
		const tout = normalizeSymbol(tokenOut)
		const book = limits.books.find((candidate) => {
			const base = normalizeSymbol(candidate.base)
			const quote = normalizeSymbol(candidate.quote)
			return (base === tin && quote === tout) || (quote === tin && base === tout)
		})
		if (!book) {
			const known = limits.books.map((candidate) => candidate.id).join(", ")
			throw new LimitOrderValidationError(
				`No book trades ${tokenIn} against ${tokenOut}. The orderbook offers: ${known || "none"}`,
			)
		}
		return book
	}

	private validate(
		request: CreateLimitOrderRequest,
		book: Book,
		limits: OrderbookLimits,
		ttlSecs: number,
	): { amountIn: bigint; amountOut: bigint } {
		if (!this.configService.getConfiguredChainIds().includes(getChainId(request.fillChain) ?? -1)) {
			throw new LimitOrderValidationError(`'${request.fillChain}' is not a chain this filler is configured for`)
		}

		// Whole tokens in, 1e18 out. An operator states what they are trading, not
		// what the orderbook's scale or the asset's decimals happen to be.
		const scaled: Record<"amountIn" | "amountOut", bigint> = { amountIn: 0n, amountOut: 0n }
		for (const name of ["amountIn", "amountOut"] as const) {
			try {
				scaled[name] = fromHuman(request[name] ?? "")
			} catch (err) {
				throw new LimitOrderValidationError(
					`${name} must be an amount in whole tokens, like "1000" or "1500.25"; ${(err as Error).message}`,
				)
			}
			if (scaled[name] <= 0n) throw new LimitOrderValidationError(`${name} must be greater than zero`)
		}

		const sources = request.acceptedSources ?? []
		if (sources.length === 0) {
			throw new LimitOrderValidationError(
				"acceptedSources must name at least one source chain; the orderbook rejects an order that declares none",
			)
		}
		if (new Set(sources).size !== sources.length) {
			throw new LimitOrderValidationError("acceptedSources must not repeat a chain")
		}
		if (sources.length > MAX_DECLARED_ENTRIES) {
			throw new LimitOrderValidationError(`acceptedSources cannot name more than ${MAX_DECLARED_ENTRIES} chains`)
		}
		for (const source of sources) {
			const bytes = new TextEncoder().encode(source).length
			if (bytes === 0 || bytes > 255) {
				throw new LimitOrderValidationError(`'${source}' is not a state machine id of 1 to 255 UTF-8 bytes`)
			}
		}

		// The orderbook lists the chains it serves, so a typo in a source chain is
		// worth catching here rather than as an `UNSUPPORTED_SOURCE_CHAIN` against a
		// row already stored.
		const served = limits.serverInfo.chains ?? []
		if (served.length > 0) {
			const unknown = [request.fillChain, ...sources].filter((chain) => !served.includes(chain))
			if (unknown.length > 0) {
				throw new LimitOrderValidationError(
					`The orderbook does not serve ${unknown.join(", ")}. It serves: ${served.join(", ")}`,
				)
			}
		}

		// The other half of `UNSUPPORTED_SOURCE_CHAIN`: a swapper on a source chain pays the
		// order's input there, so the orderbook must register it on that chain. Only chains its
		// registry lists are judged. A same-asset order is never posted, so the rule is not its.
		if (book.base !== book.quote) {
			const input = normalizeSymbol(request.tokenIn)
			const lacking = sources.filter((source) => {
				const listed = limits.chains?.find((chain) => chain.id === source)
				return listed && !listed.tokens.some((token) => normalizeSymbol(token.symbol) === input)
			})
			if (lacking.length > 0) {
				throw new LimitOrderValidationError(
					`The orderbook has no ${request.tokenIn} on ${lacking.join(", ")}, so swaps from there cannot pay this order`,
				)
			}
		}

		if (ttlSecs < limits.serverInfo.minOrderTtlSecs) {
			throw new LimitOrderValidationError(
				`ttlSecs must be at least the orderbook's minimum of ${limits.serverInfo.minOrderTtlSecs}; got ${ttlSecs}`,
			)
		}

		// The dust floor applies to what the operator pays out, which is the side
		// the orderbook advertises depth on.
		const floor = limits.serverInfo.minOrderSizes.find((entry) => entry.symbol === request.tokenOut)
		if (floor && scaled.amountOut < BigInt(floor.size)) {
			throw new LimitOrderValidationError(
				`amountOut is below the orderbook's dust floor for ${request.tokenOut} (${toHuman(BigInt(floor.size))})`,
			)
		}

		// Resolved here rather than at post time so an unknown symbol reads as an
		// error on the request instead of a rejection against a row already stored.
		for (const symbol of [book.base, book.quote]) {
			if (!this.assetRegistry.getAddress(symbol, request.fillChain)) {
				throw new LimitOrderValidationError(`'${symbol}' does not resolve to a token address on ${request.fillChain}`)
			}
		}

		return { amountIn: scaled.amountIn, amountOut: scaled.amountOut }
	}

	/**
	 * Refuses an order the wallet cannot pay out.
	 *
	 * The orderbook backs an entry with the solver's actual balance and cuts down
	 * what it is not holding, so an order written against money that is not there
	 * is refused or silently shrunk rather than filled. Checked here, against the
	 * balance on the fill chain, so the operator hears it while they are creating
	 * the order.
	 *
	 * A fill pays out of the wallet and then withdraws any shortfall from the
	 * configured ERC-4626 vaults in the same batch, so what those vaults hold of
	 * the payout token backs the order too and is counted here. Without it an
	 * operator who sweeps inventory into a vault is refused orders their filler
	 * would have filled.
	 *
	 * Each order is checked against the whole balance, not against what is left of
	 * it after the others. One balance backs every order resting on it — that is
	 * what quoting both sides of a book is — and the orderbook says as much: it
	 * advertises each entry at `min(quoted, balance)` rather than dividing the
	 * balance between them. Whichever order fills first draws the inventory down,
	 * and the rest are cut to what is left. Netting them here would refuse the
	 * second side of every book. Vault positions are counted the same way: the
	 * whole position, not what is left after reservations for fills in flight.
	 */
	private async assertWalletCanPay(symbol: string, chain: string, payout: bigint): Promise<void> {
		const token = this.assetRegistry.getAddress(symbol, chain)
		if (!token) return

		const decimals = await this.decimalsFor(symbol, token, chain)
		const wallet = toScaled(
			await this.contractService.getTokenBalance(chain, token, this.signer.address as HexString),
			decimals,
		)
		const vaults = await this.vaultHoldings(token, chain)

		if (wallet + vaults < payout) {
			const held = vaults > 0n ? `${toHuman(wallet)} in the wallet and ${toHuman(vaults)} in vaults` : toHuman(wallet)
			throw new LimitOrderValidationError(
				`The wallet holds ${held} ${symbol} on ${chain}, which cannot pay out ${toHuman(payout)}`,
			)
		}
	}

	/**
	 * What the configured vaults hold of `token` on `chain`, in scaled units.
	 *
	 * Zero when no vault is configured. A snapshot that cannot be read is also
	 * zero: the wallet alone then has to cover the order, which refuses one the
	 * filler might have managed rather than accepting one it could not.
	 */
	private async vaultHoldings(token: HexString, chain: string): Promise<bigint> {
		if (!this.vaultBalances) return 0n
		let positions: VaultBalancePosition[]
		try {
			positions = await this.vaultBalances.getBalanceSnapshot(chain)
		} catch (err) {
			this.logger.warn({ chain, token, err }, "Could not read vault balances; backing the order on the wallet alone")
			return 0n
		}
		const wanted = token.toLowerCase()
		let total = 0n
		for (const position of positions) {
			if (position.chain !== chain || position.asset.toLowerCase() !== wanted) continue
			total += toScaled(position.positionAssets, position.decimals)
		}
		return total
	}

	/**
	 * Works a limit order down by what a fill delivered, and puts the rest back on
	 * the orderbook.
	 *
	 * The order is moved to `resizing` and its new size written before the
	 * orderbook is touched, so a crash mid-way leaves a row that says what it was
	 * doing rather than one that silently advertises output it has already paid.
	 *
	 * The old entry is cancelled before the new one is posted. Two live entries
	 * for one liability would advertise the same output twice, and the gap between
	 * the two calls is at most one request.
	 */
	async settleFill(id: string, delivered: bigint): Promise<LimitOrder | null> {
		const order = await this.store.get(id)
		if (!order) return null

		const remaining = await this.store.drawDown(id, delivered.toString())
		if (!remaining) return null
		return this.resize(remaining, delivered)
	}

	/**
	 * Puts an order that has already been worked down back on the book, or closes
	 * it when what is left is under the orderbook's dust floor for what it pays.
	 *
	 * Separate from the draw-down because that is a store write that belongs in
	 * the same transaction as the hold it settles, while this is a round trip to
	 * the orderbook and must not be inside one.
	 */
	async resize(remaining: LimitOrder, delivered: bigint): Promise<LimitOrder | null> {
		const id = remaining.id
		const floor = await this.dustFloor(remaining)
		if (BigInt(remaining.remaining) < floor) {
			this.logger.info(
				{ id, remaining: remaining.remaining, floor: floor.toString() },
				"Limit order worked down past the dust floor; closing it",
			)
			if (remaining.commitment) await this.withdraw(remaining.commitment as HexString)
			const closed = await this.store.setPosting(id, {
				commitment: null,
				bookExpiresAt: null,
				bookPrice: null,
				orderNonce: remaining.orderNonce,
				status: "filled",
				lastError: null,
			})
			if (closed) this.report({ kind: "filled", order: closed })
			return closed
		}

		const resized = await this.repost(await this.store.setStatus(id, "resizing"))
		if (resized) this.report({ kind: "resized", order: resized, delivered })
		return resized
	}

	/**
	 * Cancels the current posting and puts the order back at its present size.
	 *
	 * Also how a posting is renewed before it expires and how reconciliation
	 * restores an entry the orderbook dropped, since all three want the same
	 * thing: whatever the order has left, live on the book again.
	 */
	async repost(order: LimitOrder | null): Promise<LimitOrder | null> {
		if (!order) return null
		// A same-asset order was never on the book, so there is nothing to put back.
		if (isLocal(order)) return this.store.setStatus(order.id, "open")

		// Expiry belongs here rather than in each caller. The matcher refuses an
		// expired order, so posting one advertises depth that is quoted to swappers,
		// counted as liquidity, and then refused on every fill that comes back, which
		// is worse than no order at all. Unlike the grace period there is no timing to
		// it: an order that has expired has expired on whichever clock arrives.
		if (hasExpired(order.expiresAt, new Date())) return this.retire(order)

		if (order.commitment) {
			const withdrawn = await this.withdraw(order.commitment as HexString)
			// Only a cancel the orderbook confirmed, or its word that the entry is
			// already gone, means there is nothing left up. Anything else leaves the
			// entry live, and posting over it is how two entries end up behind one
			// liability. The row keeps its commitment and the next cycle tries again.
			const gone = withdrawn.kind === "cancelled" || (withdrawn.kind === "rejected" && withdrawn.code === "UNKNOWN_ORDER")
			if (!gone) {
				const message = withdrawn.kind === "rejected" ? `${withdrawn.code}: ${withdrawn.message}` : withdrawn.message
				this.logger.error(
					{ id: order.id, commitment: order.commitment, err: message },
					"Could not clear the old entry; leaving the posting alone rather than adding a second",
				)
				return this.store.setStatus(order.id, "open", message)
			}
		}

		// Marked for the duration. A repost is a cancel and a post, and between them
		// the row is live with no entry on the book, which is exactly what
		// reconciliation would otherwise read as one to put back. The status is what
		// `POSTABLE` lets the posting write over, and the write refreshes
		// `updatedAt`, which is what reconciliation actually leaves alone.
		await this.store.setStatus(order.id, "resizing")

		// A fresh nonce, because the orderbook remembers every op hash it has taken
		// and a signed op cannot be posted twice.
		const nonce = (BigInt(order.orderNonce) + 1n).toString()
		return (await this.post({ ...order, commitment: null, orderNonce: nonce })).order
	}

	/** A listener that throws is the listener's problem, not the fill's. */
	private report(event: LimitOrderEvent): void {
		try {
			this.onEvent?.(event)
		} catch (err) {
			this.logger.warn({ id: event.order.id, err }, "A limit order listener threw")
		}
	}

	/**
	 * Keeps the solver's postings surfaced.
	 *
	 * The orderbook suspends a solver it has not heard from for a while, and knows
	 * nothing at all about one that has never had an order accepted, so a heartbeat
	 * before the first posting can only come back `UNKNOWN_SOLVER`. Answers null
	 * when there is nothing posted to keep alive.
	 */
	async heartbeat(): Promise<HeartbeatResult | null> {
		if (!(await this.live()).some((order) => order.commitment)) return null

		const first = await this.signAndHeartbeat(nowSecs())
		if (first.kind === "accepted") {
			if (first.reactivatedOrders > 0) {
				this.logger.warn({ reactivated: first.reactivatedOrders }, "Heartbeat brought suspended postings back")
			}
			return first
		}
		// A heartbeat sent on the back of a posting can land in the same second as
		// the scheduled one, and the timestamp is the whole of the objection.
		if (first.code !== "SIGNATURE_REUSED" && first.code !== "SIGNATURE_EXPIRED") return first
		return this.signAndHeartbeat(nowSecs() + 1)
	}

	/** How often to heartbeat: half the server's interval, so one lost request is not a suspension. */
	async heartbeatIntervalMs(): Promise<number> {
		const { heartbeatIntervalSecs } = (await this.limits()).serverInfo
		return Math.max(1, Math.floor(heartbeatIntervalSecs / 2)) * 1000
	}


	/**
	 * Brings the orderbook's copy of the operator's orders back in line with ours.
	 *
	 * They drift apart when a request never got an answer or the process died
	 * between two of them: an entry the orderbook still lists that nothing here
	 * owns, an order here whose entry has gone, or an entry the orderbook has cut
	 * down because the solver cannot cover what it quoted.
	 */
	async reconcile(now: Date = new Date()): Promise<ReconcileReport> {
		const [entries, live] = await Promise.all([this.postedOrders(), this.live()])
		const owners = new Map(
			live.filter((order) => order.commitment).map((order) => [order.commitment!.toLowerCase(), order]),
		)
		const report: ReconcileReport = { cancelled: 0, reposted: 0, underFunded: 0 }
		const found = new Set<string>()

		for (const entry of entries) {
			const owner = owners.get(entry.commitment.toLowerCase())
			if (!owner) {
				this.logger.warn({ commitment: entry.commitment }, "Orderbook entry no limit order here owns; cancelling it")
				await this.withdraw(entry.commitment)
				report.cancelled++
				continue
			}
			found.add(entry.commitment.toLowerCase())
			if (underFunds(entry)) {
				const reason = underFunded(entry)
				this.logger.warn({ id: owner.id, commitment: entry.commitment }, reason)
				await this.store.setStatus(owner.id, owner.status, reason)
				report.underFunded++
			}
		}

		for (const order of live) {
			if (isLocal(order)) continue
			if (order.commitment && found.has(order.commitment.toLowerCase())) continue
			// A row touched moments ago has a posting in flight: a create posts after
			// its insert, and a repost posts after its cancel, both leaving the row
			// live with nothing on the book to find. Posting now would put a second
			// entry behind one liability, which is worse than waiting a cycle.
			if (sinceMs(order.updatedAt, now) < POST_GRACE_MS) continue

			this.logger.warn({ id: order.id }, "Limit order has no entry on the orderbook; posting it again")
			try {
				// Without the cancel `repost` leads with: the entry is already gone.
				const posted = await this.repost({ ...order, commitment: null })
				// Counted on a posting that landed, not on the attempt: `repost` also
				// retires an order that has expired and reports a refusal on the row.
				if (posted?.commitment) report.reposted++
			} catch (err) {
				this.logger.error({ id: order.id, err }, "Could not post the limit order again")
			}
		}

		return report
	}

	/**
	 * Withdraws every order that has outlived the operator's own `expiresAt`.
	 *
	 * The matcher refuses an expired order, so leaving it alone would advertise
	 * depth that no swapper could ever draw on: renewal would keep its posting
	 * alive and reconciliation would put it back if it lapsed. The orderbook's own
	 * entry expiry does not cover this, since a posting is renewed long before it
	 * reaches its TTL.
	 */
	async expireStale(now: Date = new Date()): Promise<number> {
		const stale = (await this.live()).filter((order) => hasExpired(order.expiresAt, now))
		for (const order of stale) {
			try {
				await this.retire(order)
			} catch (err) {
				this.logger.error({ id: order.id, err }, "Could not withdraw the expired limit order")
			}
		}
		return stale.length
	}

	/** Takes an expired order off the book and closes the row. */
	private async retire(order: LimitOrder): Promise<LimitOrder | null> {
		this.logger.info({ id: order.id, expiresAt: order.expiresAt }, "Limit order has expired; withdrawing it")
		if (order.commitment) await this.withdraw(order.commitment as HexString)
		return this.store.setPosting(order.id, {
			commitment: null,
			bookExpiresAt: null,
			bookPrice: null,
			orderNonce: order.orderNonce,
			status: "expired",
			lastError: null,
		})
	}

	/** Every order that has a posting on the book, or should have one. */
	private async live(): Promise<LimitOrder[]> {
		const [open, resizing] = await Promise.all([
			this.store.list({ status: "open" }),
			this.store.list({ status: "resizing" }),
		])
		return [...open, ...resizing]
	}

	/** Every entry the orderbook holds for this solver, walked to the last page. */
	private async postedOrders(): Promise<PostedOrder[]> {
		const entries: PostedOrder[] = []
		let cursor: string | undefined
		do {
			const page = await this.client.myOrders(this.signer.address, cursor)
			entries.push(...page.orders)
			cursor = page.cursor
		} while (cursor)
		return entries
	}

	private async signAndHeartbeat(timestamp: number): Promise<HeartbeatResult> {
		const { eip712DomainName, eip712DomainVersion } = (await this.limits()).serverInfo
		const solver = this.signer.address
		const signature = await this.signer.signTypedData({
			domain: { name: eip712DomainName, version: eip712DomainVersion },
			types: HEARTBEAT_TYPES,
			primaryType: "Heartbeat",
			message: { solver, timestamp },
		})
		return this.client.heartbeat({ solver, timestamp, signature })
	}

	/**
	 * A token's decimals on the fill chain.
	 *
	 * The orderbook publishes its own registry, and every post and repost needs
	 * both sides, so taking them from the limits already cached here saves two
	 * chain reads each time. It is the same registry the server prices against,
	 * which is what makes it the right source rather than merely a cheap one. A
	 * chain or symbol it does not list falls back to the token itself.
	 */
	private async decimalsFor(symbol: string, token: HexString, chain: string): Promise<number> {
		const published = (await this.limits()).chains
			?.find((entry) => entry.id === chain)
			?.tokens.find((entry) => entry.symbol === symbol)
		return published ? published.decimals : this.contractService.getTokenDecimals(token, chain)
	}

	/** The orderbook's dust floor for what this order pays out, or zero when it names none. */
	private async dustFloor(order: LimitOrder): Promise<bigint> {
		const paid = order.side === "BID" ? order.quote : order.base
		const { serverInfo } = await this.limits()
		const floor = serverInfo.minOrderSizes.find((entry) => entry.symbol === paid)
		return floor ? BigInt(floor.size) : 0n
	}

	/** Builds and submits the posting, then writes the orderbook's answer onto the row. */
	private async post(order: LimitOrder): Promise<PostedLimitOrder> {
		const { result, orderNonce } = await this.submit(order)

		if (result.kind === "accepted" || result.kind === "unchanged") {
			const posted = result.order
			this.logger.info(
				{ id: order.id, commitment: posted.commitment, price: posted.price },
				"Limit order posted to the orderbook",
			)
			const stored = await this.store.setPosting(
				order.id,
				{
					commitment: posted.commitment,
					bookExpiresAt: posted.expiresAt,
					bookPrice: posted.price,
					orderNonce: orderNonce.toString(),
					status: "open",
					lastError: null,
				},
				POSTABLE,
			)
			if (!stored) {
				// The operator cancelled it, or the sweep expired it, while this posting
				// was in flight. Writing it back as open would undo that, so the entry
				// the orderbook has just taken comes down instead.
				this.logger.warn(
					{ id: order.id, commitment: posted.commitment },
					"Limit order moved on while it was being posted; withdrawing the entry",
				)
				await this.withdraw(posted.commitment)
				return { order: (await this.store.get(order.id))!, result }
			}
			// A solver the orderbook has just met is suspended until it hears from it,
			// which is what `surfaced: false` is saying. The posting is also what makes
			// the heartbeat answerable, so it goes out now rather than on the next tick.
			if (result.kind === "accepted" && !result.surfaced) {
				await this.heartbeat().catch((err) =>
					this.logger.warn({ id: order.id, err }, "Could not heartbeat the posting into view"),
				)
			}
			return { order: stored!, result }
		}

		const message = `${result.code}: ${result.message}`

		// A failure is not a refusal. The orderbook could not decide the op, whether
		// its database was away or the request never landed, so the order stays open
		// with the reason on it and something posts it again later. Marking it
		// `rejected` would retire an order the operator still wants over one timeout.
		if (result.kind === "failed" && result.retryable) {
			this.logger.warn({ id: order.id, err: message }, "Could not post the limit order; leaving it to be posted again")
			return {
				order: (await this.store.setPosting(order.id, {
					commitment: null,
					bookExpiresAt: null,
					bookPrice: null,
					orderNonce: orderNonce.toString(),
					status: "open",
					lastError: message,
				}))!,
				result,
			}
		}

		this.logger.error({ id: order.id, err: message }, "Orderbook refused the limit order")
		// Guarded for the same reason: a refusal that arrives after the operator
		// cancelled says nothing about the row they left behind.
		const rejected = await this.store.setStatus(order.id, "rejected", message, POSTABLE)
		return { order: rejected ?? (await this.store.get(order.id))!, result }
	}

	/**
	 * Submits the posting, answering a nonce the orderbook has already seen with a
	 * fresh one.
	 *
	 * The two refusals that say so are not the same thing. `REPLAYED` is an op hash
	 * it remembers for an order that is gone, and the nonce is the only way past.
	 * `ORDER_EXISTS` is a live entry sitting at that commitment: if it is ours, the
	 * posting has already happened and is taken as accepted, because posting again
	 * on a new nonce would put a second entry behind the same liability and record
	 * only the second one.
	 */
	private async submit(order: LimitOrder): Promise<{ result: SubmitOrderResult; orderNonce: bigint }> {
		const orderNonce = BigInt(order.orderNonce)
		const attempt = await this.buildAndSubmit(order, orderNonce)
		const first = attempt.result
		if (first.kind !== "rejected" || (first.code !== "REPLAYED" && first.code !== "ORDER_EXISTS")) {
			return { result: first, orderNonce }
		}

		if (first.code === "ORDER_EXISTS" && attempt.commitment) {
			const live = await this.entryAt(order, attempt.commitment)
			if (live) {
				this.logger.info({ id: order.id, commitment: live.commitment }, "This posting is already on the orderbook")
				return { result: { kind: "unchanged", order: live }, orderNonce }
			}
		}

		this.logger.warn({ id: order.id, code: first.code }, "Orderbook has seen this op before; reposting on a new nonce")
		const retried = orderNonce + 1n
		return { result: (await this.buildAndSubmit(order, retried)).result, orderNonce: retried }
	}

	/**
	 * The entry the orderbook holds at `commitment`, if it holds one. A lookup that
	 * fails answers null, which sends the caller down the nonce-bump path it would
	 * have taken anyway.
	 */
	private async entryAt(order: LimitOrder, commitment: HexString): Promise<PostedOrder | null> {
		try {
			return await this.client.orderAt(this.signer.address, commitment)
		} catch (err) {
			this.logger.warn({ id: order.id, err }, "Could not read the entry the orderbook says exists")
			return null
		}
	}

	/** The orderbook's answer, and the commitment the op we sent hashes to. */
	private async buildAndSubmit(
		order: LimitOrder,
		orderNonce: bigint,
	): Promise<{ result: SubmitOrderResult; commitment?: HexString }> {
		const { commitment, userOp } = await this.buildUserOp(order, orderNonce)
		try {
			return { result: await this.client.submitOrder(userOp), commitment }
		} catch (err) {
			if (err instanceof OrderbookRequestError) {
				return { result: { kind: "failed", code: "REQUEST_FAILED", message: err.message, retryable: true }, commitment }
			}
			throw err
		}
	}

	private async buildUserOp(order: LimitOrder, orderNonce: bigint): Promise<{ commitment: HexString; userOp: HexString }> {
		const baseToken = this.assetRegistry.getAddress(order.base, order.fillChain)!
		const quoteToken = this.assetRegistry.getAddress(order.quote, order.fillChain)!
		const [baseDecimals, quoteDecimals] = await Promise.all([
			this.decimalsFor(order.base, baseToken, order.fillChain),
			this.decimalsFor(order.quote, quoteToken, order.fillChain),
		])

		// A bid receives the base and pays the quote; an ask is the other way round.
		const [inputToken, outputToken] = order.side === "BID" ? [baseToken, quoteToken] : [quoteToken, baseToken]
		const { inputAmount, outputAmount } = signedAmounts({
			side: order.side,
			size: BigInt(order.remaining),
			price: BigInt(order.price),
			baseDecimals,
			quoteDecimals,
		})

		const entryPointAddress = this.configService.getEntryPointAddress(order.fillChain)
		if (!entryPointAddress) {
			throw new LimitOrderValidationError(`No EntryPoint is configured for ${order.fillChain}`)
		}

		return this.contractService.prepareLimitOrderUserOp({
			fillChain: order.fillChain,
			entryPointAddress,
			inputToken,
			outputToken,
			inputAmount,
			outputAmount,
			orderNonce,
			ttlSecs: order.ttlSecs,
			acceptedSourceChains: order.acceptedSources,
		})
	}
}

/**
 * Where a new order's nonce starts.
 *
 * The posted op is built from the order's tokens, amounts, TTL and this nonce, and nothing else, so
 * two orders on the same terms sign byte-identical ops. The orderbook refuses any op it has seen
 * as REPLAYED and the poster bumps the nonce once, so an order re-created on the terms of two
 * earlier ones, which had used 0 and 1, was refused for good. A random 64-bit start keeps each
 * order's nonces its own; a resize still steps on from it by one.
 */
export function initialOrderNonce(): bigint {
	return BigInt(`0x${randomBytes(8).toString("hex")}`)
}

/** `symbol` as `book` spells it: its base or its quote, whichever it names. */
function spelledAs(book: { base: string; quote: string }, symbol: string): string {
	return normalizeSymbol(symbol) === normalizeSymbol(book.base) ? book.base : book.quote
}

function nowSecs(): number {
	return Math.floor(Date.now() / 1000)
}

/** Whether this order is ours alone: a same-asset quote has no book to sit on. */
function isLocal(order: LimitOrder): boolean {
	return order.base === order.quote
}

/** Whether the operator's own expiry has passed. An unreadable one never has. */
function hasExpired(expiresAt: string | null, now: Date): boolean {
	if (!expiresAt) return false
	const at = Date.parse(expiresAt)
	return !Number.isNaN(at) && at <= now.getTime()
}


/** How long ago a row was written. Its stamps are UTC but not marked as such. */
function sinceMs(updatedAt: string, now: Date): number {
	const written = Date.parse(`${updatedAt.replace(" ", "T")}Z`)
	return Number.isNaN(written) ? Number.POSITIVE_INFINITY : now.getTime() - written
}

/**
 * Whether the orderbook is telling us the solver cannot cover this posting.
 *
 * `backed` is false before any balance has been read as well as when one falls
 * short, and a posting surfaces at its full quoted size until a cycle reaches
 * it. Only a `backed: false` a cycle actually decided is worth an operator's
 * attention; the rest is a posting that has simply not been looked at yet.
 */
function underFunds(entry: PostedOrder): boolean {
	return entry.resized === true || (entry.backed === false && !!entry.validatedAt)
}

/** What the operator has to act on: the posting is live but not covered in full. */
function underFunded(entry: PostedOrder): string {
	return entry.resized
		? `UNDER_FUNDED: the orderbook is advertising ${entry.advertisedSize} of the ${entry.quotedAmount} quoted`
		: "UNDER_FUNDED: the solver's balance does not cover this posting"
}
