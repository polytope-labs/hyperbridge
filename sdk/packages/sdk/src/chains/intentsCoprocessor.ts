import { ApiPromise, Keyring, WsProvider } from "@polkadot/api"
import type { ApiOptions, SubmittableExtrinsic } from "@polkadot/api/types"
import type { KeyringPair } from "@polkadot/keyring/types"
import { hexToU8a, u8aToHex, u8aConcat } from "@polkadot/util"
import { decodeAddress, keccakAsU8a } from "@polkadot/util-crypto"
import { numberToBytes, bytesToBigInt } from "viem"
import { Bytes, Struct, u8, Vector } from "scale-ts"
import PQueue from "p-queue"
import type {
	BidSubmissionResult,
	HexString,
	PackedUserOperation,
	BidStorageEntry,
	FillerBid,
	RpcBidInfo,
} from "@/types"
import type { SubstrateChain } from "./substrate"
import { TokenBucket } from "@/utils/rateLimiter"
import { BatchingHttpProvider } from "@/utils/batchingHttpProvider"

/** Offchain storage key prefix for bids */
const OFFCHAIN_BID_PREFIX = new TextEncoder().encode("intents::bid::")

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

/**
 * Sustained request rate allowed against a Hyperbridge HTTP endpoint, in requests per second.
 *
 * Public endpoints police the instantaneous rate — the observed limit is 10/s — and reads such as
 * the per-bid offchain lookups arrive in bursts. Sitting under the limit rather than at it leaves
 * room for the traffic this process does not pace: the websocket, and any other instance sharing
 * the address.
 *
 * `HYPERBRIDGE_RPC_MAX_RPS` overrides it for a private endpoint with a different budget.
 */
const DEFAULT_RPC_MAX_RPS = 8

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
 * websocket: bids are read out of that node's offchain worker storage, which is node-local and
 * not replicated, so a separately configured host would return nothing for bids its storage said
 * exist.
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

/** A submission result carrying the retry loop's internal state. */
interface SubmissionOutcome extends BidSubmissionResult {
	/**
	 * The extrinsic reached the pool under this account's nonce and was still there when the watch
	 * timed out — the one flavour of `pending` a replacement can act on, as opposed to a rejection
	 * that bounced off a copy already pooled. Internal to the retry loop; callers see `pending`.
	 */
	stalled?: boolean
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

	/** The HTTP-backed api, connected on first use. Cleared after a failed attempt so it retries. */
	private httpApi: Promise<ApiPromise> | null = null

	// Serialises every extrinsic submission on this instance's substrate account. All submit/retract
	// methods funnel through signAndSendExtrinsic, each using the API's auto-nonce; fired in parallel
	// (bids for orders on different chains) they would grab
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
	 * endpoint that accepts the request and then goes quiet, so a caller awaiting it never hangs
	 * silently. A failed attempt is not cached, so the next call tries again.
	 */
	private async http(): Promise<ApiPromise> {
		if (!this.httpApi) {
			const httpUrl = deriveHttpUrl(this.wsEndpoint())
			const api = new ApiPromise({
				// Response cache off (third argument, capacity 0). polkadot-js caches every request that
				// names a block hash — `chain_getHeader(hash)`, `state_getRuntimeVersion(hash)`, storage
				// reads at a hash — by storing the request promise itself, a rejected one included, for a
				// 30s TTL that every hit refreshes. A caller retrying a failed read with identical
				// parameters would get the same rejection replayed from memory for as long as it kept
				// retrying inside the TTL, and the node would never see a second request.
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
						resolve({
							success: true,
							blockHash: (result.status.isInBlock
								? result.status.asInBlock
								: result.status.asFinalized
							).toHex() as HexString,
							extrinsicHash: extrinsic.hash.toHex() as HexString,
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
	 * A filler can hold several bids on one order, one per price it offers, and `bid` tells them
	 * apart: submitting again under an identifier the filler already holds replaces that bid alone.
	 * By convention it is `keccak256` of the UserOp's `callData`, which is also what gives each bid
	 * its own nonce key on the `SolverAccount` (see `CryptoUtils.bidId`).
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @param userOp - The encoded PackedUserOperation as hex string
	 * @param bid - Identifies this bid among the filler's bids on the order (bytes32)
	 * @returns BidSubmissionResult with success status and block/extrinsic hash
	 */
	async submitBid(commitment: HexString, userOp: HexString, bid: HexString): Promise<BidSubmissionResult> {
		try {
			return await this.signAndSendExtrinsic((api) => api.tx.intentsCoprocessor.placeBid(commitment, bid, userOp))
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
	 * @param bid - Which of the filler's bids on the order to retract (bytes32)
	 * @returns BidSubmissionResult with success status and block/extrinsic hash
	 */
	async retractBid(commitment: HexString, bid: HexString): Promise<BidSubmissionResult> {
		try {
			return await this.signAndSendExtrinsic((api) => api.tx.intentsCoprocessor.retractBid(commitment, bid))
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
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
		const entries = await (api.query.intentsCoprocessor.orderBids as any).entries(commitment)

		return entries.map(([storageKey, depositValue]: [any, any]) => ({
			commitment,
			filler: storageKey.args[1].toString() as string,
			bid: storageKey.args[2].toHex() as HexString,
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
			return { filler, bid: entry.bid, userOp, deposit: 0n }
		})
	}

	/**
	 * Fetches bids using on-chain storage entries + parallel offchain lookups.
	 * Slower but works on all nodes and includes deposit amounts.
	 */
	private async getBidsViaStorage(commitment: HexString): Promise<FillerBid[]> {
		const api = await this.http()
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const entries = await (api.query.intentsCoprocessor.orderBids as any).entries(commitment)

		if (entries.length === 0) return []

		const bidPromises = entries.map(async ([storageKey, depositValue]: [any, any]) => {
			try {
				const filler = storageKey.args[1].toString()
				const bid = storageKey.args[2].toHex() as HexString
				const deposit = BigInt(depositValue.toString())

				const offchainKey = this.buildOffchainBidKey(commitment, filler, bid)
				const offchainKeyHex = u8aToHex(offchainKey)

				const offchainResult = await api.rpc.offchain.localStorageGet("PERSISTENT", offchainKeyHex)

				if (!offchainResult || offchainResult.isNone) return null

				const bidData = offchainResult.unwrap().toHex() as HexString
				const decoded = this.decodeBid(bidData)

				return { filler: decoded.filler, bid, userOp: decoded.userOp, deposit }
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

	/** Builds offchain storage key: "intents::bid::" + commitment + filler + bid */
	private buildOffchainBidKey(commitment: HexString, filler: string, bid: HexString): Uint8Array {
		return u8aConcat(OFFCHAIN_BID_PREFIX, hexToU8a(commitment), decodeAddress(filler), hexToU8a(bid))
	}
}
