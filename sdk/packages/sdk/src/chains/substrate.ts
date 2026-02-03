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
	SubstrateStateProof,
	isEvmChain,
	isSubstrateChain,
	replaceWebsocketWithHttp,
} from "@/utils"
import { ExpectedError } from "@/utils/exceptions"
import { keccakAsU8a } from "@polkadot/util-crypto"

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

	constructor(private readonly params: ISubstrateConfig) {
		const url = this.params.wsUrl

		const httpUrl = replaceWebsocketWithHttp(url)
		this.rpcClient = new HttpRpcClient(httpUrl)
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
		const prefix = toHex(":child_storage:default:ISMP")
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
		const prefix = toHex(":child_storage:default:ISMP")
		const key = this.requestReceiptKey(commitment)

		const item: any = await this.rpcClient.call("childstate_getStorage", [prefix, key])

		return item
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
			const encoded = SubstrateStateProof.enc({
				tag: "OverlayProof",
				value: {
					hasher: {
						tag: this.params.hasher,
						value: undefined,
					},
					storageProof: basicProof,
				},
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
	 * @returns The state proof as a hexadecimal string.
	 */
	async queryStateProof(at: bigint, keys: HexString[]): Promise<HexString> {
		const encodedKeys = keys.map((key) => Array.from(hexToBytes(key)))
		const proof: any = await this.rpcClient.call("ismp_queryChildTrieProof", [Number(at), encodedKeys])
		const basicProof = BasicProof.dec(toHex(proof.proof))
		const encoded = SubstrateStateProof.enc({
			tag: "OverlayProof",
			value: {
				hasher: {
					tag: this.params.hasher,
					value: undefined,
				},
				storageProof: basicProof,
			},
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
	 * Get the state machine update time for a given state machine height.
	 * @param {StateMachineHeight} stateMachineHeight - The state machine height.
	 * @returns {Promise<bigint>} The statemachine update time in seconds.
	 */
	async stateMachineUpdateTime(stateMachineHeight: StateMachineHeight): Promise<bigint> {
		const state_id = convertStateIdToStateMachineId(stateMachineHeight.id.stateId)

		const stateMachineId = {
			state_id,
			consensus_state_id: stateMachineHeight.id.consensusStateId,
		}

		const payload = {
			id: stateMachineId,
			height: Number(stateMachineHeight.height),
		}

		const updateTime: number = await this.rpcClient.call("ismp_queryStateMachineUpdateTime", [payload])
		return BigInt(updateTime)
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
	 * Returns the index of a pallet by its name, by looking up the pallets in the runtime metadata.
	 * @param {string} name - The name of the pallet.
	 * @returns {number} The index of the pallet.
	 */
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
			.exhaustive()
	} catch (error) {
		throw new Error("Failed to encode ISMP message", { cause: error })
	}
}
