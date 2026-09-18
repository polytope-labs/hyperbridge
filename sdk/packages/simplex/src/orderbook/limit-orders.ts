import { randomUUID } from "node:crypto"
import { getChainId, MAX_DECLARED_ENTRIES, type HexString } from "@hyperbridge/sdk"
import type { AssetRegistry } from "@/config/asset-registry"
import type { LimitOrder, LimitOrderFilter, LimitOrderInsert, LimitOrderStore } from "@/data/types"
import type { ContractInteractionService } from "@/services/ContractInteractionService"
import type { DelegationService } from "@/services/DelegationService"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { Signer } from "@/services/wallet"
import { fromHuman, rateFrom, signedAmounts, toHuman } from "./amounts"
import { OrderbookClient, OrderbookRequestError } from "./client"
import type { Book, CancelOrderResult, OrderbookLimits, PostedOrder, SubmitOrderResult } from "./types"

/** How long a read of `serverInfo` and `books` is reused before being refreshed. */
const LIMITS_TTL_MS = 5 * 60 * 1000

/** EIP-712 types for the signed messages the orderbook accepts. */
const CANCEL_ORDER_TYPES = {
	EIP712Domain: [
		{ name: "name", type: "string" },
		{ name: "version", type: "string" },
	],
	CancelOrder: [
		{ name: "commitment", type: "bytes32" },
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
	 * How long the order lives, in seconds. It is the TTL of the posting and the
	 * life of the order itself: one clock, derived into `expiresAt` on the row, and
	 * nothing renews it. When it runs out the posting lapses and the order is done.
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

/** The stored limit order, and what the orderbook said about its posting. */
export interface PostedLimitOrder {
	order: LimitOrder
	result: SubmitOrderResult
}

export interface CancelledLimitOrder {
	order: LimitOrder
	result: CancelOrderResult
}

/**
 * Creates the operator's limit orders and keeps the orderbook's copy of them.
 *
 * A limit order is stored before it is posted, so a posting that fails leaves a
 * row the operator can see and act on rather than a request that vanished. The
 * orderbook's answer is then written back onto that row: an accepted order
 * carries its commitment, a rejected one carries the reason.
 */
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
	async create(request: CreateLimitOrderRequest): Promise<PostedLimitOrder> {
		// Ahead of the book lookup, which would otherwise report a same-symbol
		// request as an unknown pair.
		if (request.tokenIn === request.tokenOut) {
			throw new LimitOrderValidationError("tokenIn and tokenOut must be different symbols")
		}
		const limits = await this.limits()
		const book = this.resolveBook(limits, request.tokenIn, request.tokenOut)
		const ttlSecs = request.ttlSecs ?? this.defaultTtlSecs
		const { amountIn, amountOut } = this.validate(request, book, limits, ttlSecs)

		const { side, price } = rateFrom({
			base: book.base,
			quote: book.quote,
			tokenIn: request.tokenIn,
			amountIn,
			amountOut,
		})

		if (this.delegationService && !(await this.delegationService.setupDelegation(request.fillChain))) {
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
		}
		return this.post(await this.store.create(insert))
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
		const signature = await this.signer.signTypedData({
			domain: { name: eip712DomainName, version: eip712DomainVersion },
			types: CANCEL_ORDER_TYPES,
			primaryType: "CancelOrder",
			message: { commitment, timestamp },
		})
		try {
			return await this.client.cancelOrder({ solver: this.signer.address, commitment, timestamp, signature })
		} catch (err) {
			// Never `UNKNOWN_ORDER`: that is the orderbook's considered answer that the
			// entry is gone, and this is the request not getting one at all.
			if (err instanceof OrderbookRequestError) return { kind: "failed", message: err.message }
			throw err
		}
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

	/** The book that trades this pair of symbols, whichever way round they were given. */
	private resolveBook(limits: OrderbookLimits, tokenIn: string, tokenOut: string): Book {
		const book = limits.books.find(
			(candidate) =>
				(candidate.base === tokenIn && candidate.quote === tokenOut) ||
				(candidate.quote === tokenIn && candidate.base === tokenOut),
		)
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
		// row already stored. It only answers half the question: whether the input
		// symbol is registered on that chain is the server's own config.
		const served = limits.serverInfo.chains ?? []
		if (served.length > 0) {
			const unknown = [request.fillChain, ...sources].filter((chain) => !served.includes(chain))
			if (unknown.length > 0) {
				throw new LimitOrderValidationError(
					`The orderbook does not serve ${unknown.join(", ")}. It serves: ${served.join(", ")}`,
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
			const stored = await this.store.setPosting(order.id, {
				commitment: posted.commitment,
				bookExpiresAt: posted.expiresAt,
				bookPrice: posted.price,
				orderNonce: orderNonce.toString(),
				status: "open",
				lastError: null,
			})
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
		return { order: (await this.store.setStatus(order.id, "rejected", message))!, result }
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
			this.contractService.getTokenDecimals(baseToken, order.fillChain),
			this.contractService.getTokenDecimals(quoteToken, order.fillChain),
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

function nowSecs(): number {
	return Math.floor(Date.now() / 1000)
}
