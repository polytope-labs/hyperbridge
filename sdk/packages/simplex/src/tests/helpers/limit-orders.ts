import type { HexString } from "@hyperbridge/sdk"
import { MemoryDataStore } from "@/data/memory"
import type { LimitOrderSide, LimitOrderStore } from "@/data/types"
import { ORDERBOOK_SCALE } from "@/orderbook/amounts"
import { LimitOrderService, type CreateLimitOrderRequest, type VaultHoldings } from "@/orderbook/limit-orders"
import type {
	CancelOrderResult,
	HeartbeatResult,
	OrderbookLimits,
	PostedOrder,
	PostedOrderPage,
	SubmitOrderResult,
} from "@/orderbook/types"

/** One resting order, written the way an operator would state it. */
export interface TestLimitOrder {
	base: string
	quote: string
	/** BID takes the base in and pays the quote out; ASK is the other way round. */
	side: LimitOrderSide
	fillChain: string
	/** Quote per 1 base, in whole units (e.g. "1500"). */
	price: string
	/** What the order will pay out in total, in whole units of the token it pays. */
	size: string
	/** Defaults to accepting the fill chain itself, which covers same-chain tests. */
	acceptedSources?: string[]
	id?: string
}

/** Whole units to the 1e18 the store and matcher work in. */
function scale(amount: string): string {
	const [whole, fraction = ""] = amount.split(".")
	const padded = (fraction + "0".repeat(18)).slice(0, 18)
	return (BigInt(whole) * ORDERBOOK_SCALE + BigInt(padded || "0")).toString()
}

/**
 * A limit order store holding the orders a test prices against.
 *
 * The filler has no prices of its own, so a test that expects a fill has to say
 * what the operator was offering, the same way a live filler would only fill
 * what a resting order covers.
 */
export async function limitOrderStore(orders: TestLimitOrder[]): Promise<LimitOrderStore> {
	const store = new MemoryDataStore().limitOrders
	for (const [index, order] of orders.entries()) {
		await store.create({
			id: order.id ?? `limit-${index}`,
			book: `${order.base}/${order.quote}`,
			base: order.base,
			quote: order.quote,
			side: order.side,
			fillChain: order.fillChain,
			price: scale(order.price),
			size: scale(order.size),
			acceptedSources: order.acceptedSources ?? [order.fillChain],
			ttlSecs: 900,
		})
	}
	return store
}

const CHAIN = "EVM-8453"
const USDC = "0x1111111111111111111111111111111111111111" as HexString
const CNGN = "0x2222222222222222222222222222222222222222" as HexString
const SOLVER = "0x3333333333333333333333333333333333333333" as HexString
const ONE = ORDERBOOK_SCALE

export const ORDERBOOK_FIXTURES = { CHAIN, USDC, CNGN, SOLVER, ONE }

export const LIMITS: OrderbookLimits = {
	serverInfo: {
		minOrderTtlSecs: 900,
		heartbeatIntervalSecs: 60,
		signatureSkewSecs: 30,
		maxBatchSize: 20,
		minOrderSizes: [{ symbol: "CNGN", size: (1000n * ONE).toString() }],
		chains: [CHAIN, "EVM-1"],
		eip712DomainName: "HyperFX Orderbook",
		eip712DomainVersion: "1",
	},
	books: [{ id: "USDC/CNGN", base: "USDC", quote: "CNGN" }],
	chains: [
		{
			id: CHAIN,
			name: "Base",
			tokens: [
				{ symbol: "USDC", decimals: 6 },
				{ symbol: "CNGN", decimals: 18 },
			],
		},
	],
}

export function postedOrder(overrides: Partial<PostedOrder> = {}): PostedOrder {
	return {
		commitment: "0xabc" as HexString,
		side: "BID",
		status: "ACTIVE",
		price: (1490n * ONE).toString(),
		quotedAmount: (1_500_000n * ONE).toString(),
		advertisedSize: (1_500_000n * ONE).toString(),
		expiresAt: "2026-09-15T12:00:00.000Z",
		acceptedSources: ["EVM-1"],
		...overrides,
	}
}

/** An orderbook that answers from a queue, and records what it was sent. */
export function fakeClient(results: SubmitOrderResult[], cancels: CancelOrderResult[] = []) {
	const submitted: HexString[] = []
	const heartbeats: number[] = []
	return {
		submitted,
		heartbeats,
		/** Entries the orderbook holds for this solver, for reconciliation to walk. */
		entries: [] as PostedOrder[],
		heartbeatResults: [] as HeartbeatResult[],
		limits: async () => LIMITS,
		orderAt: async function (_solver: HexString, commitment: HexString) {
			return this.entries.find((entry) => entry.commitment === commitment) ?? null
		},
		submitOrder: async (userOp: HexString) => {
			submitted.push(userOp)
			return results.shift() ?? { kind: "accepted" as const, order: postedOrder(), surfaced: true }
		},
		cancelOrder: async (_params: { solver: HexString; commitment: HexString; timestamp: number; signature: HexString }) =>
			cancels.shift() ?? ({ kind: "cancelled", commitment: "0xabc" as HexString } as CancelOrderResult),
		heartbeat: async function (params: { timestamp: number }): Promise<HeartbeatResult> {
			heartbeats.push(params.timestamp)
			return (
				this.heartbeatResults.shift() ?? {
					kind: "accepted",
					status: "ACTIVE",
					reactivatedOrders: 0,
					heartbeatDueBy: "2026-09-16T12:00:00.000Z",
				}
			)
		},
		myOrders: async function (): Promise<PostedOrderPage> {
			return { orders: this.entries, status: "ACTIVE" }
		},
	}
}

/**
 * A {@link LimitOrderService} over a fake orderbook, with the collaborators the
 * posting path touches stubbed down to what it reads from them.
 */
export function limitOrderService(
	client: ReturnType<typeof fakeClient>,
	store = new MemoryDataStore().limitOrders,
	/** What the wallet holds of each payout token, in whole tokens. Plenty by default. */
	balances: Record<string, bigint> = {},
	/** Where a new order's nonce starts. Pinned to 0 so the tests can name the nonces a posting walks through. */
	startingNonce: () => bigint = () => 0n,
	/** The configured vaults' holdings, when the case has any. */
	vaultBalances?: VaultHoldings,
) {
	const contractService = {
		getTokenDecimals: async (token: string) => (token === USDC ? 6 : 18),
		// Creation checks the wallet can pay out what an order promises.
		getTokenBalance: async (_chain: string, token: HexString) => {
			const decimals = token === USDC ? 6n : 18n
			const whole = balances[token] ?? 1_000_000_000n
			return whole * 10n ** decimals
		},
		// The op is opaque to the service; the nonce is echoed so a repost is visible.
		prepareLimitOrderUserOp: async ({ orderNonce }: { orderNonce: bigint }) => ({
			commitment: "0xabc" as HexString,
			userOp: `0x0${orderNonce}` as HexString,
		}),
	}
	const configService = {
		getConfiguredChainIds: () => [8453],
		getEntryPointAddress: () => "0x4444444444444444444444444444444444444444" as HexString,
	}
	const assetRegistry = {
		getAddress: (symbol: string, chain: string) =>
			chain === CHAIN ? (({ USDC, CNGN } as Record<string, HexString>)[symbol] ?? null) : null,
	}
	const signer = { address: SOLVER, signTypedData: async () => "0xsig" as HexString }

	// biome-ignore lint/suspicious/noExplicitAny: narrow stubs for the collaborators this path touches
	const service = new LimitOrderService(
		store,
		client as any,
		contractService as any,
		configService as any,
		assetRegistry as any,
		signer as any,
		900,
		undefined,
		undefined,
		startingNonce,
		vaultBalances,
	)
	return { service, store }
}

/** Take in 1,000 USDC, pay out 1,500,000 cNGN: a USDC to cNGN order at 1,500. */
export const CREATE_REQUEST: CreateLimitOrderRequest = {
	fillChain: CHAIN,
	tokenIn: "USDC",
	amountIn: "1000",
	tokenOut: "CNGN",
	amountOut: "1500000",
	acceptedSources: ["EVM-1"],
}
