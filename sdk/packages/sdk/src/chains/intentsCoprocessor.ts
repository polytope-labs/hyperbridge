import { ApiPromise, HttpProvider, Keyring, WsProvider } from "@polkadot/api"
import type { ApiOptions, SubmittableExtrinsic } from "@polkadot/api/types"
import type { KeyringPair } from "@polkadot/keyring/types"
import { hexToU8a, u8aToHex, u8aConcat } from "@polkadot/util"
import { decodeAddress, keccakAsU8a } from "@polkadot/util-crypto"
import { numberToBytes, bytesToBigInt, decodeAbiParameters, hexToBytes } from "viem"
import { Bytes, Struct, u8, Vector } from "scale-ts"
import PQueue from "p-queue"
import type { BidSubmissionResult, HexString, PackedUserOperation, BidStorageEntry, FillerBid, Order } from "@/types"
import type { SubstrateChain } from "./substrate"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"

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
}

export interface PollPhantomOrdersOptions {
	/** How often to check for a new head. Defaults to 6s, roughly one block. */
	intervalMs?: number
	/**
	 * Most blocks scanned in a single poll, so a long outage catches up over several ticks instead of
	 * one unbounded scan. Defaults to 500.
	 */
	maxBlocksPerPoll?: number
	/**
	 * How many blocks before the current head to start from on the first poll. Defaults to 0 (start
	 * at the head). Set this to the runtime's bid window to have a restarting process pick up orders
	 * whose window is still open.
	 */
	lookbackBlocks?: number
	/** Notified when a poll fails; polling continues regardless. */
	onError?: (err: unknown) => void
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
				provider: new HttpProvider(httpUrl),
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
	 * (or is confirmed still pooled and returned as `pending`) before the next is signed; auto-nonce
	 * via `system.accountNextIndex` counts pooled extrinsics, so a pending one is never re-used.
	 *
	 * The extrinsic is built rather than passed in because the api it is built on decides where it
	 * is signed and sent: a websocket that is down when the queue reaches this submission diverts it
	 * to {@link sendViaHttp}, which needs the call bound to the HTTP api instead.
	 */
	private async signAndSendExtrinsic(
		build: ExtrinsicBuilder,
		maxRetries: number = 3,
		timeoutMs: number = 30_000,
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
	 * A retry only happens when the previous attempt verifiably went nowhere. Once an attempt's
	 * extrinsic is known to be pooled (`pending`), re-signing the same call would race our own
	 * submission: the copy either bounces off the pool (1014, same nonce below the replacement
	 * priority bump) or — if the original lands first, freeing the nonce — executes as a duplicate
	 * and fails on-chain (e.g. `BidNotFound` for a retraction). Neither can succeed, so the pending
	 * result is returned for the caller to confirm later.
	 */
	private async sendExtrinsicWithRetries(
		extrinsic: SubmittableExtrinsic<"promise">,
		maxRetries: number,
		timeoutMs: number,
	): Promise<SubmissionOutcome> {
		const keyPair = this.getKeyPair()
		let attempt = 0

		while (attempt < maxRetries) {
			const currentTip = BASE_TIP * BigInt(2 ** attempt) // Double tip on each retry
			attempt++

			try {
				const result = await this.sendWithTimeout(extrinsic, keyPair, currentTip, timeoutMs)
				if (result.success || result.pending || result.error?.includes("Dispatch error")) {
					// Return immediately on success, an in-flight extrinsic, or dispatch errors
					// (non-recoverable)
					return result
				}
			} catch (err) {
				// Unexpected error, return immediately
				return {
					success: false,
					error: err instanceof Error ? err.message : "Unknown error",
				}
			}
		}

		return {
			success: false,
			error: `Transaction failed after ${maxRetries} attempts`,
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
	 * in flight and may well execute after the watch is abandoned — the result is then `pending`,
	 * telling the caller to confirm the outcome later instead of re-signing the same call.
	 */
	private async sendWithTimeout(
		extrinsic: SubmittableExtrinsic<"promise">,
		keyPair: KeyringPair,
		tip: bigint,
		timeoutMs: number,
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
						extrinsicHash: enteredPool ? (extrinsic.hash.toHex() as HexString) : undefined,
						error: `Transaction timed out after ${timeoutMs}ms${enteredPool ? " while in the transaction pool" : ""}`,
					})
				}
			}, timeoutMs)

			extrinsic
				.signAndSend(keyPair, { tip }, (result) => {
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
	 */
	async getPhantomOrdersInBlock(blockNumber: number): Promise<PhantomOrderEvent[]> {
		const api = await this.http()
		const blockHash = await api.rpc.chain.getBlockHash(blockNumber)
		const apiAt = await api.at(blockHash)
		const records = await apiAt.query.system.events()

		const orders: PhantomOrderEvent[] = []
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		for (const { event } of records as unknown as Array<{ event: any }>) {
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
	 * Returns a function that stops polling.
	 */
	pollPhantomOrders(
		callback: (events: PhantomOrderEvent[]) => void,
		options: PollPhantomOrdersOptions = {},
	): () => void {
		const { intervalMs = 6_000, maxBlocksPerPoll = 500, lookbackBlocks = 0, onError } = options

		// Last block whose events have been delivered. Null until the first successful head read.
		let cursor: number | null = null
		let inFlight = false
		let stopped = false

		const tick = async (): Promise<void> => {
			// A scan slower than the interval must not stack up behind itself.
			if (inFlight || stopped) return
			inFlight = true
			try {
				const head = (await (await this.http()).rpc.chain.getHeader()).number.toNumber()

				if (cursor === null) {
					// Start just below the head so the head itself is scanned, less any lookback.
					cursor = Math.max(head - 1 - lookbackBlocks, -1)
				}
				if (head <= cursor) return

				const to = Math.min(head, cursor + maxBlocksPerPoll)
				for (let blockNumber = cursor + 1; blockNumber <= to; blockNumber++) {
					if (stopped) return
					const orders = await this.getPhantomOrdersInBlock(blockNumber)
					if (orders.length > 0) callback(orders)
					// Advance per block, not per range: a failure partway through re-scans only the
					// blocks that were never read, and never re-delivers ones that were.
					cursor = blockNumber
				}
			} catch (err) {
				onError?.(err)
			} finally {
				inFlight = false
			}
		}

		void tick()
		const timer = setInterval(() => void tick(), intervalMs)

		return () => {
			stopped = true
			clearInterval(timer)
		}
	}
}
