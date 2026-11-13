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
import type { OrderbookClient } from "./client"
import type { ScreenOrderResult } from "./types"

/**
 * The waits before each retry of one order's screening. Four attempts in all, about 3.5 s of
 * waiting: long enough for the orderbook's own RPC to see a placement this filler's scanner saw
 * first (`ORDER_NOT_ON_CHAIN`), short enough that a same-chain order is not held for long.
 */
export const SCREENING_BACKOFF_MS: readonly number[] = [500, 1_000, 2_000]

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
 * or beneficiary is flagged by Chainalysis, and an `unavailable` one could not be screened at all.
 * `code` is the orderbook's code for why, or `REQUEST_FAILED` when no answer came back.
 */
export type ScreeningDecision =
	| { kind: "clear" }
	| { kind: "flagged" }
	| { kind: "unavailable"; code: string; message: string }

/** Screens one swapper's order before the filler bids on it. */
export interface OrderScreening {
	screen(order: Order): Promise<ScreeningDecision>
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

/**
 * `abi.encode(order)`, the bytes the order's commitment hashes, or undefined when they do not hash
 * to `order.id`. The orderbook refuses bytes that do not match the commitment, so they are left
 * off rather than sent wrong, and the orderbook reads the order from its indexer instead.
 */
export function encodeOrder(order: Order): HexString | undefined {
	if (!ORDER_PARAMETER || !order.id) return undefined
	try {
		const encoded = encodeAbiParameters([ORDER_PARAMETER], [transformOrderForContract(order) as never])
		return keccak256(encoded).toLowerCase() === order.id.toLowerCase() ? encoded : undefined
	} catch {
		return undefined
	}
}

/**
 * Screens swappers' orders through the HyperFX orderbook's `screenOrder`, which screens the
 * order's user and beneficiary against Chainalysis and keeps the verdict for good once any solver
 * first asks.
 *
 * Each attempt is a fresh EIP-712 `ScreenOrder` signature, since the orderbook spends one once it
 * has checked it. An attempt is retried, after {@link SCREENING_BACKOFF_MS}, when it got no answer,
 * when the orderbook says a retry may succeed, or when the only objection was the timestamp
 * (`SIGNATURE_EXPIRED`, `SIGNATURE_REUSED`). Anything else is final: the order is unavailable to
 * screen and the filler drops it.
 */
export class OrderScreener implements OrderScreening {
	private logger: Logger

	constructor(
		private readonly client: Pick<OrderbookClient, "screenOrder">,
		private readonly signer: Pick<Signer, "address" | "signTypedData">,
		/** The orderbook's EIP-712 domain, as `LimitOrderService.signingDomain` reads it. */
		private readonly domain: () => Promise<{ name: string; version: string }>,
		loggers: LoggerContext = defaultLoggerContext(),
		private readonly backoffMs: readonly number[] = SCREENING_BACKOFF_MS,
	) {
		this.logger = loggers.get("screening")
	}

	async screen(order: Order): Promise<ScreeningDecision> {
		const commitment = order.id as HexString | undefined
		if (!commitment) {
			return { kind: "unavailable", code: "NO_COMMITMENT", message: "the order carries no commitment" }
		}
		const encoded = encodeOrder(order)
		if (!encoded) {
			this.logger.warn(
				{ commitment },
				"The order does not encode to its commitment; screening it without the order",
			)
		}

		let timestamp = 0
		let last: ScreeningDecision = { kind: "unavailable", code: "REQUEST_FAILED", message: "not attempted" }
		for (let attempt = 0; attempt <= this.backoffMs.length; attempt++) {
			if (attempt > 0) await sleep(this.backoffMs[attempt - 1])
			// Every attempt is signed afresh, and never in a second an earlier attempt already signed.
			timestamp = Math.max(nowSecs(), timestamp + 1)
			let result: ScreenOrderResult
			try {
				result = await this.request(commitment, timestamp, encoded)
			} catch (err) {
				last = { kind: "unavailable", code: "REQUEST_FAILED", message: describe(err) }
				this.logger.debug({ commitment, attempt, err }, "Screening request got no answer")
				continue
			}

			switch (result.kind) {
				case "screened":
					if (result.commitment.toLowerCase() !== commitment.toLowerCase()) {
						return {
							kind: "unavailable",
							code: "COMMITMENT_MISMATCH",
							message: `the orderbook answered for ${result.commitment}`,
						}
					}
					return result.flagged ? { kind: "flagged" } : { kind: "clear" }
				case "rejected":
					last = { kind: "unavailable", code: result.code, message: result.message }
					if (result.code === "SIGNATURE_EXPIRED" || result.code === "SIGNATURE_REUSED") break
					return last
				case "failed":
					last = { kind: "unavailable", code: result.code, message: result.message }
					if (result.retryable) break
					return last
			}
			this.logger.debug({ commitment, attempt, code: last.code }, "Screening request may succeed on a retry")
		}
		return last
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

function nowSecs(): number {
	return Math.floor(Date.now() / 1000)
}

function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms))
}

function describe(err: unknown): string {
	return err instanceof Error ? err.message : String(err)
}
