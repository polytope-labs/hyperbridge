import { encodeAbiParameters, keccak256 } from "viem"
import {
	IntentGatewayABI,
	TESTNET_CHAINS,
	transformOrderForContract,
	type HexString,
	type Order,
} from "@hyperbridge/sdk"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type { Signer } from "@/services/wallet"
import { describeError, OrderbookGraphqlError, type OrderbookClient } from "./client"
import type { ScreenOrderResult } from "./types"

/**
 * How long one order's screening keeps asking, and how often.
 *
 * A screening that gets no verdict drops the order for good, and every solver drops it alike, so
 * a passing outage must not end it. It keeps asking for at least `minBudgetMs`, and for as long as
 * the order's source confirmations take when that is longer, since the order waits that long
 * anyway. The wait between attempts starts at `firstBackoffMs` and doubles up to `maxBackoffMs`.
 */
export interface ScreeningRetry {
	firstBackoffMs: number
	maxBackoffMs: number
	minBudgetMs: number
}

export const SCREENING_RETRY: ScreeningRetry = { firstBackoffMs: 500, maxBackoffMs: 8_000, minBudgetMs: 60_000 }

/** What the screener reads the time from and waits on. A test passes its own. */
export interface ScreeningClock {
	now(): number
	sleep(ms: number): Promise<void>
}

const SYSTEM_CLOCK: ScreeningClock = {
	now: () => Date.now(),
	sleep: (ms) => new Promise((resolve) => setTimeout(resolve, ms)),
}

const SCREEN_ORDER_TYPES = {
	EIP712Domain: [
		{ name: "name", type: "string" },
		{ name: "version", type: "string" },
	],
	ScreenOrder: [
		{ name: "solver", type: "address" },
		{ name: "commitment", type: "bytes32" },
		{ name: "timestamp", type: "uint64" },
	],
} as const

/**
 * What screening an order came to. Only `clear` lets an order through: a `flagged` order's user
 * or beneficiary is flagged by Elliptic, and an `unavailable` one could not be screened at all.
 * `code` is the orderbook's code for why, `REQUEST_FAILED` when no answer came back,
 * `GRAPHQL_ERROR` when the orderbook refused the request as written, or `ABORTED` when the caller
 * stopped waiting.
 */
export type ScreeningDecision =
	| { kind: "clear" }
	| { kind: "flagged" }
	| { kind: "unavailable"; code: string; message: string }

export interface ScreenOptions {
	/** How long the order would wait anyway, for its source confirmations. Retries run at least this long. */
	budgetMs?: number
	/** Stops the retries: the caller no longer needs a verdict. */
	signal?: AbortSignal
}

/** Screens one swapper's order before the filler bids on it. */
export interface OrderScreening {
	screen(order: Order, options?: ScreenOptions): Promise<ScreeningDecision>
}

/**
 * Whether an order is screened: every order except one whose source and destination are both
 * testnets. A chain the SDK does not list as a testnet counts as mainnet, so an order that cannot
 * be placed on a network is screened rather than let through.
 */
export function requiresScreening(order: Pick<Order, "source" | "destination">): boolean {
	return !(TESTNET_CHAINS.has(order.source) && TESTNET_CHAINS.has(order.destination))
}

const ORDER_PARAMETER = IntentGatewayABI.find(
	(item) => item.type === "function" && "name" in item && item.name === "placeOrder",
)?.inputs?.[0]

function abiEncodeOrder(order: Order): HexString {
	if (!ORDER_PARAMETER) throw new Error("The IntentGateway ABI has no placeOrder(Order) to encode an order with")
	return encodeAbiParameters([ORDER_PARAMETER], [transformOrderForContract(order) as never])
}

/**
 * `abi.encode(order)`, the bytes the order's commitment hashes, or undefined when they do not hash
 * to `order.id`. The orderbook refuses bytes that do not match the commitment, so they are left
 * off rather than sent wrong, and the orderbook reads the order from its indexer instead.
 */
export function encodeOrder(order: Order): HexString | undefined {
	if (!order.id) return undefined
	try {
		const encoded = abiEncodeOrder(order)
		return keccak256(encoded).toLowerCase() === order.id.toLowerCase() ? encoded : undefined
	} catch {
		return undefined
	}
}

/**
 * An order with every field set, and the commitment Foundry's `cast` computes for it, with no
 * code from this package or the SDK:
 *
 * ```
 * cast keccak $(cast abi-encode "f((bytes32,bytes,bytes,uint256,uint256,uint256,address,((bytes32,uint256)[],bytes),(bytes32,uint256)[],(bytes32,(bytes32,uint256)[],bytes)))" \
 *   "(0x…11,$(cast from-utf8 EVM-8453),$(cast from-utf8 EVM-56),123456789,7,42,0x5555…55,([(0x…66,1000)],0xdeadbeef),[(0x…22,5000000)],(0x…33,[(0x…44,99000000000000000000)],0xcafe))"
 * ```
 */
export const ORDER_ENCODING_VECTOR: { order: Order; commitment: HexString } = {
	order: {
		user: `0x${"00".repeat(12)}${"11".repeat(20)}`,
		source: "EVM-8453",
		destination: "EVM-56",
		deadline: 123_456_789n,
		nonce: 7n,
		fees: 42n,
		session: `0x${"55".repeat(20)}`,
		predispatch: {
			assets: [{ token: `0x${"00".repeat(12)}${"66".repeat(20)}`, amount: 1_000n }],
			call: "0xdeadbeef",
		},
		inputs: [{ token: `0x${"00".repeat(12)}${"22".repeat(20)}`, amount: 5_000_000n }],
		output: {
			beneficiary: `0x${"00".repeat(12)}${"33".repeat(20)}`,
			assets: [{ token: `0x${"00".repeat(12)}${"44".repeat(20)}`, amount: 99n * 10n ** 18n }],
			call: "0xcafe",
		},
	} as Order,
	commitment: "0xb112c44bbf2cc87b4120f68448709930e0c0e4b4cdd39a91cb748316ecbb46c0",
}

/**
 * Throws unless orders encode here as Solidity's `abi.encode(order)` does. Run at boot: an
 * encoding that has drifted would send every screening without its order, leaving the orderbook to
 * find each one in its indexer, so it stops the filler rather than degrading one order at a time.
 */
export function assertOrderEncoding(): void {
	const { order, commitment } = ORDER_ENCODING_VECTOR
	const hashed = keccak256(abiEncodeOrder(order))
	if (hashed !== commitment) {
		throw new Error(
			`Orders no longer encode as abi.encode(order): the reference order hashes to ${hashed}, not ${commitment}. ` +
				"Screening needs the order's exact bytes; check the SDK's IntentGateway ABI and transformOrderForContract.",
		)
	}
}

/**
 * Says at boot when the orderbook cannot screen orders, since every mainnet order is then dropped
 * one at a time with nothing else pointing at why. Only a filler with a chain the SDK does not list
 * as a testnet screens anything, so only one is told. Never rejects.
 */
export async function reportScreeningStatus(
	client: Pick<OrderbookClient, "screeningEnabled">,
	chains: string[],
	logger: Pick<Logger, "error" | "warn" | "info">,
): Promise<void> {
	if (!chains.some((chain) => requiresScreening({ source: chain, destination: chain }))) return
	try {
		if (await client.screeningEnabled()) {
			logger.info("The orderbook screens orders; mainnet orders are screened before any bid")
			return
		}
		logger.error(
			"The orderbook has screening turned off, so every mainnet order will be dropped as unscreened. " +
				"Check that [orderbook] url points at the mainnet orderbook.",
		)
	} catch (err) {
		if (err instanceof OrderbookGraphqlError) {
			logger.error(
				{ err: err.message },
				"The orderbook does not serve screening yet, so every mainnet order will be dropped as unscreened until it does",
			)
			return
		}
		logger.warn({ err: describeError(err) }, "Could not read whether the orderbook screens orders")
	}
}

/**
 * Screens swappers' orders through the HyperFX orderbook's `screenOrder`, which screens the
 * order's user and beneficiary against Elliptic and keeps the verdict for good once any solver
 * first asks.
 *
 * Each attempt is a fresh EIP-712 `ScreenOrder` signature, since the orderbook spends one once it
 * has checked it. An attempt is retried, within {@link ScreeningRetry}'s budget, when:
 * - it got no answer;
 * - the orderbook says a retry may succeed;
 * - the only objection was the timestamp (`SIGNATURE_EXPIRED`, `SIGNATURE_REUSED`);
 * - the solver had no active order on the book (`NO_ACTIVE_ORDER`). That is often a passing
 *   state: between withdrawing a limit order and posting its replacement, or until the next
 *   heartbeat brings suspended postings back.
 *
 * Anything else is final, a GraphQL error included: the orderbook read the request and refused
 * it, and would again. The order is then unavailable to screen, and the filler drops it.
 */
export class OrderScreener implements OrderScreening {
	private logger: Logger

	constructor(
		private readonly client: Pick<OrderbookClient, "screenOrder">,
		private readonly signer: Pick<Signer, "address" | "signTypedData">,
		/** The orderbook's EIP-712 domain, as `LimitOrderService.signingDomain` reads it. */
		private readonly domain: () => Promise<{ name: string; version: string }>,
		loggers: LoggerContext = defaultLoggerContext(),
		private readonly retry: ScreeningRetry = SCREENING_RETRY,
		private readonly clock: ScreeningClock = SYSTEM_CLOCK,
	) {
		this.logger = loggers.get("screening")
	}

	async screen(order: Order, options: ScreenOptions = {}): Promise<ScreeningDecision> {
		const commitment = order.id as HexString | undefined
		if (!commitment) {
			return { kind: "unavailable", code: "NO_COMMITMENT", message: "the order carries no commitment" }
		}
		const encoded = encodeOrder(order)
		if (!encoded) {
			this.logger.error(
				{ commitment },
				"The order does not encode to its commitment; screening it without the order",
			)
		}

		const deadline = this.clock.now() + Math.max(this.retry.minBudgetMs, options.budgetMs ?? 0)
		let backoff = this.retry.firstBackoffMs
		let timestamp = 0
		for (let attempt = 0; ; attempt++) {
			if (options.signal?.aborted) {
				return { kind: "unavailable", code: "ABORTED", message: "the order no longer needs a verdict" }
			}
			// Every attempt is signed afresh, and never in a second an earlier attempt already signed.
			timestamp = Math.max(Math.floor(this.clock.now() / 1000), timestamp + 1)
			let step: Step
			try {
				step = classify(commitment, await this.request(commitment, timestamp, encoded))
			} catch (err) {
				step =
					err instanceof OrderbookGraphqlError
						? final({ kind: "unavailable", code: "GRAPHQL_ERROR", message: err.message })
						: {
								decision: { kind: "unavailable", code: "REQUEST_FAILED", message: describeError(err) },
								retry: true,
							}
			}

			if (!step.retry || this.clock.now() + backoff > deadline) return step.decision
			this.logger.debug(
				{
					commitment,
					attempt,
					code: step.decision.kind === "unavailable" ? step.decision.code : undefined,
					retryInMs: backoff,
				},
				"Screening request may succeed on a retry",
			)
			await this.clock.sleep(backoff)
			backoff = Math.min(backoff * 2, this.retry.maxBackoffMs)
		}
	}

	private async request(commitment: HexString, timestamp: number, order: HexString | undefined) {
		const solver = this.signer.address as HexString
		const signature = await this.signer.signTypedData({
			domain: await this.domain(),
			types: SCREEN_ORDER_TYPES,
			primaryType: "ScreenOrder",
			message: { solver, commitment, timestamp },
		})
		return this.client.screenOrder({ solver, commitment, timestamp, signature, order })
	}
}

/** The message rejections a later attempt may get past. */
const RETRIED_REJECTIONS = new Set<string>(["SIGNATURE_EXPIRED", "SIGNATURE_REUSED", "NO_ACTIVE_ORDER"])

/** What one attempt came to, and whether another may come to more. */
interface Step {
	decision: ScreeningDecision
	retry: boolean
}

function final(decision: ScreeningDecision): Step {
	return { decision, retry: false }
}

/** The orderbook's answer as a decision, and whether it is worth asking again. */
function classify(commitment: HexString, result: ScreenOrderResult): Step {
	switch (result.kind) {
		case "screened":
			if (result.commitment.toLowerCase() !== commitment.toLowerCase()) {
				return final({
					kind: "unavailable",
					code: "COMMITMENT_MISMATCH",
					message: `the orderbook answered for ${result.commitment}`,
				})
			}
			return final(result.flagged ? { kind: "flagged" } : { kind: "clear" })
		case "rejected":
			return {
				decision: { kind: "unavailable", code: result.code, message: result.message },
				retry: RETRIED_REJECTIONS.has(result.code),
			}
		case "failed":
			return {
				decision: { kind: "unavailable", code: result.code, message: result.message },
				retry: result.retryable,
			}
	}
}
