import { ApiPromise, Keyring, WsProvider } from "@polkadot/api"
import type { SubmittableExtrinsic } from "@polkadot/api/types"
import type { KeyringPair } from "@polkadot/keyring/types"
import { hexToU8a, u8aToHex, u8aConcat } from "@polkadot/util"
import { decodeAddress, keccakAsU8a } from "@polkadot/util-crypto"
import { numberToBytes, bytesToBigInt } from "viem"
import { Bytes, Struct, u8, Vector } from "scale-ts"
import type { BidSubmissionResult, HexString, PackedUserOperation, BidStorageEntry, FillerBid } from "@/types"
import type { SubstrateChain } from "./substrate"

/** Offchain storage key prefix for bids */
const OFFCHAIN_BID_PREFIX = new TextEncoder().encode("intents::bid::")

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

/**
 * Service for interacting with Hyperbridge's pallet-intents coprocessor.
 * Handles bid submission and retrieval for the IntentGatewayV2 protocol.
 *
 * Can be created from an existing SubstrateChain instance to share the connection.
 */
export class IntentsCoprocessor {
	/** Cached result of whether the node exposes intents_* RPC methods */
	private hasIntentsRpc: boolean | null = null

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
	 * Creates a Substrate keypair from the configured private key
	 * Supports both hex seed (without 0x prefix) and mnemonic phrases
	 */
	public getKeyPair(): KeyringPair {
		if (!this.substratePrivateKey) {
			throw new Error("Substrate PrivateKey Required")
		}

		const keyring = new Keyring({ type: "sr25519" })

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
	 * Checks if the connected node exposes intents_* RPC methods.
	 * Result is cached after the first check.
	 */
	private async checkIntentsRpc(): Promise<boolean> {
		if (this.hasIntentsRpc !== null) return this.hasIntentsRpc
		try {
			const methods = await this.api.rpc.rpc.methods()
			this.hasIntentsRpc = methods.methods.some((m: any) => m.toString().startsWith("intent"))
		} catch {
			this.hasIntentsRpc = false
		}
		return this.hasIntentsRpc
	}

	/**
	 * Signs and sends an extrinsic, handling status updates and errors
	 * Implements retry logic with progressive tip increases for stuck transactions
	 */
	private async signAndSendExtrinsic(
		extrinsic: SubmittableExtrinsic<"promise">,
		maxRetries: number = 3,
		timeoutMs: number = 30_000,
	): Promise<BidSubmissionResult> {
		const keyPair = this.getKeyPair()
		let baseTip = 500_000_000_000n // 0.5 BRIDGE tip to increase priority
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

					if (result.status.isInBlock || result.status.isFinalized) {
						resolved = true
						clearTimeout(timeoutId)
						resolve({
							success: true,
							blockHash: result.status.asInBlock.toHex() as HexString,
							extrinsicHash: extrinsic.hash.toHex() as HexString,
						})
					} else if (result.dispatchError) {
						resolved = true
						clearTimeout(timeoutId)
						resolve({
							success: false,
							error: `Dispatch error: ${result.dispatchError.toString()}`,
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
		const hasRpc = await this.checkIntentsRpc()

		if (hasRpc) {
			try {
				return await this.getBidsViaRpc(commitment)
			} catch (err) {
				console.warn("intents RPC failed, falling back to storage queries:", err)
			}
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
}
