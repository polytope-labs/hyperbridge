import type { HexString } from "@hyperbridge/sdk"
import { defaultLoggerContext, type Logger, type LoggerContext } from "@/services/Logger"
import type {
	CancelOrderResult,
	MessageRejectionCode,
	OrderbookLimits,
	PostedOrder,
	RejectionCode,
	SubmitOrderResult,
} from "./types"

const LIMITS_QUERY = `
	query Limits {
		serverInfo {
			minOrderTtlSecs
			heartbeatIntervalSecs
			signatureSkewSecs
			maxBatchSize
			minOrderSizes { symbol size }
			chains
			eip712DomainName
			eip712DomainVersion
		}
		books { id base quote }
	}
`

const POSTED_ORDER_FIELDS = "commitment side status price quotedAmount advertisedSize expiresAt acceptedSources"

const SUBMIT_ORDER_MUTATION = `
	mutation SubmitOrder($userOp: Bytes!) {
		submitOrder(userOp: $userOp) {
			__typename
			... on OrderAccepted { surfaced order { ${POSTED_ORDER_FIELDS} } }
			... on OrderUnchanged { order { ${POSTED_ORDER_FIELDS} } }
			... on OrderRejected { code message }
			... on OrderSubmissionFailed { code message retryable }
		}
	}
`

const ORDER_QUERY = `
	query OrderAt($solver: Address!, $commitment: Bytes!) {
		order(solver: $solver, commitment: $commitment) { ${POSTED_ORDER_FIELDS} }
	}
`

const CANCEL_ORDER_MUTATION = `
	mutation CancelOrder($solver: Address!, $commitment: Bytes!, $timestamp: Int!, $signature: Bytes!) {
		cancelOrder(solver: $solver, commitment: $commitment, timestamp: $timestamp, signature: $signature) {
			__typename
			... on OrderCancelled { commitment }
			... on MessageRejected { code message }
		}
	}
`

/**
 * A GraphQL error the orderbook returned, or a transport failure reaching it.
 *
 * Distinct from a rejection: a rejection is the orderbook's considered answer
 * about an order, while this is the request never getting one. Callers back off
 * and retry these; they act on rejections.
 */
export class OrderbookRequestError extends Error {}

/**
 * Talks to one HyperFX orderbook over GraphQL.
 *
 * Stateless apart from the endpoint and timeout. Every method either returns the
 * orderbook's own answer, including its refusals, or throws
 * {@link OrderbookRequestError} because it could not get one.
 */
export class OrderbookClient {
	private logger: Logger

	constructor(
		private readonly url: string,
		private readonly requestTimeoutMs: number,
		loggers: LoggerContext = defaultLoggerContext(),
	) {
		this.logger = loggers.get("orderbook")
	}

	/** The limits to validate against before posting, plus the books on offer. */
	async limits(): Promise<OrderbookLimits> {
		return this.request<OrderbookLimits>(LIMITS_QUERY, {})
	}

	async submitOrder(userOp: HexString): Promise<SubmitOrderResult> {
		const { submitOrder } = await this.request<{ submitOrder: RawSubmitOrder }>(SUBMIT_ORDER_MUTATION, { userOp })
		switch (submitOrder.__typename) {
			case "OrderAccepted":
				return { kind: "accepted", order: submitOrder.order!, surfaced: submitOrder.surfaced ?? false }
			case "OrderUnchanged":
				return { kind: "unchanged", order: submitOrder.order! }
			case "OrderRejected":
				return { kind: "rejected", code: submitOrder.code as RejectionCode, message: submitOrder.message! }
			case "OrderSubmissionFailed":
				return {
					kind: "failed",
					code: submitOrder.code!,
					message: submitOrder.message!,
					retryable: submitOrder.retryable ?? false,
				}
			default:
				throw new OrderbookRequestError(`Unknown submitOrder result ${submitOrder.__typename}`)
		}
	}

	/**
	 * The entry this solver holds at `commitment`, or null when it holds none.
	 *
	 * What tells `ORDER_EXISTS` from `REPLAYED`: the first means a live entry is
	 * already sitting there, and posting again on a new nonce would put a second
	 * one behind the same liability.
	 */
	async orderAt(solver: HexString, commitment: HexString): Promise<PostedOrder | null> {
		const { order } = await this.request<{ order: PostedOrder | null }>(ORDER_QUERY, { solver, commitment })
		return order
	}

	async cancelOrder(params: {
		solver: HexString
		commitment: HexString
		timestamp: number
		signature: HexString
	}): Promise<CancelOrderResult> {
		const { cancelOrder } = await this.request<{ cancelOrder: RawCancelOrder }>(CANCEL_ORDER_MUTATION, params)
		if (cancelOrder.__typename === "OrderCancelled") {
			return { kind: "cancelled", commitment: cancelOrder.commitment! }
		}
		if (cancelOrder.__typename === "MessageRejected") {
			return { kind: "rejected", code: cancelOrder.code as MessageRejectionCode, message: cancelOrder.message! }
		}
		throw new OrderbookRequestError(`Unknown cancelOrder result ${cancelOrder.__typename}`)
	}

	private async request<T>(query: string, variables: Record<string, unknown>): Promise<T> {
		const controller = new AbortController()
		const timer = setTimeout(() => controller.abort(), this.requestTimeoutMs)
		let response: Response
		try {
			response = await fetch(this.url, {
				method: "POST",
				headers: { "content-type": "application/json" },
				body: JSON.stringify({ query, variables }),
				signal: controller.signal,
			})
		} catch (err) {
			throw new OrderbookRequestError(`Could not reach the orderbook at ${this.url}: ${describe(err)}`)
		} finally {
			clearTimeout(timer)
		}

		if (!response.ok) {
			throw new OrderbookRequestError(`Orderbook returned HTTP ${response.status} ${response.statusText}`)
		}

		const body = (await response.json().catch((err) => {
			throw new OrderbookRequestError(`Orderbook returned a body that is not JSON: ${describe(err)}`)
		})) as { data?: T; errors?: { message: string }[] }

		if (body.errors?.length) {
			const message = body.errors.map((error) => error.message).join("; ")
			this.logger.warn({ url: this.url, message }, "Orderbook request failed")
			throw new OrderbookRequestError(message)
		}
		if (!body.data) throw new OrderbookRequestError("Orderbook returned no data")
		return body.data
	}
}

function describe(err: unknown): string {
	return err instanceof Error ? err.message : String(err)
}

interface RawSubmitOrder {
	__typename: string
	order?: PostedOrder
	surfaced?: boolean
	code?: string
	message?: string
	retryable?: boolean
}

interface RawCancelOrder {
	__typename: string
	commitment?: HexString
	code?: string
	message?: string
}
