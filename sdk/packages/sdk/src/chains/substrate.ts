import { ApiPromise, WsProvider } from "@polkadot/api"
import { capitalize } from "lodash-es"
import { Vector, u8 } from "scale-ts"
import { match } from "ts-pattern"
import { bytesToHex, hexToBytes, toBytes, toHex } from "viem"

import type { IChain, IIsmpMessage, IProof } from "@/chain"
import type {
	HexString,
	IGetRequest,
	IMessage,
	IPostRequest,
	ISubstrateConfig,
	StateMachineHeight,
	StateMachineIdParams,
} from "@/types"
import {
	BasicProof,
	GetRequestsWithProof,
	type IStateMachine,
	Message,
	SubstrateStateMachineProof,
	isEvmChain,
	isSubstrateChain,
	replaceWebsocketWithHttp,
	parseStateMachineId,
} from "@/utils"
import {
	ExpectedError,
	MissingConsensusUpdateTimeError,
	MISSING_CONSENSUS_UPDATE_TIME_MESSAGE,
} from "@/utils/exceptions"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { ISMP_PREFIX } from "@/configs/constants"

/**
 * HTTP RPC Client for making JSON-RPC calls over HTTP
 */
class HttpRpcClient {
	constructor(private readonly url: string) {}

	/**
	 * Make an RPC call over HTTP
	 * @param method - The RPC method name
	 * @param params - The parameters for the RPC call
	 * @returns Promise resolving to the RPC response
	 */
	async call(method: string, params: any[] = []): Promise<any> {
		const body = JSON.stringify({
			jsonrpc: "2.0",
			id: Date.now(),
			method,
			params,
		})

		const response = await fetch(this.url, {
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body,
		})

		if (!response.ok) {
			throw new Error(`HTTP error! status: ${response.status}`)
		}

		const result = await response.json()

		if (result.error) {
			throw new Error(`RPC error: ${result.error.message}`)
		}

		return result.result
	}
}

export class SubstrateChain implements IChain {
	/*
	 * api: The Polkadot API instance for the Substrate chain.
	 */
	api?: ApiPromise
	private rpcClient: HttpRpcClient

	private constructor(private readonly params: ISubstrateConfig) {
		const url = this.params.wsUrl

		const httpUrl = replaceWebsocketWithHttp(url)
		this.rpcClient = new HttpRpcClient(httpUrl)
	}

	/**
	 * Creates a `SubstrateChain` instance and immediately connects to the
	 * WebSocket endpoint, returning a fully initialised chain client.
	 *
	 * This is the only public way to construct a `SubstrateChain` — the constructor is private.
	 *
	 * @param params - Substrate chain configuration (wsUrl, stateMachineId, etc.).
	 * @returns A connected `SubstrateChain` instance.
	 */
	static async connect(params: ISubstrateConfig): Promise<SubstrateChain> {
		const instance = new SubstrateChain(params)
		await instance.connect()
		return instance
	}

	get config(): ISubstrateConfig {
		return {
			wsUrl: this.params.wsUrl,
			consensusStateId: this.params.consensusStateId,
			hasher: this.params.hasher,
			stateMachineId: this.params.stateMachineId,
		}
	}

	/**
	 * Connects to the Substrate chain using the provided WebSocket URL.
	 */
	public async connect() {
		const wsProvider = new WsProvider(this.params.wsUrl)

		const typesBundle =
			this.params.hasher === "Keccak"
				? {
						spec: {
							nexus: { hasher: keccakAsU8a },
							gargantua: { hasher: keccakAsU8a },
						},
					}
				: {}
		this.api = await ApiPromise.create({
			provider: wsProvider,
			typesBundle,
		})
	}

	/**
	 * Disconnects the Substrate chain connection.
	 */
	public async disconnect() {
		if (this.api) {
			await this.api.disconnect()
			this.api = undefined
		}
	}

	/**
	 * Returns the storage key for a request receipt in the child trie
	 * The request commitment is the key
	 * @param key - The H256 hash key (as a 0x-prefixed hex string)
	 * @returns The storage key as a hex string
	 */
	requestReceiptKey(key: HexString): HexString {
		const prefix = new TextEncoder().encode("RequestReceipts")

		const keyBytes = hexToBytes(key)

		// Concatenate the prefix and key bytes
		return bytesToHex(new Uint8Array([...prefix, ...keyBytes]))
	}

	/**
	 * Returns the storage key for a request commitment in the child trie
	 * The request commitment is the key
	 * @param key - The H256 hash key (as a 0x-prefixed hex string)
	 * @returns The storage key as a hex string
	 */
	requestCommitmentKey(key: HexString): HexString {
		const prefix = new TextEncoder().encode("RequestCommitments")

		const keyBytes = hexToBytes(key)

		// Concatenate the prefix and key bytes
		return bytesToHex(new Uint8Array([...prefix, ...keyBytes]))
	}

	/**
	 * Queries a request commitment from the ISMP child trie storage.
	 * @param {HexString} commitment - The commitment hash to look up.
	 * @returns {Promise<HexString | undefined>} The commitment data if found, undefined otherwise.
	 */
	async queryRequestCommitment(commitment: HexString): Promise<HexString | undefined> {
		const prefix = toHex(ISMP_PREFIX)
		const key = this.requestCommitmentKey(commitment)

		const item: any = await this.rpcClient.call("childstate_getStorage", [prefix, key])

		return item
	}

	/**
	 * Queries the request receipt.
	 * @param {HexString} commitment - The commitment to query.
	 * @returns {Promise<HexString | undefined>} The relayer address responsible for delivering the request.
	 */
	async queryRequestReceipt(commitment: HexString): Promise<HexString | undefined> {
		const prefix = toHex(ISMP_PREFIX)
		const key = this.requestReceiptKey(commitment)

		const item: any = await this.rpcClient.call("childstate_getStorage", [prefix, key])

		return item
	}

	/**
	 * Returns the storage key for a response receipt in the child trie.
	 * A response receipt is keyed by the originating *request* commitment.
	 * @param {HexString} key - The request commitment (0x-prefixed H256 hex string)
	 * @returns {HexString} The storage key as a hex string
	 */
	responseReceiptKey(key: HexString): HexString {
		const prefix = new TextEncoder().encode("ResponseReceipts")

		const keyBytes = hexToBytes(key)

		// Concatenate the prefix and key bytes
		return bytesToHex(new Uint8Array([...prefix, ...keyBytes]))
	}

	/**
	 * Queries the response receipt for a request commitment. For a GET, Hyperbridge
	 * produces the response as it processes the request, so the presence of a response
	 * receipt indicates the request has already been delivered and handled.
	 * @param {HexString} commitment - The originating request commitment to query.
	 * @returns {Promise<HexString | undefined>} The receipt data if present, otherwise undefined.
	 */
	async queryResponseReceipt(commitment: HexString): Promise<HexString | undefined> {
		const prefix = toHex(ISMP_PREFIX)
		const key = this.responseReceiptKey(commitment)

		const item: any = await this.rpcClient.call("childstate_getStorage", [prefix, key])

		return item
	}

	/**
	 * Queries the state-machine commitment Hyperbridge holds for a counterparty chain at an
	 * exact height — the `BoundedStateCommitments` map that `state_machine_commitment` (and thus
	 * proof verification) reads. Returns the committed `stateRoot`, or undefined if Hyperbridge
	 * has not finalized that chain at exactly that height.
	 * @param {StateMachineHeight} height - The counterparty state machine id + height.
	 * @returns {Promise<HexString | undefined>} The committed state root, or undefined if absent.
	 */
	async queryStateMachineCommitment(height: StateMachineHeight): Promise<HexString | undefined> {
		if (!this.api) throw new Error("API not initialized")
		const id = {
			stateId: height.id.stateId,
			// on-chain StateMachineId encodes consensusStateId as [u8; 4]
			consensusStateId: toHex(toBytes(height.id.consensusStateId)),
		}
		const commitment = await this.api.query.ismp.boundedStateCommitments(id, Number(height.height))
		if ((commitment as any).isNone) return undefined
		return (commitment.toJSON() as { stateRoot?: HexString })?.stateRoot
	}

	/**
	 * Returns the current timestamp of the chain.
	 * @returns {Promise<bigint>} The current timestamp.
	 */
	async timestamp(): Promise<bigint> {
		if (!this.api) throw new Error("API not initialized")

		const now = await this.api.query.timestamp.now()

		return BigInt(now.toJSON() as number) / BigInt(1000)
	}

	/**
	 * Queries the proof of the commitments.
	 * @param {IMessage} message - The message to query.
	 * @param {string} counterparty - The counterparty address.
	 * @param {bigint} [at] - The block number to query at.
	 * @returns {Promise<HexString>} The proof.
	 */
	async queryProof(message: IMessage, counterparty: string, at?: bigint): Promise<HexString> {
		if (isEvmChain(counterparty)) {
			// for evm chains, query the mmr proof
			const proof: any = await this.rpcClient.call("mmr_queryProof", [Number(at), message])
			return toHex(proof.proof)
		}

		if (isSubstrateChain(counterparty)) {
			// for substrate chains, we use the child trie proof
			const childTrieKeys =
				"Requests" in message
					? message.Requests.map(requestCommitmentStorageKey)
					: message.Responses.map(responseCommitmentStorageKey)
			const proof: any = await this.rpcClient.call("ismp_queryChildTrieProof", [Number(at), childTrieKeys])
			const basicProof = BasicProof.dec(toHex(proof.proof))
			const encoded = SubstrateStateMachineProof.enc({
				hasher: {
					tag: this.params.hasher,
					value: undefined,
				},
				storageProof: basicProof,
			})
			return toHex(encoded)
		}

		throw new ExpectedError(`Unsupported chain type for counterparty: ${counterparty}`)
	}

	/**
	 * Submit an unsigned ISMP transaction to the chain. Resolves when the transaction is finalized.
	 * @param message - The message to be submitted.
	 * @returns A promise that resolves to an object containing the transaction hash, block hash, and block number.
	 */
	async submitUnsigned(message: IIsmpMessage): Promise<{
		transactionHash: string
		blockHash: string
		blockNumber: number
		timestamp: number
	}> {
		if (!this.api) throw new Error("API not initialized")
		const { api } = this

		const args = encodeISMPMessage(message)
		let tx
		if (message.kind === "GetRequest") {
			tx = api.tx.stateCoprocessor.handleUnsigned(args)
		} else {
			tx = api.tx.ismp.handleUnsigned(args)
		}

		return new Promise((resolve, reject) => {
			let unsub = () => {}

			tx.send(async ({ isInBlock, isFinalized, isError, dispatchError, txHash, status }) => {
				if (isFinalized || isInBlock) {
					unsub()
					const blockHash = isInBlock ? status.asInBlock.toHex() : status.asFinalized.toHex()
					const header = await api.rpc.chain.getHeader(blockHash)
					// Get a decorated api instance at a specific block
					const apiAt = await api.at(blockHash)
					const timestamp = await apiAt.query.timestamp.now()
					resolve({
						transactionHash: txHash.toHex(),
						blockHash: blockHash,
						blockNumber: header.number.toNumber(),
						timestamp: Number(timestamp.toJSON()) / 1000,
					})
				} else if (isError) {
					unsub()
					console.error("Unsigned transaction failed: ", dispatchError)
					reject(dispatchError)
				}
			})
				.then((unsubscribe) => {
					unsub = unsubscribe
				})
				.catch(reject)
		})
	}

	/**
	 * Query the state proof for a given set of keys at a specific block height.
	 * @param at The block height to query the state proof at.
	 * @param keys The keys to query the state proof for.
	 * @param _address - Optional address (ignored for Substrate; present for IChain compatibility).
	 * @returns The state proof as a hexadecimal string.
	 */
	async queryStateProof(at: bigint, keys: HexString[], _address?: HexString): Promise<HexString> {
		const encodedKeys = keys.map((key) => Array.from(hexToBytes(key)))
		const proof: any = await this.rpcClient.call("ismp_queryChildTrieProof", [Number(at), encodedKeys])
		const basicProof = BasicProof.dec(toHex(proof.proof))
		const encoded = SubstrateStateMachineProof.enc({
			hasher: {
				tag: this.params.hasher,
				value: undefined,
			},
			storageProof: basicProof,
		})
		return toHex(encoded)
	}

	/**
	 * Get the latest state machine height for a given state machine ID.
	 * @param {StateMachineIdParams} stateMachineId - The state machine ID.
	 * @returns {Promise<bigint>} The latest state machine height.
	 */
	async latestStateMachineHeight(stateMachineId: StateMachineIdParams): Promise<bigint> {
		const state_id = convertStateIdToStateMachineId(stateMachineId.stateId)

		const payload = {
			state_id,
			consensus_state_id: stateMachineId.consensusStateId,
		}

		const latestHeight: number = await this.rpcClient.call("ismp_queryStateMachineLatestHeight", [payload])
		return BigInt(latestHeight)
	}

	/**
	 * Get the state machine update time for a given state machine height. Reads the
	 * `BoundedStateMachineUpdateTime` map in pallet-ismp directly, so the height is either
	 * still retained on-chain or it has been evicted — there's no intermediate RPC to
	 * reinterpret the absence.
	 * @param {StateMachineHeight} stateMachineHeight - The state machine height.
	 * @returns {Promise<bigint>} The statemachine update time in seconds.
	 */
	async stateMachineUpdateTime(stateMachineHeight: StateMachineHeight): Promise<bigint> {
		if (!this.api) throw new Error("API not initialized")

		const id = {
			stateId: stateMachineHeight.id.stateId,
			// on-chain StateMachineId encodes consensusStateId as [u8; 4]
			consensusStateId: toHex(toBytes(stateMachineHeight.id.consensusStateId)),
		}

		const updateTime = await this.api.query.ismp.boundedStateMachineUpdateTime(
			id,
			Number(stateMachineHeight.height),
		)

		if ((updateTime as any).isNone) {
			throw new MissingConsensusUpdateTimeError(MISSING_CONSENSUS_UPDATE_TIME_MESSAGE)
		}

		return BigInt((updateTime as any).unwrap().toString())
	}

	/**
	 * Get the challenge period for a given state machine id.
	 * @param {StateMachineIdParams} stateMachineId - The state machine ID.
	 * @returns {Promise<bigint>} The challenge period in seconds.
	 */
	async challengePeriod(stateMachineId: StateMachineIdParams): Promise<bigint> {
		const state_id = convertStateIdToStateMachineId(stateMachineId.stateId)

		const payload = {
			state_id,
			consensus_state_id: stateMachineId.consensusStateId,
		}

		const challengePeriod: number = await this.rpcClient.call("ismp_queryChallengePeriod", [payload])
		return BigInt(challengePeriod)
	}

	/**
	 * Encode an ISMP calldata for a substrate chain.
	 * @param message The ISMP message to encode.
	 * @returns The encoded message as a hexadecimal string.
	 */
	encode(message: IIsmpMessage): HexString {
		const palletIndex = this.getPalletIndex("Ismp")
		const args = encodeISMPMessage(message)

		// Encoding the call enum and call index
		const call = Vector(u8, 2).enc([palletIndex, 0])

		return toHex(new Uint8Array([...call, ...args]))
	}

	/**
	 * Queries the latest proven height for this chain from pallet-ismp.
	 */
	async queryLastProvenHeight(): Promise<bigint> {
		return this.latestStateMachineHeight({
			stateId: parseStateMachineId(this.params.stateMachineId).stateId,
			consensusStateId: this.params.consensusStateId,
		})
	}

	/**
	 * Queries all consensus proofs needed to catch up from a given on-chain epoch
	 * to the latest proven state. Returns rotation proofs (in order) to advance
	 * the authority set, plus the final messaging proof that covers `neededHeight`.
	 *
	 * @param neededHeight - The Hyperbridge block height that must be covered
	 * @param currentEpoch - The current authority set id on the destination EVM chain
	 */
	async queryConsensusProofs(
		neededHeight: bigint,
		currentEpoch: bigint,
	): Promise<{ proofs: HexString[]; provenHeight: bigint } | undefined> {
		const lastProvenHeight = await this.queryLastProvenHeight()
		if (lastProvenHeight < neededHeight) return undefined

		if (!this.api) throw new Error("API not initialized")

		// Read RotationProofs BTreeMap<u64, u64> from storage
		const rotationProofsRaw = await (this.api.query as any).beefyConsensusProofs.rotationProofs()
		const rotationMap: Map<bigint, bigint> = new Map()
		for (const [setId, height] of rotationProofsRaw.entries()) {
			rotationMap.set(BigInt(setId.toString()), BigInt(height.toString()))
		}

		const proofs: HexString[] = []

		// The map is keyed by the set each proof rotated *to*, and HandlerV2 records
		// `nextAuthoritySetId - 1`, so `currentEpoch` is the destination's current set. The
		// first proof it still needs is therefore the one rotating to currentEpoch + 1. This
		// matches `rotation_proofs_from`, which walks from `from_set_id + 1`.
		let epoch = currentEpoch + 1n
		while (rotationMap.has(epoch)) {
			const key = rotationOffchainKey(epoch)
			let proof = await this.rpcClient.call("offchain_localStorageGet", ["PERSISTENT", key])
			if (!proof) {
				// Rotations recorded before the namespace split are still under the messaging
				// key at their height. Drop this once every destination is past the upgrade.
				const legacyKey = messagingOffchainKey(rotationMap.get(epoch)!)
				proof = await this.rpcClient.call("offchain_localStorageGet", ["PERSISTENT", legacyKey])
			}
			if (!proof) return undefined
			proofs.push(proof as HexString)
			epoch++
		}

		// Fetch the final messaging proof at lastProvenHeight. A rotation advances the proven
		// height without writing a messaging blob, so when nothing is there, resolve the
		// height through the rotation index instead.
		const messagingKey = messagingOffchainKey(lastProvenHeight)
		let messagingProof = await this.rpcClient.call("offchain_localStorageGet", ["PERSISTENT", messagingKey])
		if (!messagingProof) {
			for (const [setId, height] of rotationMap) {
				if (height !== lastProvenHeight) continue
				messagingProof = await this.rpcClient.call("offchain_localStorageGet", [
					"PERSISTENT",
					rotationOffchainKey(setId),
				])
				break
			}
		}
		if (!messagingProof) return undefined
		proofs.push(messagingProof as HexString)

		return { proofs, provenHeight: lastProvenHeight }
	}

	private getPalletIndex(name: string): number {
		if (!this.api) throw new Error("API not initialized")
		const pallets = this.api.runtimeMetadata.asLatest.pallets.entries()

		for (const p of pallets) {
			if (p[1].name.toString() === name) {
				const index = p[1].index.toNumber()

				return index
			}
		}

		throw new Error(`${name} not found in runtime`)
	}
}

function beefyOffchainKey(prefix: string, id: bigint): HexString {
	const prefixBytes = new TextEncoder().encode(prefix)
	const idBytes = new Uint8Array(8)
	new DataView(idBytes.buffer).setBigUint64(0, id, false)
	return toHex(new Uint8Array([...prefixBytes, ...idBytes]))
}

/**
 * Offchain key for a messaging proof, keyed by the parachain height it advanced to.
 * Mirrors `messaging_offchain_key` in pallet-beefy-consensus-proofs.
 */
function messagingOffchainKey(provenHeight: bigint): HexString {
	return beefyOffchainKey("beefy_consensus_proofs::", provenHeight)
}

/**
 * Offchain key for a mandatory proof, keyed by the authority set id it rotated to.
 * Mirrors `rotation_offchain_key` in pallet-beefy-consensus-proofs. Mandatory proofs sit in
 * their own namespace because a rotation can finalize a head a messaging proof already
 * covered, so height alone would let one overwrite the other.
 */
function rotationOffchainKey(setId: bigint): HexString {
	return beefyOffchainKey("beefy_consensus_proofs::rotation::", setId)
}

function requestCommitmentStorageKey(key: HexString): number[] {
	// Convert "RequestCommitments" to bytes
	const prefix = new TextEncoder().encode("RequestCommitments")

	// Convert hex key to bytes
	const keyBytes = hexToBytes(key)

	// Combine prefix and key bytes
	return Array.from(new Uint8Array([...prefix, ...keyBytes]))
}

function responseCommitmentStorageKey(key: HexString): number[] {
	// Convert "ResponseCommitments" to bytes
	const prefix = new TextEncoder().encode("ResponseCommitments")

	// Convert hex key to bytes
	const keyBytes = hexToBytes(key)

	// Combine prefix and key bytes
	return Array.from(new Uint8Array([...prefix, ...keyBytes]))
}

/**
 * Converts a state machine ID string to an enum value.
 * @param {string} id - The state machine ID string.
 * @returns {IStateMachine} The corresponding enum value.
 */
export function convertStateMachineIdToEnum(id: string): IStateMachine {
	let [tag, value]: any = id.split("-")
	tag = capitalize(tag)
	if (["Evm", "Polkadot", "Kusama"].includes(tag)) {
		value = Number.parseInt(value)
	} else {
		value = Array.from(toBytes(value))
	}

	return { tag, value }
}

/**
 * Converts a state machine enum representation to a string.
 * @param {IStateMachine} stateMachine - The state machine enum object.
 * @returns {string} The state machine ID string like "EVM-97" or "SUBSTRATE-cere".
 */
export function convertStateMachineEnumToString(stateMachine: { tag: string; value: number | number[] }): string {
	const tag = stateMachine.tag.toUpperCase()
	if (tag === "EVM" || tag === "POLKADOT" || tag === "KUSAMA") {
		return `${tag}-${stateMachine.value}`
	} else {
		const bytes = new Uint8Array(stateMachine.value as number[])
		const decoder = new TextDecoder("utf-8")
		const decoded = decoder.decode(bytes)
		return `${tag}-${decoded}`
	}
}

/**
 * Converts a stateId object back to the state_id format used by the RPC.
 * @param stateId - The stateId object from StateMachineIdParams
 * @returns The string representation like "EVM-11155111" or "SUBSTRATE-cere"
 */
export function convertStateIdToStateMachineId(stateId: {
	Evm?: number
	Substrate?: HexString
	Polkadot?: number
	Kusama?: number
}): string {
	switch (true) {
		case stateId.Evm !== undefined:
			return `EVM-${stateId.Evm}`
		case stateId.Polkadot !== undefined:
			return `POLKADOT-${stateId.Polkadot}`
		case stateId.Kusama !== undefined:
			return `KUSAMA-${stateId.Kusama}`
		case stateId.Substrate !== undefined: {
			const bytes = hexToBytes(stateId.Substrate as HexString)
			const decoder = new TextDecoder("utf-8")
			const decoded = decoder.decode(bytes)
			return `SUBSTRATE-${decoded}`
		}
		default:
			throw new Error("Unsupported stateId variant")
	}
}

/**
 * Converts an array of IPostRequest objects to a codec representation.
 * @param {IPostRequest} request - The array of IPostRequest objects.
 * @returns The codec representation of the requests.
 */
function convertIPostRequestToCodec(request: IPostRequest) {
	return {
		tag: "Post",
		value: {
			source: convertStateMachineIdToEnum(request.source),
			dest: convertStateMachineIdToEnum(request.dest),
			from: Array.from(hexToBytes(request.from)),
			to: Array.from(hexToBytes(request.to)),
			nonce: request.nonce,
			body: Array.from(hexToBytes(request.body)),
			timeoutTimestamp: request.timeoutTimestamp,
		},
	} as const
}

/**
 * Converts an IGetRequest object to a codec representation.
 * @param {IGetRequest} request - The IGetRequest object.
 * @returns The codec representation of the request.
 */
export function convertIGetRequestToCodec(request: IGetRequest) {
	return {
		source: convertStateMachineIdToEnum(request.source),
		dest: convertStateMachineIdToEnum(request.dest),
		from: Array.from(hexToBytes(request.from)),
		nonce: request.nonce,
		keys: request.keys.map((key) => Array.from(hexToBytes(key))),
		context: Array.from(hexToBytes(request.context)),
		timeoutTimestamp: request.timeoutTimestamp,
		height: request.height,
	} as const
}

/**
 * Convert codec representation back to IGetRequest
 */
export function convertCodecToIGetRequest(codec: {
	source: { tag: string; value: number | number[] }
	dest: { tag: string; value: number | number[] }
	from: number[]
	nonce: bigint
	keys: number[][]
	height: bigint
	context: number[]
	timeoutTimestamp: bigint
}): IGetRequest {
	return {
		source: convertStateMachineEnumToString(codec.source),
		dest: convertStateMachineEnumToString(codec.dest),
		from: bytesToHex(new Uint8Array(codec.from)) as HexString,
		nonce: codec.nonce,
		keys: codec.keys.map((key) => bytesToHex(new Uint8Array(key)) as HexString),
		height: codec.height,
		context: bytesToHex(new Uint8Array(codec.context)) as HexString,
		timeoutTimestamp: codec.timeoutTimestamp,
	}
}

/**
 * Converts an IProof object to a codec representation.
 * @param {IProof} proof - The IProof object.
 * @returns The codec representation of the proof.
 */
export function convertIProofToCodec(proof: IProof) {
	return {
		height: {
			height: proof.height,
			id: {
				consensusStateId: Array.from(toBytes(proof.consensusStateId)),
				id: convertStateMachineIdToEnum(proof.stateMachine),
			},
		},
		proof: Array.from(hexToBytes(proof.proof)),
	} as const
}

/**
 * Converts a codec representation back to an IProof object.
 * @param {any} codec - The codec representation of the proof.
 * @returns {IProof} The IProof object.
 */
export function convertCodecToIProof(codec: {
	height: {
		height: bigint
		id: {
			consensusStateId: number[]
			id: { tag: string; value: number | number[] }
		}
	}
	proof: number[]
}): IProof {
	const consensusStateIdBytes = new Uint8Array(codec.height.id.consensusStateId)
	const decoder = new TextDecoder("utf-8")
	const consensusStateId = decoder.decode(consensusStateIdBytes)

	return {
		height: codec.height.height,
		stateMachine: convertStateMachineEnumToString(codec.height.id.id),
		consensusStateId,
		proof: bytesToHex(new Uint8Array(codec.proof)) as HexString,
	}
}

export function encodeISMPMessage(message: IIsmpMessage): Uint8Array {
	try {
		return match(message)
			.with({ kind: "PostRequest" }, (message) => {
				return Vector(Message).enc([
					{
						tag: "RequestMessage",
						value: {
							requests: message.requests.map(
								(post_request) => convertIPostRequestToCodec(post_request).value,
							),
							proof: {
								height: {
									height: message.proof.height,
									id: {
										consensusStateId: Array.from(toBytes(message.proof.consensusStateId)),
										id: convertStateMachineIdToEnum(message.proof.stateMachine),
									},
								},
								proof: Array.from(hexToBytes(message.proof.proof)),
							},
							signer: Array.from(hexToBytes(message.signer)),
						},
					},
				])
			})
			.with({ kind: "GetResponse" }, (message) => {
				throw new Error("GetResponse is not yet supported on Substrate chains")
			})
			.with({ kind: "GetRequest" }, (message) => {
				return GetRequestsWithProof.enc({
					requests: message.requests.map((request) => convertIGetRequestToCodec(request)),
					source: {
						height: {
							height: message.source.height,
							id: {
								consensusStateId: Array.from(toBytes(message.source.consensusStateId)),
								id: convertStateMachineIdToEnum(message.source.stateMachine),
							},
						},
						proof: Array.from(hexToBytes(message.source.proof)),
					},
					response: {
						height: {
							height: message.response.height,
							id: {
								consensusStateId: Array.from(toBytes(message.response.consensusStateId)),
								id: convertStateMachineIdToEnum(message.response.stateMachine),
							},
						},
						proof: Array.from(hexToBytes(message.response.proof)),
					},
					signer: Array.from(hexToBytes(message.signer)),
				})
			})
			.with({ kind: "TimeoutPostRequest" }, (message) => {
				return Vector(Message).enc([
					{
						tag: "TimeoutMessage",
						value: {
							tag: "Post",
							value: {
								requests: message.requests.map((r) => convertIPostRequestToCodec(r)),
								proof: {
									height: {
										height: message.proof.height,
										id: {
											consensusStateId: Array.from(toBytes(message.proof.consensusStateId)),
											id: convertStateMachineIdToEnum(message.proof.stateMachine),
										},
									},
									proof: Array.from(hexToBytes(message.proof.proof)),
								},
							},
						},
					},
				])
			})
			.with({ kind: "Consensus" }, () => {
				throw new Error("Consensus encoding is only supported on EVM chains")
			})
			.with({ kind: "BatchConsensusAndPostRequest" }, () => {
				throw new Error("BatchConsensusAndPostRequest is only supported on EVM chains")
			})
			.with({ kind: "BatchConsensusAndGetResponse" }, () => {
				throw new Error("BatchConsensusAndGetResponse is only supported on EVM chains")
			})
			.exhaustive()
	} catch (error) {
		throw new Error("Failed to encode ISMP message", { cause: error })
	}
}
