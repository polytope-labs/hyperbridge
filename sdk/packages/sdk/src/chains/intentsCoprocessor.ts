import { ApiPromise, Keyring, WsProvider } from "@polkadot/api"
import type { ApiOptions, SubmittableExtrinsic } from "@polkadot/api/types"
import type { KeyringPair } from "@polkadot/keyring/types"
import type { RuntimeVersion } from "@polkadot/types/interfaces"
import { hexToU8a, u8aToHex, u8aConcat } from "@polkadot/util"
import { decodeAddress, keccakAsU8a, xxhashAsU8a } from "@polkadot/util-crypto"
import { numberToBytes, bytesToBigInt, decodeAbiParameters, hexToBytes } from "viem"
import { Bytes, Struct, u8, Vector } from "scale-ts"
import PQueue from "p-queue"
import type { BidSubmissionResult, HexString, PackedUserOperation, BidStorageEntry, FillerBid, Order } from "@/types"
import type { SubstrateChain } from "./substrate"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"
import { TokenBucket } from "@/utils/rateLimiter"
import { BatchingHttpProvider } from "@/utils/batchingHttpProvider"

/**
 * The key `frame_system` writes its events under: `twox_128("System") ++ twox_128("Events")`.
 *
 * Computed rather than taken from the decorated api, mirroring `system_events_storage_key` in
 * `parachain/simtests`. A plain entry's key is a pure function of its pallet and item names, so
 * there is nothing to look up and nothing to get wrong — where asking polkadot-js for it meant
 * choosing correctly between three near-identical accessors that fail in different silent ways.
 * See {@link IntentsCoprocessor.getPhantomOrdersInRange}.
 */
const SYSTEM_EVENTS_KEY = u8aToHex(u8aConcat(xxhashAsU8a("System", 128), xxhashAsU8a("Events", 128)))

/** Offchain storage key prefix for bids */
const OFFCHAIN_BID_PREFIX = new TextEncoder().encode("intents::bid::")
/** Offchain storage key prefix for phantom orders */
const OFFCHAIN_PHANTOM_PREFIX = new TextEncoder().encode("intents::phantom::order::")

/** Hyperbridge runtimes hash with keccak, so the registry needs the hasher for both spec names. */
const HYPERBRIDGE_TYPES_BUNDLE: ApiOptions["typesBundle"] = {
	spec: {
		nexus: { hasher: keccakAsU8a },
		gargantua: { hasher: keccakAsU8a },
	},
}

/** Base tip (0.001 BRIDGE) added to every submission to lift it above untipped traffic. */
const BASE_TIP = 1_000_000_000n

/** How long the HTTP api has to come up (metadata included) before the attempt is abandoned. */
const HTTP_CONNECT_TIMEOUT_MS = 20_000

/**
 * How long a submitted extrinsic has to reach a block before the attempt is treated as stalled.
 *
 * Sized against the bid window, not against how long inclusion can conceivably take: a bid is worth
 * nothing once its window closes, so an extrinsic still sitting in the pool after a few blocks is
 * better replaced by a higher-tipped copy than waited on.
 */
export const INCLUSION_TIMEOUT_MS = 20_000

/** Default phantom order poll cadence, used for every runtime except Gargantua. */
const PHANTOM_POLL_INTERVAL_MS = 15_000

/** Gargantua keeps the original one-block cadence. */
const GARGANTUA_PHANTOM_POLL_INTERVAL_MS = 6_000

/**
 * Sustained request rate allowed against a Hyperbridge HTTP endpoint, in requests per second.
 *
 * Public endpoints police the instantaneous rate — the observed limit is 10/s — and every read here
 * arrives in bursts: a poll tick fires its whole block range back-to-back, and a phantom order
 * interval fans out one offchain read per configured chain at once. Sitting under the limit rather
 * than at it leaves room for the traffic this process does not pace: the websocket, and any other
 * instance sharing the address.
 *
 * `HYPERBRIDGE_RPC_MAX_RPS` overrides it for a private endpoint with a different budget.
 */
const DEFAULT_RPC_MAX_RPS = 8

/**
 * How far behind the head a single poll tick will catch up.
 *
 * A bid window is a handful of blocks, so blocks much further back than that carry orders that can
 * no longer be bid on — scanning them faster than the chain produces them is all recovery needs to
 * do, and ten blocks a tick against the one or two produced clears a backlog quickly enough.
 *
 * Kept small even though `state_queryStorage` reads the whole range in one call, because the cost
 * moved rather than vanished: `sc-rpc` warns that its complexity is `O(|keys| * dist(from, to))` in
 * both time and memory, so a wide range is one request the node spends a long time on. It also
 * bounds the fallback, which is still one request per block.
 */
const DEFAULT_MAX_BLOCKS_PER_POLL = 10

/** Ticks to sit out after a 429, doubling per consecutive rejection. */
const MAX_RATE_LIMIT_BACKOFF_TICKS = 8

/** Buckets are per endpoint, not per instance: the limit being respected is the server's, per address. */
const rpcLimiters = new Map<string, TokenBucket>()

/**
 * The bucket pacing every request to `httpUrl`, shared by every coprocessor in this process that
 * talks to it. Several fillers in one process would otherwise each pace to the full budget and
 * collectively exceed it by their instance count.
 */
function limiterFor(httpUrl: string): TokenBucket {
	const key = new URL(httpUrl).origin
	let limiter = rpcLimiters.get(key)
	if (!limiter) {
		limiter = new TokenBucket(configuredRpcMaxRps())
		rpcLimiters.set(key, limiter)
	}
	return limiter
}

function configuredRpcMaxRps(): number {
	const raw = typeof process !== "undefined" ? process.env?.HYPERBRIDGE_RPC_MAX_RPS : undefined
	const parsed = raw === undefined ? Number.NaN : Number(raw)
	return Number.isFinite(parsed) && parsed > 0 ? parsed : DEFAULT_RPC_MAX_RPS
}

/**
 * Whether a failure is the endpoint refusing traffic for being too fast, as opposed to being down.
 *
 * polkadot-js surfaces the status in the message it throws (`[429]: Too Many Requests`), which is
 * all there is to match on — the provider does not carry the response through. Matched bracketed
 * rather than as a bare `429`, which appears in block numbers and hashes that other failures quote.
 */
function isRateLimited(err: unknown): boolean {
	const message = err instanceof Error ? err.message : String(err)
	return message.includes("[429]") || /too many requests/i.test(message)
}

/**
 * Whether a failure means the node will not serve this method at all — either it does not exist, or
 * `--rpc-methods=safe` is refusing it. `sc-rpc` answers a denied unsafe call with `MethodNotFound`
 * and the message below, so the two cases are one check and one response: stop asking.
 */
function isMethodUnavailable(err: unknown): boolean {
	const message = err instanceof Error ? err.message : String(err)
	const code = (err as { code?: number })?.code
	return code === -32601 || /method not found|unsafe to be called externally/i.test(message)
}

/**
 * Thrown when a value read as `system.events` is not a decoded event vector — which in practice
 * means polkadot-js could not resolve the storage entry's type and handed back raw bytes.
 */
class EventDecodeError extends Error {}

/**
 * Pulls the phantom order registrations out of one block's decoded `system.events`.
 *
 * Throws rather than returning nothing when handed something that is not a decoded event vector.
 * Every value here arrives through polkadot-js's storage decoding, which falls back to raw bytes
 * when it cannot resolve the entry's type instead of failing — iterating that yields numbers, and
 * the loop below would quietly find no phantom orders in a block that had them. A block reading as
 * empty is exactly the silent miss the block cursor exists to rule out, so a shape this does not
 * recognise has to be loud.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
function phantomOrdersFrom(records: any): PhantomOrderEvent[] {
	if (records == null || typeof records[Symbol.iterator] !== "function") {
		throw new EventDecodeError(`Expected a decoded event vector, got ${typeof records}`)
	}
	const orders: PhantomOrderEvent[] = []
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	for (const record of records as unknown as Array<{ event: any }>) {
		if (typeof record !== "object" || record === null || !("event" in record)) {
			throw new EventDecodeError(
				"system.events did not decode to event records — the storage entry's metadata was " +
					"probably not carried through, leaving the value as raw bytes",
			)
		}
		const { event } = record
		if (event.section !== "intentsCoprocessor" || event.method !== "PhantomOrderRegistered") continue
		const [commitment, chain, createdAt, legs] = event.data
		orders.push({
			commitment: commitment.toHex() as HexString,
			chain: new TextDecoder().decode(hexToU8a(chain.toHex())),
			createdAt: createdAt.toNumber(),
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			legs: (legs as any[]).map((leg: any) => ({
				tokenA: leg.tokenA.toHex() as HexString,
				tokenB: leg.tokenB.toHex() as HexString,
				standardAmount: BigInt(leg.standardAmount.toString()),
			})),
		})
	}
	return orders
}

/** Rejects after `ms`, without holding a node process open on its own. */
function rejectAfter(ms: number, message: string): Promise<never> {
	return new Promise((_resolve, reject) => {
		const timer = setTimeout(() => reject(new Error(message)), ms)
		;(timer as unknown as { unref?: () => void }).unref?.()
	})
}

/**
 * Maps a websocket endpoint onto the HTTP endpoint of the same node — substrate serves both on the
 * same host and port, so the scheme is the only difference. Throws for anything that is not a
 * `ws(s)://` url rather than guessing at an endpoint.
 *
 * The HTTP endpoint is always derived, never configured, because it must be the *same node* as the
 * websocket: phantom orders are read out of that node's offchain worker storage, which is
 * node-local and not replicated, so a separately configured host would return nothing for orders
 * the events said exist.
 */
export function deriveHttpUrl(wsUrl: string): string {
	if (wsUrl.startsWith("wss://")) return `https://${wsUrl.slice("wss://".length)}`
	if (wsUrl.startsWith("ws://")) return `http://${wsUrl.slice("ws://".length)}`
	throw new Error(`Cannot derive an HTTP endpoint from a non-websocket url: ${wsUrl}`)
}

/** Builds the extrinsic to submit against whichever api is live at signing time. */
type ExtrinsicBuilder = (api: ApiPromise) => SubmittableExtrinsic<"promise">

/** SCALE codec for Bid { filler: AccountId, user_op: Vec<u8> } */
const BidCodec = Struct({ filler: Bytes(32), user_op: Vector(u8) })

/**
 * SCALE codec for PackedUserOperation
 * Uses Vec<u8> for all fields to handle hex strings uniformly
 */
const PackedUserOperationCodec = Struct({
	sender: Bytes(20), // address is 20 bytes
	nonce: Bytes(32), // uint256 as 32 bytes
	initCode: Vector(u8), // variable length bytes
	callData: Vector(u8), // variable length bytes
	accountGasLimits: Bytes(32), // bytes32
	preVerificationGas: Bytes(32), // uint256 as 32 bytes
	gasFees: Bytes(32), // bytes32
	paymasterAndData: Vector(u8), // variable length bytes
	signature: Vector(u8), // variable length bytes
})

/**
 * Encodes a PackedUserOperation using SCALE codec for submission to Hyperbridge.
 * This is the recommended way to encode UserOps for the intents coprocessor.
 *
 * @param userOp - The PackedUserOperation to encode
 * @returns Hex-encoded SCALE bytes
 */
export function encodeUserOpScale(userOp: PackedUserOperation): HexString {
	const encoded = PackedUserOperationCodec.enc({
		sender: hexToU8a(userOp.sender),
		nonce: numberToBytes(userOp.nonce, { size: 32 }),
		initCode: Array.from(hexToU8a(userOp.initCode)),
		callData: Array.from(hexToU8a(userOp.callData)),
		accountGasLimits: hexToU8a(userOp.accountGasLimits),
		preVerificationGas: numberToBytes(userOp.preVerificationGas, { size: 32 }),
		gasFees: hexToU8a(userOp.gasFees),
		paymasterAndData: Array.from(hexToU8a(userOp.paymasterAndData)),
		signature: Array.from(hexToU8a(userOp.signature)),
	})

	return u8aToHex(encoded) as HexString
}

/**
 * Decodes a SCALE-encoded PackedUserOperation.
 *
 * @param hex - The hex-encoded SCALE bytes
 * @returns The decoded PackedUserOperation
 */
export function decodeUserOpScale(hex: HexString): PackedUserOperation {
	const decoded = PackedUserOperationCodec.dec(hexToU8a(hex))

	return {
		sender: u8aToHex(new Uint8Array(decoded.sender)) as HexString,
		nonce: bytesToBigInt(new Uint8Array(decoded.nonce)),
		initCode: u8aToHex(new Uint8Array(decoded.initCode)) as HexString,
		callData: u8aToHex(new Uint8Array(decoded.callData)) as HexString,
		accountGasLimits: u8aToHex(new Uint8Array(decoded.accountGasLimits)) as HexString,
		preVerificationGas: bytesToBigInt(new Uint8Array(decoded.preVerificationGas)),
		gasFees: u8aToHex(new Uint8Array(decoded.gasFees)) as HexString,
		paymasterAndData: u8aToHex(new Uint8Array(decoded.paymasterAndData)) as HexString,
		signature: u8aToHex(new Uint8Array(decoded.signature)) as HexString,
	}
}

/** RPC response shape from intents_getBidsForOrder */
interface RpcBidInfo {
	commitment: HexString
	filler: HexString
	user_op: HexString
}

/** One directed leg of a phantom order: `standardAmount` of `tokenA` quoted in `tokenB`. */
export interface PhantomOrderLeg {
	tokenA: HexString
	tokenB: HexString
	standardAmount: bigint
}

export interface PhantomOrderEvent {
	commitment: HexString
	chain: string
	createdAt: number
	/**
	 * Every configured pair expands into its configured direction plus the reverse. Listed in
	 * the same order as the order's asset lists, so index `i` here is the order's leg `i`.
	 */
	legs: PhantomOrderLeg[]
}

/** The pallet's phantom order timings, as the chain currently has them. */
export interface PhantomTimings {
	/**
	 * Blocks after registration during which the pallet accepts a bid: the `PhantomBidWindow`
	 * storage value, or the `PhantomOrderBidWindowBlocks` runtime constant when that value is zero,
	 * which is the same fallback the pallet's own `phantom_bid_window()` applies.
	 */
	bidWindowBlocks: number
	/**
	 * Blocks between generations (`PhantomOrderInterval`). Zero is meaningful rather than unset —
	 * the pallet generates once and never regenerates — so there is no constant to fall back to.
	 */
	intervalBlocks: number
}

/** One phantom bid to place, and the bid it replaces on the same chain. */
export interface PhantomBid {
	/** The phantom order commitment being bid on. */
	commitment: HexString
	/** The SCALE-encoded PackedUserOperation backing the quote. */
	userOp: HexString
	/**
	 * A live bid from a previous interval on the same chain, retracted alongside this one to
	 * reclaim its deposit. Best-effort: a retraction that fails never affects the bid.
	 */
	retractCommitment?: HexString
}

/** What became of one bid in a batch. */
export interface PhantomBidOutcome {
	commitment: HexString
	success: boolean
	/** The dispatch error that rejected this bid, when it failed. */
	error?: string
}

export interface PhantomBidBatchResult {
	/** One entry per submitted bid, in the order they were given. */
	bids: PhantomBidOutcome[]
	/** The block and extrinsic the bids landed in. */
	blockHash?: HexString
	extrinsicHash?: HexString
	/**
	 * The batch reached the pool but its inclusion was not observed, so no per-bid outcome is
	 * known. Same contract as {@link BidSubmissionResult.pending}: in flight, do not re-sign.
	 */
	pending?: boolean
	/** Set when the batch never landed at all, or when its item events could not be attributed. */
	error?: string
}

/** A submission result plus the extrinsic's own events, for callers that read per-item outcomes. */
interface SubmissionOutcome extends BidSubmissionResult {
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	events?: { event: { section: string; method: string; data: any } }[]
	/**
	 * The extrinsic reached the pool under this account's nonce and was still there when the watch
	 * timed out — the one flavour of `pending` a replacement can act on, as opposed to a rejection
	 * that bounced off a copy already pooled. Internal to the retry loop; callers see `pending`.
	 */
	stalled?: boolean
}

export interface PollPhantomOrdersOptions {
	/** How often to check for a new head. Defaults to 15s, or 6s when the runtime is Gargantua. */
	intervalMs?: number
	/**
	 * Most blocks scanned in a single poll, so a long outage catches up over several ticks instead of
	 * one burst. Defaults to 10 — several times the rate the chain produces blocks, so recovery is
	 * still quick, but small enough to bound both the work one `state_queryStorage` call asks of the
	 * node and the requests the per-block fallback queues ahead of a bid submission.
	 */
	maxBlocksPerPoll?: number
	/** Notified when a poll fails; polling continues regardless. */
	onError?: (err: unknown) => void
	/**
	 * Notified when the cursor was forced forward past a backlog too old to bid on, with the range
	 * that was never scanned. Nothing else reports it, and skipping blocks is worth a line in the log.
	 */
	onSkip?: (skipped: { from: number; to: number; head: number }) => void
}

/**
 * Service for interacting with Hyperbridge's pallet-intents coprocessor.
 * Handles bid submission and retrieval for the IntentGatewayV2 protocol.
 *
 * Can be created from an existing SubstrateChain instance to share the connection.
 */
export class IntentsCoprocessor {
	/** Cached result of whether the node exposes intents_* RPC methods */
	private hasIntentsRpc: boolean | null = null

	/** The pallet's phantom timings, read once. Cleared on failure so the read retries. */
	private phantomTimingsRead: Promise<PhantomTimings> | null = null

	/** The HTTP-backed api, connected on first use. Cleared after a failed attempt so it retries. */
	private httpApi: Promise<ApiPromise> | null = null

	/** Last runtime version read from the node, for {@link confirmedRuntimeVersion} to compare against. */
	private lastRuntimeVersion: RuntimeVersion | undefined

	/** Set once the node refuses `state_queryStorage`, so the poll stops asking for it. */
	private rangeQueryUnavailable = false

	// Serialises every extrinsic submission on this instance's substrate account. All submit/retract
	// methods funnel through signAndSendExtrinsic, each using the API's auto-nonce; fired in parallel
	// (bids for orders on different chains, or several phantom orders in one interval) they would grab
	// the same nonce and all but one would fail. Concurrency 1 sequences them.
	private submissionQueue = new PQueue({ concurrency: 1 })

	/**
	 * Creates and connects an IntentsCoprocessor to a Hyperbridge node.
	 * This creates and manages its own API connection.
	 *
	 * @param wsUrl - WebSocket URL of the Hyperbridge node
	 * @param substratePrivateKey - Private key for signing extrinsics (optional for read-only operations)
	 * @returns Promise resolving to a connected IntentsCoprocessor
	 */
	static async connect(wsUrl: string, substratePrivateKey?: string): Promise<IntentsCoprocessor> {
		const api = await ApiPromise.create({
			provider: new WsProvider(wsUrl),
			typesBundle: HYPERBRIDGE_TYPES_BUNDLE,
		})
		return new IntentsCoprocessor(api, substratePrivateKey, true)
	}

	/**
	 * Creates an IntentsCoprocessor from an existing SubstrateChain instance.
	 * This shares the connection - the SubstrateChain must already be connected.
	 *
	 * @param chain - Connected SubstrateChain instance (typically Hyperbridge)
	 * @param substratePrivateKey - Private key for signing extrinsics (optional for read-only operations)
	 */
	static fromSubstrateChain(chain: SubstrateChain, substratePrivateKey?: string): IntentsCoprocessor {
		if (!chain.api) {
			throw new Error("SubstrateChain must be connected before creating IntentsCoprocessor")
		}
		return new IntentsCoprocessor(chain.api, substratePrivateKey, false)
	}

	/**
	 * Creates an IntentsCoprocessor from an existing ApiPromise instance.
	 *
	 * @param api - Connected ApiPromise instance
	 * @param substratePrivateKey - Private key for signing extrinsics (optional for read-only operations)
	 */
	static fromApi(api: ApiPromise, substratePrivateKey?: string): IntentsCoprocessor {
		return new IntentsCoprocessor(api, substratePrivateKey, false)
	}

	private constructor(
		private api: ApiPromise,
		private substratePrivateKey?: string,
		private ownsConnection: boolean = false,
	) {}

	/**
	 * The API every RPC query runs on: HTTP, connected to the same node as the websocket. Exposed so
	 * callers query through this connection rather than opening one of their own.
	 *
	 * The split is by what each transport is for. Queries are one-shot request/response, which HTTP
	 * serves without holding any state that can silently rot between calls. The websocket earns its
	 * keep only where subscriptions do — watching a submitted extrinsic to inclusion.
	 */
	async queryApi(): Promise<ApiPromise> {
		return await this.http()
	}

	/**
	 * The websocket API, exposed so callers share this one connection instead of opening a second
	 * socket to the same node. Only needed for subscriptions; use {@link queryApi} to read.
	 */
	get apiConnection(): ApiPromise {
		return this.api
	}

	/**
	 * Disconnects the underlying API connection if this instance owns it.
	 * Only disconnects the websocket if created via `connect()`, not when using shared connections.
	 * The HTTP api is always created here, so it is always ours to close.
	 */
	async disconnect(): Promise<void> {
		const http = this.httpApi
		this.httpApi = null
		if (http) {
			// A creation that never settled must not turn shutdown into an unhandled rejection.
			await http.then((api) => api.disconnect()).catch(() => {})
		}
		if (this.ownsConnection) {
			await this.api.disconnect()
		}
	}

	/**
	 * The HTTP api for this node, connected on first use. Every coprocessor has one — the endpoint
	 * is derived from the websocket's own endpoint, so there is nothing to configure and nothing to
	 * be absent.
	 *
	 * The connection attempt is bounded on both sides. `isReadyOrError` rejects on a failed
	 * handshake, where plain `isReady` would simply never resolve, and the timeout covers an
	 * endpoint that accepts the request and then goes quiet. An unbounded wait here would hang the
	 * poll tick awaiting it, which is precisely the silent stall that polling exists to avoid. A
	 * failed attempt is not cached, so the next call tries again.
	 */
	private async http(): Promise<ApiPromise> {
		if (!this.httpApi) {
			const httpUrl = deriveHttpUrl(this.wsEndpoint())
			const api = new ApiPromise({
				// Response cache off (third argument, capacity 0). polkadot-js caches every request that
				// names a block hash — `chain_getHeader(hash)`, `state_getRuntimeVersion(hash)`, storage
				// reads at a hash — by storing the request promise itself, a rejected one included, for a
				// 30s TTL that every hit refreshes. The phantom poll retries the block it failed on with
				// identical parameters every tick, so one reset connection became the same rejection
				// replayed from memory on every tick, faster than the TTL could lapse, and the node never
				// saw a second request. The cache bought nothing here anyway: the poll reads each block
				// once, and `api.at(hash)` reuses registries at the api layer regardless.
				// Concurrent calls are coalesced into one JSON-RPC batch request, and every request
				// to this endpoint is paced by a bucket shared with any other coprocessor in this
				// process pointed at the same host — the limit is the server's, and it counts
				// requests per address rather than per connection.
				provider: new BatchingHttpProvider(httpUrl, {}, limiterFor(httpUrl)),
				typesBundle: HYPERBRIDGE_TYPES_BUNDLE,
				// A second connection to the node the ws api already reported on; its init warnings
				// would just be duplicates.
				noInitWarn: true,
			})
			this.httpApi = Promise.race([
				api.isReadyOrError,
				rejectAfter(HTTP_CONNECT_TIMEOUT_MS, `HTTP RPC ${httpUrl} did not become ready`),
			]).catch(async (err) => {
				// Nothing else holds this half-open api; leaving it would keep retrying underneath.
				await api.disconnect().catch(() => {})
				this.httpApi = null
				// polkadot-js reports a failed init as a bare "fetch failed", which says nothing
				// about where it was fetching from.
				throw new Error(`HTTP RPC ${httpUrl} is unavailable: ${err instanceof Error ? err.message : err}`)
			})
		}
		return await this.httpApi
	}

	/**
	 * The endpoint the websocket provider is connected to. Read from the provider rather than
	 * remembered from a constructor argument, so it is the one endpoint in use no matter which
	 * factory built this instance.
	 */
	private wsEndpoint(): string {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const endpoint = ((this.api as any)._rpcCore?.provider as { endpoint?: string } | undefined)?.endpoint
		if (!endpoint) {
			throw new Error("Cannot determine the Hyperbridge websocket endpoint to derive an HTTP endpoint from")
		}
		return endpoint
	}

	/**
	 * Creates a Substrate keypair from the configured private key.
	 * Supports hex seed (with or without 0x), mnemonic phrases, and URI derivation paths (//Alice).
	 */
	public getKeyPair(): KeyringPair {
		if (!this.substratePrivateKey) {
			throw new Error("Substrate PrivateKey Required")
		}

		const keyring = new Keyring({ type: "sr25519" })

		if (this.substratePrivateKey.startsWith("//")) {
			return keyring.addFromUri(this.substratePrivateKey)
		}
		if (this.substratePrivateKey.includes(" ")) {
			return keyring.addFromMnemonic(this.substratePrivateKey)
		}
		const hex = this.substratePrivateKey.startsWith("0x")
			? this.substratePrivateKey.slice(2)
			: this.substratePrivateKey
		const seedBytes = Buffer.from(hex, "hex")
		return keyring.addFromSeed(seedBytes)
	}

	/**
	 * Signs and sends an extrinsic. Submissions are serialised through {@link submissionQueue} so
	 * concurrent calls never collide on the substrate account nonce — each extrinsic reaches a block
	 * (or is confirmed still pooled and returned as `pending`) before the next is signed. The
	 * auto-nonce is the account's on-chain nonce, so a still-pooled extrinsic does not advance it: a
	 * submission signed behind a pending one bounces off it (1013/1014) and is reported as pending
	 * too, rather than landing as a second copy.
	 *
	 * The extrinsic is built rather than passed in because the api it is built on decides where it
	 * is signed and sent: a websocket that is down when the queue reaches this submission diverts it
	 * to {@link sendViaHttp}, which needs the call bound to the HTTP api instead.
	 */
	private async signAndSendExtrinsic(
		build: ExtrinsicBuilder,
		maxRetries: number = 3,
		timeoutMs: number = INCLUSION_TIMEOUT_MS,
	): Promise<SubmissionOutcome> {
		const result = await this.submissionQueue.add(async () => {
			// Checked here, not at call time: the queue may have held this submission for a while.
			if (!this.api.isConnected) {
				try {
					return await this.sendViaHttp(await this.http(), build)
				} catch (err) {
					return { success: false, error: err instanceof Error ? err.message : String(err) }
				}
			}
			return await this.sendExtrinsicWithRetries(build(this.api), maxRetries, timeoutMs)
		})
		return result ?? { success: false, error: "Submission queue returned no result" }
	}

	/**
	 * Last-resort submission for when the websocket is down at signing time. A bid is only worth
	 * anything inside its window, so waiting for a reconnect usually means not bidding at all.
	 *
	 * HTTP has no subscriptions, so this is `author_submitExtrinsic`: the node accepts the extrinsic
	 * into its pool and returns its hash, and nothing further is observable from here. That is
	 * exactly the `pending` contract — in flight, outcome unknown, do not re-sign — so the result
	 * says so rather than claiming a success it cannot see.
	 *
	 * Only reached when the socket was already down before signing. A submission that got as far as
	 * the pool over the websocket is never retried here: that is the duplicate-nonce race the
	 * `pending` result exists to prevent.
	 */
	private async sendViaHttp(api: ApiPromise, build: ExtrinsicBuilder): Promise<BidSubmissionResult> {
		try {
			const hash = await build(api).signAndSend(this.getKeyPair(), { tip: BASE_TIP })
			return { success: false, pending: true, extrinsicHash: hash.toHex() as HexString }
		} catch (err) {
			return this.classifySubmissionError(err instanceof Error ? err : new Error(String(err)))
		}
	}

	/**
	 * Signs and sends an extrinsic, handling status updates and errors.
	 * Implements retry logic with progressive tip increases for stuck transactions.
	 *
	 * Two kinds of failure are retried, and the difference is the nonce.
	 *
	 * An attempt that verifiably went nowhere (rejected before the pool, dropped, invalid) leaves
	 * the account nonce free, so the next attempt simply re-signs with the auto-nonce.
	 *
	 * An attempt that reached the pool and was still there when the watch timed out (`stalled`) is
	 * retried as a *replacement*: the same nonce it was signed with, and double the tip. Substrate's
	 * pool evicts a pooled extrinsic in favour of a higher-priority one at the same (account, nonce),
	 * so exactly one of the two can ever execute. This matters for a bid, which is worth nothing once
	 * its window closes — waiting out a stalled extrinsic usually means not bidding at all.
	 *
	 * Re-signing without pinning the nonce is what must never happen here. The auto-nonce is read
	 * from on-chain state, so it is only the stalled extrinsic's nonce for as long as that extrinsic
	 * stays out of a block — and a stall is precisely the case where it may land at any moment. Once
	 * it does, an unpinned retry takes the *next* nonce and both execute: a duplicate `placeBid` that
	 * fails on-chain, and a second `retractBid` that pulls the bid just placed. When the signed nonce
	 * cannot be read, the stalled result is returned rather than guessed at.
	 *
	 * A rejection that bounced off a copy already pooled (1013/1014) is likewise left alone: that
	 * copy is in flight and its outcome is unknown here, so the `pending` result goes back to the
	 * caller to confirm later.
	 */
	private async sendExtrinsicWithRetries(
		extrinsic: SubmittableExtrinsic<"promise">,
		maxRetries: number,
		timeoutMs: number,
	): Promise<SubmissionOutcome> {
		const keyPair = this.getKeyPair()
		let attempt = 0
		// Set once an attempt stalls in the pool, so every later attempt replaces it instead of
		// queueing behind it.
		let nonce: number | undefined
		let stalled: SubmissionOutcome | undefined

		while (attempt < maxRetries) {
			const currentTip = BASE_TIP * BigInt(2 ** attempt) // Double tip on each retry
			attempt++

			try {
				const result = await this.sendWithTimeout(extrinsic, keyPair, currentTip, timeoutMs, nonce)
				if (result.success || result.error?.includes("Dispatch error")) {
					// Return immediately on success or dispatch errors (non-recoverable)
					return result
				}
				if (result.stalled) {
					stalled = result
					nonce ??= this.signedNonce(extrinsic)
					// Without the nonce the extrinsic went out under, a retry cannot be a replacement.
					if (nonce === undefined) return result
					continue
				}
				// A copy of this nonce is already pooled — in flight, not ours to replace. When that
				// copy is the attempt that stalled here, its own result is the one to report: it
				// carries the extrinsic hash the caller would look up.
				if (result.pending) return stalled ?? result
			} catch (err) {
				// Unexpected error. A stalled extrinsic is still in flight whatever went wrong on
				// this attempt, so it is reported as pending rather than as a failure.
				return (
					stalled ?? {
						success: false,
						error: err instanceof Error ? err.message : "Unknown error",
					}
				)
			}
		}

		// A stalled extrinsic that outlived every bump is still in flight, so it is reported as
		// pending — the caller must not re-sign it.
		return (
			stalled ?? {
				success: false,
				error: `Transaction failed after ${maxRetries} attempts`,
			}
		)
	}

	/**
	 * The nonce an extrinsic was signed with, or undefined if it carries no readable one — which is
	 * the case before it has ever been signed, and for a stub api in tests.
	 */
	private signedNonce(extrinsic: SubmittableExtrinsic<"promise">): number | undefined {
		try {
			const nonce = (extrinsic as unknown as { nonce?: { toNumber?: () => number } }).nonce?.toNumber?.()
			return typeof nonce === "number" && Number.isFinite(nonce) ? nonce : undefined
		} catch {
			return undefined
		}
	}

	/**
	 * Classifies a submission rejection. Codes 1013 ("already imported") and 1014 ("priority is
	 * too low") both mean a copy of this account+nonce is already in the pool — almost always our
	 * own earlier attempt whose watch handle didn't confirm cleanly. That extrinsic is in flight;
	 * resubmitting can only bounce again or land a duplicate, so these are surfaced as `pending`
	 * rather than failure.
	 */
	private classifySubmissionError(err: Error): BidSubmissionResult {
		const code = (err as { code?: number }).code
		const inFlight =
			code === 1013 ||
			code === 1014 ||
			/already imported|already in the pool|priority is too low/i.test(err.message)
		return { success: false, pending: inFlight || undefined, error: err.message }
	}

	/**
	 * Sends an extrinsic with a timeout.
	 *
	 * A timeout is only a failure when the extrinsic never made it into the transaction pool.
	 * Once a pool-entry status (Future/Ready/Broadcast/Retracted) has been seen, the extrinsic is
	 * in flight and may well execute after the watch is abandoned — the result is then `pending`
	 * and `stalled`, telling the caller to replace it under the same nonce or confirm it later,
	 * never to re-sign the same call under a fresh one.
	 *
	 * `nonce` pins the submission to a specific account nonce, which is what makes a retry a pool
	 * replacement rather than a second extrinsic queued behind the first. Left undefined on the
	 * first attempt, where the api's auto-nonce is correct.
	 */
	private async sendWithTimeout(
		extrinsic: SubmittableExtrinsic<"promise">,
		keyPair: KeyringPair,
		tip: bigint,
		timeoutMs: number,
		nonce?: number,
	): Promise<SubmissionOutcome> {
		return new Promise<SubmissionOutcome>((resolve) => {
			let resolved = false
			let unsubscribe: (() => void) | null = null
			let enteredPool = false

			// Set timeout to detect stuck transactions
			const timeoutId = setTimeout(() => {
				if (!resolved) {
					resolved = true
					if (unsubscribe) {
						unsubscribe()
					}
					resolve({
						success: false,
						pending: enteredPool || undefined,
						stalled: enteredPool || undefined,
						extrinsicHash: enteredPool ? (extrinsic.hash.toHex() as HexString) : undefined,
						error: `Transaction timed out after ${timeoutMs}ms${enteredPool ? " while in the transaction pool" : ""}`,
					})
				}
			}, timeoutMs)

			extrinsic
				.signAndSend(keyPair, nonce === undefined ? { tip } : { tip, nonce }, (result) => {
					if (resolved) return

					if (
						result.status.isFuture ||
						result.status.isReady ||
						result.status.isBroadcast ||
						result.status.isRetracted
					) {
						enteredPool = true
					}

					if (result.dispatchError && (result.status.isInBlock || result.status.isFinalized)) {
						resolved = true
						clearTimeout(timeoutId)
						resolve({
							success: false,
							error: `Dispatch error: ${this.describeDispatchError(result.dispatchError)}`,
						})
					} else if (
						result.status.isDropped ||
						result.status.isInvalid ||
						result.status.isUsurped ||
						result.status.isFinalityTimeout
					) {
						// Pool-level terminal statuses — don't retry, let caller decide
						resolved = true
						clearTimeout(timeoutId)
						resolve({
							success: false,
							error: `Transaction ${result.status.type.toLowerCase()}`,
						})
					} else if (result.status.isInBlock || result.status.isFinalized) {
						resolved = true
						clearTimeout(timeoutId)

						// A utility.batch extrinsic is itself `Ok` even when one of its calls fails: the
						// failure surfaces as a BatchInterrupted event, NOT a top-level dispatchError.
						// BatchInterrupted { index, error } means calls [0, index) succeeded and the call
						// at `index` failed (the rest are skipped). We order batches so the primary call
						// is first, so an interruption at index 0 means nothing meaningful landed -> report
						// failure. A later index means the primary call succeeded and only a trailing
						// best-effort call (e.g. a deposit retraction) was skipped, which is still success.
						const interrupted = result.events.find(
							({ event }) => event.section === "utility" && event.method === "BatchInterrupted",
						)
						if (interrupted) {
							// eslint-disable-next-line @typescript-eslint/no-explicit-any
							const [indexCodec, dispatchError] = interrupted.event.data as any
							if (Number(indexCodec.toString()) === 0) {
								resolve({
									success: false,
									error: `Dispatch error: ${this.describeDispatchError(dispatchError)}`,
								})
								return
							}
						}

						resolve({
							success: true,
							blockHash: (result.status.isInBlock
								? result.status.asInBlock
								: result.status.asFinalized
							).toHex() as HexString,
							extrinsicHash: extrinsic.hash.toHex() as HexString,
							// Carried so a batch caller can attribute each item's outcome.
							events: result.events,
						})
					}
				})
				.then((unsub) => {
					if (resolved) {
						unsub()
					} else {
						unsubscribe = unsub
					}
				})
				.catch((err: Error) => {
					if (!resolved) {
						resolved = true
						clearTimeout(timeoutId)
						resolve(this.classifySubmissionError(err))
					}
				})
		})
	}

	/**
	 * Submits a bid to Hyperbridge's pallet-intents
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @param userOp - The encoded PackedUserOperation as hex string
	 * @returns BidSubmissionResult with success status and block/extrinsic hash
	 */
	async submitBid(commitment: HexString, userOp: HexString): Promise<BidSubmissionResult> {
		try {
			return await this.signAndSendExtrinsic((api) => api.tx.intentsCoprocessor.placeBid(commitment, userOp))
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	/**
	 * Retracts a bid from Hyperbridge and reclaims the deposit
	 *
	 * Use this to remove unused quotes and claim back deposited BRIDGE tokens.
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @returns BidSubmissionResult with success status and block/extrinsic hash
	 */
	async retractBid(commitment: HexString): Promise<BidSubmissionResult> {
		try {
			return await this.signAndSendExtrinsic((api) => api.tx.intentsCoprocessor.retractBid(commitment))
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	/**
	 * Places a new bid and retracts a previous one in a single transaction via utility.batch.
	 *
	 * The new bid is the primary operation, so `placeBid` MUST run first. `utility.batch` is
	 * non-atomic: a failing call interrupts the batch (via a BatchInterrupted event) without
	 * reverting the calls that already succeeded. Placing first guarantees the new bid lands even
	 * when the retraction then fails — which it routinely does, because a previous commitment's bid
	 * may already be gone (or was itself never placed), making `retractBid` return `BidNotFound`.
	 *
	 * Ordering retraction first (the previous behaviour) caused a self-sustaining cascade: a
	 * `BidNotFound` on the leading retract skipped the trailing `placeBid`, so the current bid never
	 * landed, so the *next* interval's retract of that never-placed commitment also failed, and so
	 * on — silently, because the batch extrinsic itself reports success. The deposit reclaim is
	 * best-effort; landing the bid is not.
	 *
	 * @param retractCommitment - The order commitment of the bid to retract (bytes32)
	 * @param bidCommitment - The order commitment of the new bid (bytes32)
	 * @param userOp - The encoded PackedUserOperation as hex string
	 * @returns BidSubmissionResult with success status and block/extrinsic hash
	 */
	async submitBidWithRetraction(
		retractCommitment: HexString,
		bidCommitment: HexString,
		userOp: HexString,
	): Promise<BidSubmissionResult> {
		try {
			return await this.signAndSendExtrinsic((api) =>
				api.tx.utility.batch([
					api.tx.intentsCoprocessor.placeBid(bidCommitment, userOp),
					api.tx.intentsCoprocessor.retractBid(retractCommitment),
				]),
			)
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	/**
	 * Places every phantom bid of one interval in a single extrinsic, retracting each chain's
	 * previous bid alongside it.
	 *
	 * The pallet registers one phantom order per configured chain in the same block, so this is the
	 * whole interval's set. Submitting them one at a time costs a block per chain: submissions are
	 * serialised on the account nonce and each waits for inclusion, so the last chain's bid is many
	 * blocks behind the first, against a bid window measured in tens of blocks. Batched, every bid
	 * lands in the same block.
	 *
	 * Uses `utility.force_batch`, not `utility.batch`. `batch` stops at the first failing call, so
	 * one rejected bid — a closed window, a duplicate, an insufficient deposit — would silently
	 * drop every bid after it. `force_batch` runs them all and reports each outcome. It needs no
	 * special origin: any signed account may call it, exactly like `batch`.
	 *
	 * @param bids - The bids to place; an empty list is a no-op
	 * @returns Per-bid outcomes, in the order given
	 */
	async submitPhantomBids(bids: PhantomBid[]): Promise<PhantomBidBatchResult> {
		if (bids.length === 0) return { bids: [] }

		// Call order is the only key the item events give us, so record where each bid's placeBid
		// sits as the calls are built.
		const placeIndexByBid: number[] = []
		let callCount = 0
		for (const bid of bids) {
			placeIndexByBid.push(callCount)
			callCount += bid.retractCommitment ? 2 : 1
		}

		const outcome = await this.signAndSendExtrinsic((api) =>
			api.tx.utility.forceBatch(
				bids.flatMap((bid) => {
					const calls = [api.tx.intentsCoprocessor.placeBid(bid.commitment, bid.userOp)]
					// Placed first so the pairing reads the same as the call order; with force_batch
					// the ordering no longer decides whether the bid survives a failed retraction.
					if (bid.retractCommitment) {
						calls.push(api.tx.intentsCoprocessor.retractBid(bid.retractCommitment))
					}
					return calls
				}),
			),
		)

		if (!outcome.success) {
			// Nothing landed, so there is nothing to attribute: every bid shares the batch's fate.
			return {
				bids: bids.map((bid) => ({ commitment: bid.commitment, success: false, error: outcome.error })),
				pending: outcome.pending,
				extrinsicHash: outcome.extrinsicHash,
				error: outcome.error,
			}
		}

		const items = this.readForceBatchItems(outcome.events ?? [], callCount)

		return {
			bids: bids.map((bid, index) => {
				const error = items.errors[placeIndexByBid[index]]
				return { commitment: bid.commitment, success: !error, error }
			}),
			blockHash: outcome.blockHash,
			extrinsicHash: outcome.extrinsicHash,
			error: items.error,
		}
	}

	/**
	 * Reads one outcome per call out of a force_batch's events.
	 *
	 * `ItemFailed` carries no index — pallet-utility emits exactly one `ItemCompleted` or
	 * `ItemFailed` per call, in call order, so the k-th item event belongs to call k.
	 *
	 * A count that does not match the calls submitted means the events are not the ones assumed
	 * here, and every attribution after the discrepancy would be off by one. The bids are then
	 * reported as placed: a bid wrongly recorded as landed is retracted next interval and the
	 * retraction harmlessly fails with `BidNotFound`, whereas one wrongly recorded as failed is
	 * never retracted at all and leaves its deposit reserved.
	 */
	private readForceBatchItems(
		events: NonNullable<SubmissionOutcome["events"]>,
		callCount: number,
	): { errors: (string | undefined)[]; error?: string } {
		const errors: (string | undefined)[] = []
		for (const { event } of events) {
			if (event.section !== "utility") continue
			if (event.method === "ItemCompleted") errors.push(undefined)
			else if (event.method === "ItemFailed") errors.push(this.describeDispatchError(event.data[0]))
		}

		if (errors.length !== callCount) {
			return {
				errors: new Array(callCount).fill(undefined),
				error: `force_batch reported ${errors.length} item events for ${callCount} calls; outcomes not attributed`,
			}
		}
		return { errors }
	}

	/** Renders a DispatchError as `pallet::Error`, falling back to its raw form. */
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	private describeDispatchError(dispatchError: any): string {
		if (dispatchError?.isModule) {
			const decoded = this.api.registry.findMetaError(dispatchError.asModule)
			return `${decoded.section}::${decoded.name}`
		}
		return dispatchError?.toString() ?? "unknown dispatch error"
	}

	/**
	 * Fetches all bid storage entries for a given order commitment.
	 * Returns the on-chain data only (filler addresses and deposits).
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @returns Array of BidStorageEntry objects
	 */
	async getBidStorageEntries(commitment: HexString): Promise<BidStorageEntry[]> {
		const api = await this.http()
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const entries = await (api.query.intentsCoprocessor.bids as any).entries(commitment)

		return entries.map(([storageKey, depositValue]: [any, any]) => ({
			commitment,
			filler: storageKey.args[1].toString() as string,
			deposit: BigInt(depositValue.toString()),
		}))
	}

	/**
	 * Fetches all bids for a given order commitment from Hyperbridge.
	 *
	 * Uses the custom intents_getBidsForOrder RPC if available on the node
	 * for a single round-trip. Falls back to parallel storage + offchain
	 * lookups otherwise.
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @returns Array of FillerBid objects containing filler address, userOp, and deposit
	 */
	async getBidsForOrder(commitment: HexString): Promise<FillerBid[]> {
		try {
			return await this.getBidsViaRpc(commitment)
		} catch (err) {
			console.warn("intents RPC failed, falling back to storage queries:", err)
		}

		return await this.getBidsViaStorage(commitment)
	}

	/**
	 * Fetches bids using the custom intents_getBidsForOrder RPC.
	 * Single round-trip but does not include deposit amounts.
	 */
	private async getBidsViaRpc(commitment: HexString): Promise<FillerBid[]> {
		const api = await this.http()
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const result: RpcBidInfo[] = await (api as any)._rpcCore.provider.send("intents_getBidsForOrder", [commitment])

		return result.map((entry) => {
			const userOp = decodeUserOpScale(entry.user_op as HexString)
			const filler = new Keyring({ type: "sr25519" }).encodeAddress(hexToU8a(entry.filler))
			return { filler, userOp, deposit: 0n }
		})
	}

	/**
	 * Fetches bids using on-chain storage entries + parallel offchain lookups.
	 * Slower but works on all nodes and includes deposit amounts.
	 */
	private async getBidsViaStorage(commitment: HexString): Promise<FillerBid[]> {
		const api = await this.http()
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const entries = await (api.query.intentsCoprocessor.bids as any).entries(commitment)

		if (entries.length === 0) return []

		const bidPromises = entries.map(async ([storageKey, depositValue]: [any, any]) => {
			try {
				const filler = storageKey.args[1].toString()
				const deposit = BigInt(depositValue.toString())

				const offchainKey = this.buildOffchainBidKey(commitment, filler)
				const offchainKeyHex = u8aToHex(offchainKey)

				const offchainResult = await api.rpc.offchain.localStorageGet("PERSISTENT", offchainKeyHex)

				if (!offchainResult || offchainResult.isNone) return null

				const bidData = offchainResult.unwrap().toHex() as HexString
				const decoded = this.decodeBid(bidData)

				return { filler: decoded.filler, userOp: decoded.userOp, deposit }
			} catch {
				return null
			}
		})

		const results = await Promise.all(bidPromises)
		return results.filter((b): b is FillerBid => b !== null)
	}

	/** Decodes SCALE-encoded Bid struct and SCALE-encoded PackedUserOperation */
	private decodeBid(hex: HexString): { filler: string; userOp: PackedUserOperation } {
		const decoded = BidCodec.dec(hexToU8a(hex))
		const filler = new Keyring({ type: "sr25519" }).encodeAddress(new Uint8Array(decoded.filler))
		const userOpHex = u8aToHex(new Uint8Array(decoded.user_op)) as HexString

		// Decode UserOp using SCALE codec
		const userOp = decodeUserOpScale(userOpHex)

		return { filler, userOp }
	}

	/** Builds offchain storage key: "intents::bid::" + commitment + filler */
	private buildOffchainBidKey(commitment: HexString, filler: string): Uint8Array {
		return u8aConcat(OFFCHAIN_BID_PREFIX, hexToU8a(commitment), decodeAddress(filler))
	}

	/**
	 * Fetches the ABI-encoded phantom order from offchain storage and decodes it
	 * into an `Order` object. The pallet writes the order bytes under the key
	 * `intents::phantom::order::<commitment>` when it calls `on_initialize`.
	 *
	 * Returns `null` if the key is absent (e.g. the node is not an offchain worker
	 * or the commitment has expired and been cleared).
	 */
	async fetchPhantomOrder(commitment: HexString): Promise<Order | null> {
		const key = u8aConcat(OFFCHAIN_PHANTOM_PREFIX, hexToU8a(commitment))
		const api = await this.http()
		const result = await api.rpc.offchain.localStorageGet("PERSISTENT", u8aToHex(key))
		if (!result || result.isNone) return null

		const rawHex = result.unwrap().toHex() as HexString
		if (rawHex === "0x" || rawHex === "0x00") return null

		const placeOrderAbi = (
			IntentGatewayV2.ABI as readonly { type: string; name?: string; inputs?: unknown[] }[]
		).find((item) => item.type === "function" && item.name === "placeOrder")
		const orderType = placeOrderAbi?.inputs?.[0]
		if (!orderType) return null

		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const [decoded] = decodeAbiParameters([orderType as any], rawHex)
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const d = decoded as any
		const textDecoder = new TextDecoder()

		return {
			id: commitment,
			user: d.user as HexString,
			source: textDecoder.decode(hexToBytes(d.source as HexString)),
			destination: textDecoder.decode(hexToBytes(d.destination as HexString)),
			deadline: d.deadline as bigint,
			nonce: d.nonce as bigint,
			fees: d.fees as bigint,
			session: d.session as HexString,
			predispatch: {
				assets: (d.predispatch.assets as { token: HexString; amount: bigint }[]).map((a) => ({
					token: a.token,
					amount: a.amount,
				})),
				call: d.predispatch.call as HexString,
			},
			inputs: (d.inputs as { token: HexString; amount: bigint }[]).map((i) => ({
				token: i.token,
				amount: i.amount,
			})),
			output: {
				beneficiary: d.output.beneficiary as HexString,
				assets: (d.output.assets as { token: HexString; amount: bigint }[]).map((a) => ({
					token: a.token,
					amount: a.amount,
				})),
				call: d.output.call as HexString,
			},
		}
	}

	/**
	 * Reads the PhantomOrderRegistered events emitted in a single block.
	 *
	 * Costs two RPCs per block when `knownVersion` is supplied and four without it, which is why the
	 * poll goes to the trouble of establishing one. `api.at(hash)` has to work out which metadata to
	 * decode the block against, and with nothing to go on it fetches the header and then the runtime
	 * version at its parent — every block, forever. Its cheaper paths are a registry already pinned
	 * to this exact hash (only ever the previous block's) or one matching a version the caller
	 * names, so naming the version is the only way out. See `getBlockRegistry` in
	 * `@polkadot/api/base/Init`; the `getUpgradeVersion` shortcut that would otherwise skip the
	 * lookup only covers chains hardcoded in `@polkadot/types-known`, which Hyperbridge is not.
	 *
	 * @param knownVersion - the runtime version this block is known to run, if the caller has
	 *   established one. Passing a version the block does not actually run decodes it against the
	 *   wrong metadata, so this is for callers that have checked, not a place to pass a guess.
	 */
	async getPhantomOrdersInBlock(blockNumber: number, knownVersion?: RuntimeVersion): Promise<PhantomOrderEvent[]> {
		const api = await this.http()
		const blockHash = await api.rpc.chain.getBlockHash(blockNumber)
		return await this.getPhantomOrdersAtHash(blockHash.toHex() as HexString, knownVersion)
	}

	/**
	 * The same read, for a caller that already holds the block's hash.
	 *
	 * Split out so the poll can fetch a whole range's hashes in one concurrent wave — which the
	 * provider coalesces into a single batched request — and then read each block's events knowing
	 * its hash. `chain_getBlockHash` is the half of the pair that parallelises safely: it takes no
	 * historic block hash, so it never triggers polkadot-js's per-hash registry resolution, and
	 * concurrent calls cannot race each other's registry state.
	 */
	async getPhantomOrdersAtHash(blockHash: HexString, knownVersion?: RuntimeVersion): Promise<PhantomOrderEvent[]> {
		const api = await this.http()
		const apiAt = await api.at(blockHash, knownVersion)
		return phantomOrdersFrom(await apiAt.query.system.events())
	}

	/**
	 * Every block's phantom orders across a whole range, in one `state_queryStorage` call.
	 *
	 * This is the cheap path: the request cost of a scan stops depending on how many blocks it
	 * covers. The events key is the only key queried, and both bounds are block hashes the caller
	 * already holds.
	 *
	 * Two properties of the RPC shape the result.
	 *
	 * It returns *diffs*: `query_storage_unfiltered` in `sc-rpc` pushes a change set for a block only
	 * when the value differs from the previous block in the range (`has_changed`, and the set is
	 * dropped when empty), so a block whose events encode byte-for-byte identically to its
	 * predecessor's is simply absent. That happens on a quiet chain, where consecutive blocks carry
	 * nothing but the timestamp inherent's `ExtrinsicSuccess`. It is safe here because an absent
	 * block provably carries no phantom orders: a `PhantomOrderRegistered` commitment is derived from
	 * the block number (`phantom_order_commitment`), so a block that registered orders can never
	 * encode identically to any other block. Absent therefore means "same as the previous block",
	 * and the previous block having orders would contradict that.
	 *
	 * And it is gated by `--rpc-methods` (`check_if_safe` in `sc-rpc`), which answers a denied call
	 * with `Method not found`. The node this reads from must already run unsafe RPC to serve
	 * `offchain_localStorageGet` for the orders themselves, so this is normally available; the poll
	 * falls back to reading block by block when it is not.
	 *
	 * @returns one entry per block the node reported a change for, in ascending block order.
	 */
	async getPhantomOrdersInRange(fromBlockHash: HexString, toBlockHash: HexString): Promise<PhantomOrderEvent[][]> {
		const api = await this.http()
		// `.raw` returns the reply as the node sent it, and the key and type are named outright.
		// The formatted call instead builds a `StorageKey` from whatever it is handed and decodes
		// each value from that key's metadata, which is three ways to be silently wrong:
		// `entry.key()` is a hex string carrying no metadata, so values arrive as undecoded `Raw`;
		// the decorated `entry` carries metadata but `StorageKey` derives its bytes by *calling* it,
		// and calling a decorated entry runs the query, so the key becomes `[object Promise]`,
		// matches no change set, and the value falls back to the entry's empty default; only
		// `entry.creator` is right. None of them fail loudly. Naming both leaves nothing to pick.
		const changeSets = (await api.rpc.state.queryStorage.raw(
			[SYSTEM_EVENTS_KEY],
			fromBlockHash,
			toBlockHash,
		)) as unknown as Array<{ changes: Array<[string, string | null]> }>
		return changeSets.map((changeSet) => {
			const value = changeSet?.changes?.find(([key]) => key === SYSTEM_EVENTS_KEY)?.[1]
			// A change set carrying nothing for this key: no events, so no orders.
			if (!value) return []
			return phantomOrdersFrom(api.registry.createType("Vec<EventRecord>", value))
		})
	}

	/**
	 * Polls for newly registered phantom orders, invoking the callback once per block that carries
	 * any, with all of that block's orders.
	 *
	 * Per block rather than per order because that is how the pallet writes them: one order per
	 * configured chain, all registered in the same `on_initialize`. Delivering them together lets a
	 * caller bid on the whole interval in one extrinsic (see {@link submitPhantomBids}) instead of
	 * one per chain.
	 *
	 * Each tick reads the current head and scans every block between the last one processed and that
	 * head, so the block cursor — not the connection — determines what has been seen. This replaced a
	 * system.events subscription, which was only as reliable as its socket: polkadot-js reconnects
	 * the transport but does not reliably re-establish storage subscriptions, and anything emitted
	 * while disconnected was lost silently. With a bid window measured in a handful of blocks, that
	 * meant silently missed bids.
	 *
	 * Scanning a block range is gap-free rather than merely self-healing: an outage delays orders but
	 * cannot drop them, because the cursor only advances past a block whose events were actually
	 * read. Recovery replays the backlog.
	 *
	 * Every read here goes over HTTP, never the websocket. Polling is a sequence of independent
	 * one-shot requests with no state to lose between them, which is exactly what a stateless
	 * transport does well: a request either answers or fails loudly on this tick, instead of a
	 * socket that looks alive while delivering nothing. It also means a websocket outage does not
	 * pause phantom bidding at all — the two transports fail independently.
	 *
	 * What the cadence does *not* describe is the request rate, which is what rate limiters police.
	 * A tick costs four requests whatever the range covers — the head, the runtime version, the two
	 * bounding block hashes as one batched request, and one `state_queryStorage` for every block's
	 * events — and they go out back-to-back, so an interval well under any per-second limit could
	 * still arrive as a burst over it. Three things keep that in bounds: the provider coalesces
	 * concurrent calls into one request and paces requests through the endpoint's token bucket (see
	 * `http`), and `maxBlocksPerPoll` bounds the range. A 429 that gets through anyway backs the
	 * poll off for a doubling number of ticks, so a limiter that is already shedding load is not
	 * handed the next window's budget in rejections too.
	 *
	 * Where the node will not serve `state_queryStorage` the poll reads block by block instead, at
	 * three requests plus one per block; see {@link scanRangeAtOnce}.
	 *
	 * Returns a function that stops polling.
	 */
	pollPhantomOrders(
		callback: (events: PhantomOrderEvent[]) => void,
		options: PollPhantomOrdersOptions = {},
	): () => void {
		const {
			intervalMs,
			maxBlocksPerPoll = DEFAULT_MAX_BLOCKS_PER_POLL,
			onError,
			onSkip,
		} = options

		// Last block whose events have been delivered. Null until the first successful head read.
		let cursor: number | null = null
		let inFlight = false
		let stopped = false
		// Ticks still to sit out after a 429. Retrying an endpoint that is shedding load at the same
		// cadence that provoked it just spends the next window's budget on rejections.
		let backoffTicks = 0
		// The magnitude the countdown was last set from, kept so consecutive rejections double
		// rather than each starting over at one.
		let backoffLength = 0

		const tick = async (): Promise<void> => {
			// A scan slower than the interval must not stack up behind itself.
			if (inFlight || stopped) return
			if (backoffTicks > 0) {
				backoffTicks -= 1
				return
			}
			inFlight = true
			try {
				const api = await this.http()
				const head = (await api.rpc.chain.getHeader()).number.toNumber()

				if (cursor === null) {
					// Start just below the head, so the head itself is the first block scanned. A
					// process that restarts mid-window misses that window rather than reaching back
					// for it: the orders it would find are the ones it already had no time to bid on.
					cursor = Math.max(head - 1, -1)
				} else {
					// A backlog longer than a whole generation cycle is beyond saving: the cursor
					// gains at most `maxBlocksPerPoll` a tick, so a deficit accumulated while ticks
					// were lost (a scan overrunning the interval, a rate-limit backoff sitting them
					// out) is never repaid unless the sustained rate beats the chain's — a filler can
					// sit hours behind while looking healthy, which is what happened on mainnet.
					//
					// The cursor jumps to one window behind the head rather than to the head itself,
					// so every order that can still be bid on is kept and only the dead ones are
					// dropped. That also lands the cursor inside the threshold, so the next tick
					// resumes scanning instead of skipping again.
					const { bidWindowBlocks, intervalBlocks } = await this.phantomTimings()
					if (head - cursor > bidWindowBlocks + Math.max(intervalBlocks, bidWindowBlocks)) {
						const from = cursor + 1
						cursor = Math.max(head - 1 - bidWindowBlocks, -1)
						onSkip?.({ from, to: cursor, head })
					}
				}
				if (head <= cursor) return

				// Established after the head read and only when there is something to scan, so a
				// quiet tick still costs exactly one request.
				const knownVersion = await this.confirmedRuntimeVersion()

				const to = Math.min(head, cursor + maxBlocksPerPoll)

				// The whole range in one call, when the node will serve it.
				const ranged = await this.scanRangeAtOnce(api, cursor + 1, to, knownVersion, onError)
				if (ranged) {
					for (const orders of ranged) {
						if (stopped) return
						if (orders.length > 0) callback(orders)
					}
					// One call answered for the range or threw, so there is no partial progress to
					// preserve — the cursor moves the whole way or not at all.
					cursor = to
					backoffLength = 0
					return
				}

				const numbers: number[] = []
				for (let blockNumber = cursor + 1; blockNumber <= to; blockNumber++) numbers.push(blockNumber)

				// One concurrent wave, which the provider sends as a single batched request. Only
				// the leading run of hashes that came back is used: the events reads below advance
				// the cursor block by block, so a hash the batch could not answer simply ends this
				// tick's range and is re-read on the next, exactly as a failed events read is.
				const hashes = await Promise.allSettled(
					numbers.map((blockNumber) => api.rpc.chain.getBlockHash(blockNumber)),
				)

				for (let index = 0; index < numbers.length; index++) {
					if (stopped) return
					const hash = hashes[index]
					if (hash.status === "rejected") throw hash.reason
					const orders = await this.getPhantomOrdersAtHash(hash.value.toHex() as HexString, knownVersion)
					if (orders.length > 0) callback(orders)
					// Advance per block, not per range: a failure partway through re-scans only the
					// blocks that were never read, and never re-delivers ones that were.
					cursor = numbers[index]
				}
				backoffLength = 0
			} catch (err) {
				if (isRateLimited(err)) {
					backoffLength = Math.min(backoffLength === 0 ? 1 : backoffLength * 2, MAX_RATE_LIMIT_BACKOFF_TICKS)
					backoffTicks = backoffLength
				}
				onError?.(err)
			} finally {
				inFlight = false
			}
		}

		void tick()

		// An explicit cadence starts the timer here and now; only the runtime-derived default has to
		// wait on the node, and the scan above has already gone out either way.
		let timer: ReturnType<typeof setInterval> | null = null
		const startTimer = (ms: number) => {
			if (stopped) return
			timer = setInterval(() => void tick(), ms)
		}
		if (intervalMs !== undefined) startTimer(intervalMs)
		else void this.phantomPollIntervalMs().then(startTimer)

		return () => {
			stopped = true
			if (timer) clearInterval(timer)
		}
	}

	/**
	 * The Hyperbridge head, over HTTP like every other read here.
	 *
	 * Exposed for callers that have to know how old something is: a phantom order carries the block
	 * it was registered at, and only against the head does that become "still biddable" or "long
	 * expired".
	 */
	async latestBlockNumber(): Promise<number> {
		const api = await this.http()
		return (await api.rpc.chain.getHeader()).number.toNumber()
	}

	/**
	 * The pallet's phantom timings, read from chain state.
	 *
	 * Both are governance-settable and neither is derivable: on Nexus today the window is 15 while
	 * the runtime constant behind it is 25, so anything hard-coded is wrong in one direction or the
	 * other — too tight and live orders are dropped, too loose and bids are sent into a closed
	 * window for the pallet to reject.
	 *
	 * Read once per instance and cached, because a governance change to either is rare and a read
	 * per poll tick would be a request per tick forever. The cost is that a change is picked up on
	 * the next restart rather than immediately. A failed read is not cached, so it retries.
	 */
	async phantomTimings(): Promise<PhantomTimings> {
		if (!this.phantomTimingsRead) {
			this.phantomTimingsRead = this.readPhantomTimings().catch((err) => {
				this.phantomTimingsRead = null
				throw err
			})
		}
		return this.phantomTimingsRead
	}

	private async readPhantomTimings(): Promise<PhantomTimings> {
		const api = await this.http()
		const [window, interval] = await Promise.all([
			api.query.intentsCoprocessor.phantomBidWindow(),
			api.query.intentsCoprocessor.phantomOrderInterval(),
		])
		const stored = Number(window.toString())
		return {
			bidWindowBlocks:
				stored === 0 ? Number(api.consts.intentsCoprocessor.phantomOrderBidWindowBlocks.toString()) : stored,
			intervalBlocks: Number(interval.toString()),
		}
	}

	/**
	 * A whole range of blocks in one `state_queryStorage` call, or `null` when that is not available
	 * and the caller should read block by block.
	 *
	 * Two conditions have to hold, and both are about decoding rather than the range itself.
	 *
	 * The version must be confirmed for this tick — an upgrade inside the range means blocks decode
	 * against different metadata, and one call cannot do that.
	 *
	 * And that confirmed version must still be the one the api's own registry was built for.
	 * `state_queryStorage` declares no historic block hash, so rpc-core skips its registry swap and
	 * decodes the reply against the default registry — fixed at connect, with no
	 * `subscribeRuntimeVersion` on an HTTP api to refresh it. After an upgrade the two diverge, and
	 * the per-block path takes over for good: `api.at(hash, version)` resolves, and builds, the right
	 * registry. That costs a restart to get the cheap path back, which is the correct direction to
	 * fail in.
	 */
	private async scanRangeAtOnce(
		api: ApiPromise,
		from: number,
		to: number,
		knownVersion: RuntimeVersion | undefined,
		onError?: (err: unknown) => void,
	): Promise<PhantomOrderEvent[][] | null> {
		if (this.rangeQueryUnavailable || !knownVersion) return null
		const registryVersion = api.runtimeVersion?.specVersion
		if (!registryVersion || !knownVersion.specVersion.eq(registryVersion)) return null

		// Two calls at most, and the provider sends them as one request.
		const [fromHash, toHash] =
			from === to
				? await Promise.all([api.rpc.chain.getBlockHash(from)]).then(([only]) => [only, only])
				: await Promise.all([api.rpc.chain.getBlockHash(from), api.rpc.chain.getBlockHash(to)])

		try {
			return await this.getPhantomOrdersInRange(fromHash.toHex() as HexString, toHash.toHex() as HexString)
		} catch (err) {
			// A node that will not serve the method will not start doing so.
			if (isMethodUnavailable(err)) {
				this.rangeQueryUnavailable = true
				return null
			}
			// Neither will a reply this cannot decode start decoding. Giving up on the cheap path
			// costs requests; letting it fail every tick costs every bid, which is what happened
			// when the range read asked for a bare key and got raw bytes back. Reported once so the
			// degradation is visible, then the per-block path — which decodes differently — takes
			// over and the poll keeps working.
			if (err instanceof EventDecodeError) {
				this.rangeQueryUnavailable = true
				onError?.(err)
				return null
			}
			// Anything else is this tick's failure to report, not a reason to abandon the fast path.
			throw err
		}
	}

	/**
	 * The runtime version this tick's blocks may be decoded against, or `undefined` when that cannot
	 * be established and each block must resolve its own.
	 *
	 * Naming a version to `api.at` is what removes two of the four RPCs a block scan costs, and it
	 * is only sound while the version is actually the block's. Getting that wrong is not a loud
	 * failure: events decoded against the wrong metadata come back as a shape the scan does not
	 * recognise, so the block reads as carrying no phantom orders and the cursor advances past it —
	 * exactly the silent miss the block cursor exists to rule out.
	 *
	 * So the version is read fresh each tick and only used when it matches the previous reading.
	 * `specVersion` only ever increases, and this read happens *after* the head read, so two equal
	 * readings mean no upgrade landed anywhere in between — and therefore none in the range about to
	 * be scanned. A reading that differs means an upgrade landed inside the range: that tick falls
	 * back to per-block resolution, which is exact, and the version is used from the next tick on
	 * once it has been seen twice.
	 *
	 * The gap this leaves is a backlog reaching back past an upgrade, whose oldest blocks predate
	 * even the previous reading. Recovering from an outage that long means those bid windows closed
	 * many upgrades ago, so nothing is lost that was still winnable.
	 *
	 * A version that cannot be read at all yields `undefined` rather than an error: the scan is
	 * about to make the same request against the same endpoint and is the better place to report it.
	 */
	private async confirmedRuntimeVersion(): Promise<RuntimeVersion | undefined> {
		let api: ApiPromise
		let current: RuntimeVersion
		try {
			api = await this.http()
			current = await api.rpc.state.getRuntimeVersion()
		} catch {
			return undefined
		}

		// The api's own version was read at connect, which is a reading like any other — it lets the
		// first tick pin without spending a tick establishing what is almost always unchanged.
		const previous = this.lastRuntimeVersion ?? api.runtimeVersion
		this.lastRuntimeVersion = current
		if (!previous?.specVersion || !previous?.specName) return undefined
		return current.specVersion.eq(previous.specVersion) && current.specName.eq(previous.specName)
			? current
			: undefined
	}

	/**
	 * The poll cadence for the runtime this instance is connected to: Gargantua polls every block,
	 * everything else every 15s. Falls back to the slower cadence if the runtime cannot be read,
	 * since an unreachable node is the poll's problem to report, not the cadence lookup's.
	 */
	private async phantomPollIntervalMs(): Promise<number> {
		try {
			const specName = (await this.http()).runtimeVersion.specName.toString()
			return specName === "gargantua" ? GARGANTUA_PHANTOM_POLL_INTERVAL_MS : PHANTOM_POLL_INTERVAL_MS
		} catch {
			return PHANTOM_POLL_INTERVAL_MS
		}
	}
}
