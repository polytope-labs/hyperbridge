import { ApiPromise, Keyring, WsProvider } from "@polkadot/api"
import type { SubmittableExtrinsic } from "@polkadot/api/types"
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

export interface PhantomOrderEvent {
	commitment: HexString
	chain: string
	createdAt: number
	tokenA: HexString
	tokenB: HexString
	standardAmount: bigint
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
			typesBundle: {
				spec: {
					nexus: { hasher: keccakAsU8a },
					gargantua: { hasher: keccakAsU8a },
				},
			},
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
	 * Disconnects the underlying API connection if this instance owns it.
	 * Only disconnects if created via `connect()`, not when using shared connections.
	 */
	async disconnect(): Promise<void> {
		if (this.ownsConnection) {
			await this.api.disconnect()
		}
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
	 * before the next is signed.
	 */
	private async signAndSendExtrinsic(
		extrinsic: SubmittableExtrinsic<"promise">,
		maxRetries: number = 3,
		timeoutMs: number = 30_000,
	): Promise<BidSubmissionResult> {
		const result = await this.submissionQueue.add(() => this.sendExtrinsicWithRetries(extrinsic, maxRetries, timeoutMs))
		return result ?? { success: false, error: "Submission queue returned no result" }
	}

	/**
	 * Signs and sends an extrinsic, handling status updates and errors.
	 * Implements retry logic with progressive tip increases for stuck transactions.
	 */
	private async sendExtrinsicWithRetries(
		extrinsic: SubmittableExtrinsic<"promise">,
		maxRetries: number,
		timeoutMs: number,
	): Promise<BidSubmissionResult> {
		const keyPair = this.getKeyPair()
		const baseTip = 1_000_000_000n // 0.001 BRIDGE tip to increase priority
		let attempt = 0

		while (attempt < maxRetries) {
			const currentTip = baseTip * BigInt(2 ** attempt) // Double tip on each retry
			attempt++

			try {
				const result = await this.sendWithTimeout(extrinsic, keyPair, currentTip, timeoutMs)
				if (result.success || result.error?.includes("Dispatch error")) {
					// Return immediately on success or dispatch errors (non-recoverable)
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
	 * Sends an extrinsic with a timeout
	 */
	private async sendWithTimeout(
		extrinsic: SubmittableExtrinsic<"promise">,
		keyPair: KeyringPair,
		tip: bigint,
		timeoutMs: number,
	): Promise<BidSubmissionResult> {
		return new Promise<BidSubmissionResult>((resolve) => {
			let resolved = false
			let unsubscribe: (() => void) | null = null

			// Set timeout to detect stuck transactions
			const timeoutId = setTimeout(() => {
				if (!resolved) {
					resolved = true
					if (unsubscribe) {
						unsubscribe()
					}
					resolve({
						success: false,
						error: `Transaction timed out after ${timeoutMs}ms`,
					})
				}
			}, timeoutMs)

			extrinsic
				.signAndSend(keyPair, { tip }, (result) => {
					if (resolved) return

					if (result.dispatchError && (result.status.isInBlock || result.status.isFinalized)) {
						resolved = true
						clearTimeout(timeoutId)
						let errorMsg: string
						if (result.dispatchError.isModule) {
							const decoded = this.api.registry.findMetaError(result.dispatchError.asModule)
							errorMsg = `Dispatch error: ${decoded.section}::${decoded.name}`
						} else {
							errorMsg = `Dispatch error: ${result.dispatchError.toString()}`
						}
						resolve({
							success: false,
							error: errorMsg,
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
								let errorMsg: string
								if (dispatchError?.isModule) {
									const decoded = this.api.registry.findMetaError(dispatchError.asModule)
									errorMsg = `Dispatch error: ${decoded.section}::${decoded.name}`
								} else {
									errorMsg = `Dispatch error: batch interrupted (${dispatchError?.toString()})`
								}
								resolve({ success: false, error: errorMsg })
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
						resolve({ success: false, error: err.message })
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
			const extrinsic = this.api.tx.intentsCoprocessor.placeBid(commitment, userOp)
			return await this.signAndSendExtrinsic(extrinsic)
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
			const extrinsic = this.api.tx.intentsCoprocessor.retractBid(commitment)
			return await this.signAndSendExtrinsic(extrinsic)
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
			const batch = this.api.tx.utility.batch([
				this.api.tx.intentsCoprocessor.placeBid(bidCommitment, userOp),
				this.api.tx.intentsCoprocessor.retractBid(retractCommitment),
			])
			return await this.signAndSendExtrinsic(batch)
		} catch (error) {
			return {
				success: false,
				error: error instanceof Error ? error.message : "Unknown error",
			}
		}
	}

	/**
	 * Fetches all bid storage entries for a given order commitment.
	 * Returns the on-chain data only (filler addresses and deposits).
	 *
	 * @param commitment - The order commitment hash (bytes32)
	 * @returns Array of BidStorageEntry objects
	 */
	async getBidStorageEntries(commitment: HexString): Promise<BidStorageEntry[]> {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const entries = await (this.api.query.intentsCoprocessor.bids as any).entries(commitment)

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
		const result: RpcBidInfo[] = await (this.api as any)._rpcCore.provider.send("intents_getBidsForOrder", [
			commitment,
		])

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
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const entries = await (this.api.query.intentsCoprocessor.bids as any).entries(commitment)

		if (entries.length === 0) return []

		const bidPromises = entries.map(async ([storageKey, depositValue]: [any, any]) => {
			try {
				const filler = storageKey.args[1].toString()
				const deposit = BigInt(depositValue.toString())

				const offchainKey = this.buildOffchainBidKey(commitment, filler)
				const offchainKeyHex = u8aToHex(offchainKey)

				const offchainResult = await this.api.rpc.offchain.localStorageGet("PERSISTENT", offchainKeyHex)

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
		const result = await this.api.rpc.offchain.localStorageGet("PERSISTENT", u8aToHex(key))
		if (!result || result.isNone) return null

		const rawHex = result.unwrap().toHex() as HexString
		if (rawHex === "0x" || rawHex === "0x00") return null

		const placeOrderAbi = (IntentGatewayV2.ABI as readonly { type: string; name?: string; inputs?: unknown[] }[]).find(
			(item) => item.type === "function" && item.name === "placeOrder",
		)
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
	 * Subscribes to PhantomOrderRegistered events from the intents coprocessor pallet.
	 * Calls the callback for each new phantom order as blocks arrive.
	 * Returns an unsubscribe function to stop the subscription.
	 */
	async subscribePhantomOrders(callback: (event: PhantomOrderEvent) => void): Promise<() => void> {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		const unsub = await (this.api.query.system.events as any)((records: any[]) => {
			for (const { event } of records) {
				if (event.section !== "intentsCoprocessor" || event.method !== "PhantomOrderRegistered") continue
				const [commitment, chain, createdAt, tokenA, tokenB, standardAmount] = event.data
				callback({
					commitment: commitment.toHex() as HexString,
					chain: new TextDecoder().decode(hexToU8a(chain.toHex())),
					createdAt: createdAt.toNumber(),
					tokenA: tokenA.toHex() as HexString,
					tokenB: tokenB.toHex() as HexString,
					standardAmount: BigInt(standardAmount.toString()),
				})
			}
		})
		return unsub as () => void
	}
}
