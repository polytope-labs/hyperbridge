import type { PublicClient, TransactionReceipt } from "viem"
import { bytesToHex, getAddress, hexToBytes } from "viem"
import { u8aConcat } from "@polkadot/util"
import { blake2AsU8a, xxhashAsU8a } from "@polkadot/util-crypto"

import type { IChain, IIsmpMessage } from "@/chain"
import { EvmChain, requestCommitmentKey, responseCommitmentKey, type EvmChainParams } from "@/chains/evm"
import type {
	HexString,
	IGetRequest,
	IMessage,
	IPolkadotHubConfig,
	IPostRequest,
	StateMachineHeight,
	StateMachineIdParams,
} from "@/types"
import { replaceWebsocketWithHttp } from "@/utils"
import { decodeReviveContractTrieId } from "@/utils/reviveAccount"
import { encodeSubstrateEvmProofBytes } from "@/utils/substrate"

/** Substrate default child trie prefix (`ChildInfo::new_default`). */
const DEFAULT_CHILD_STORAGE_PREFIX = new TextEncoder().encode(":child_storage:default:")

/**
 * Full chain params: EVM JSON-RPC + Ismp host plus a Substrate JSON-RPC URL for proof queries.
 */
export type PolkadotHubChainParams = EvmChainParams & {
	substrateRpcUrl: string
}

interface ReadProofRpc {
	at?: string
	proof: string[]
}

class SubstrateHttpRpc {
	constructor(private readonly url: string) {}

	async call(method: string, params: unknown[] = []): Promise<unknown> {
		const body = JSON.stringify({
			jsonrpc: "2.0",
			id: Date.now(),
			method,
			params,
		})

		const response = await fetch(this.url, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body,
		})

		if (!response.ok) {
			throw new Error(`Substrate RPC HTTP error: ${response.status}`)
		}

		const json = (await response.json()) as { result?: unknown; error?: { message: string } }
		if (json.error) {
			throw new Error(`Substrate RPC error: ${json.error.message}`)
		}

		return json.result
	}
}

function contractInfoKey(address20: Uint8Array): Uint8Array {
	const key = new Uint8Array(16 + 16 + 20)
	key.set(xxhashAsU8a("Revive", 128), 0)
	key.set(xxhashAsU8a("AccountInfoOf", 128), 16)
	key.set(address20, 32)
	return key
}

function childPrefixedStorageKey(trieId: Uint8Array): Uint8Array {
	return u8aConcat(DEFAULT_CHILD_STORAGE_PREFIX, trieId)
}

function storageKeyForSlot(slot32: Uint8Array): Uint8Array {
	return blake2AsU8a(slot32, 256)
}

function hexKey(k: Uint8Array): HexString {
	return bytesToHex(k) as HexString
}

/**
 * Polkadot Hub (EVM on Substrate / Revive): EVM RPC + host for reads and txs; Substrate RPC for
 * child-trie proofs aligned with `tesseract/messaging/substrate-evm` (`query_requests_proof` /
 * `query_state_proof`).
 */
export class PolkadotHubChain implements IChain {
	private readonly evm: EvmChain
	private readonly substrateRpc: SubstrateHttpRpc

	static fromParams(params: PolkadotHubChainParams): PolkadotHubChain {
		const { substrateRpcUrl, ...evmParams } = params
		const evm = EvmChain.fromParams(evmParams)
		return new PolkadotHubChain(params, evm)
	}

	/**
	 * Creates a `PolkadotHubChain` by auto-detecting the EVM chain ID and `IsmpHost` address via
	 * {@link EvmChain.create}, plus a Substrate RPC URL for Revive child-trie proofs.
	 *
	 * @param evmRpcUrl - HTTP(S) JSON-RPC URL of the EVM (Revive) node
	 * @param substrateRpcUrl - Substrate node RPC (HTTP or WebSocket) for proof queries
	 * @param bundlerUrl - Optional ERC-4337 bundler URL (forwarded to `EvmChain.create`)
	 */
	static async create(evmRpcUrl: string, substrateRpcUrl: string, bundlerUrl?: string): Promise<PolkadotHubChain> {
		const evm = await EvmChain.create(evmRpcUrl, bundlerUrl)
		const chainId = Number.parseInt(evm.config.stateMachineId.replace(/^EVM-/, ""), 10)
		if (!Number.isFinite(chainId)) {
			throw new Error(`Unexpected EVM stateMachineId: ${evm.config.stateMachineId}`)
		}
		const params: PolkadotHubChainParams = {
			chainId,
			rpcUrl: evm.config.rpcUrl,
			host: evm.config.host,
			consensusStateId: evm.config.consensusStateId,
			bundlerUrl: evm.bundlerUrl,
			substrateRpcUrl,
		}
		return new PolkadotHubChain(params, evm)
	}

	private constructor(
		private readonly params: PolkadotHubChainParams,
		evm: EvmChain,
	) {
		this.evm = evm
		this.substrateRpc = new SubstrateHttpRpc(replaceWebsocketWithHttp(params.substrateRpcUrl))
	}

	get client(): PublicClient {
		return this.evm.client
	}

	get host(): HexString {
		return this.evm.host
	}

	get bundlerUrl(): string | undefined {
		return this.evm.bundlerUrl
	}

	get configService() {
		return this.evm.configService
	}

	get config(): IPolkadotHubConfig {
		return {
			...this.evm.config,
			substrateRpcUrl: this.params.substrateRpcUrl,
		}
	}

	private hostAddress20(): Uint8Array {
		return hexToBytes(getAddress(this.evm.host))
	}

	private async fetchCombinedProof(at: bigint, queries: Map<Uint8Array, Uint8Array[]>): Promise<HexString> {
		const height = Number(at)
		if (!Number.isSafeInteger(height) || height < 0) {
			throw new Error("Block height must be a non-negative safe integer for Substrate RPC")
		}

		const blockHash = (await this.substrateRpc.call("chain_getBlockHash", [height])) as string | null
		if (!blockHash) {
			throw new Error(`Block hash not found for height ${height}`)
		}

		const mainKeys: HexString[] = []
		const childInfoByAddr = new Map<string, { trieId: Uint8Array; prefixed: Uint8Array }>()
		const contractEntries = [...queries.entries()]

		for (const [addr20] of contractEntries) {
			const infoKey = contractInfoKey(addr20)
			const storageHex = (await this.substrateRpc.call("state_getStorage", [hexKey(infoKey), blockHash])) as
				| string
				| null
			if (!storageHex) {
				throw new Error(`Revive AccountInfo not found for contract ${hexKey(addr20)}`)
			}
			const trieId = decodeReviveContractTrieId(hexToBytes(storageHex as HexString))
			const prefixed = childPrefixedStorageKey(trieId)
			mainKeys.push(hexKey(infoKey))
			mainKeys.push(hexKey(prefixed))
			childInfoByAddr.set(hexKey(addr20), { trieId, prefixed })
		}

		const mainRead = (await this.substrateRpc.call("state_getReadProof", [mainKeys, blockHash])) as ReadProofRpc
		const mainProofBytes = mainRead.proof.map((p) => hexToBytes(p as HexString))

		const storageProofEncoded = new Map<Uint8Array, Uint8Array[]>()

		for (const [addr20, innerKeys] of contractEntries) {
			const addrHex = hexKey(addr20)
			const info = childInfoByAddr.get(addrHex)
			if (!info) {
				throw new Error("Internal error: missing child info for contract")
			}
			const childKeysHex = innerKeys.map((k) => hexKey(k))
			const childRead = (await this.substrateRpc.call("state_getChildReadProof", [
				hexKey(info.prefixed),
				childKeysHex,
				blockHash,
			])) as ReadProofRpc

			storageProofEncoded.set(
				addr20,
				childRead.proof.map((p) => hexToBytes(p as HexString)),
			)
		}

		const encoded = encodeSubstrateEvmProofBytes({
			mainProof: mainProofBytes,
			storageProof: storageProofEncoded,
		})
		return bytesToHex(encoded) as HexString
	}

	timestamp(): Promise<bigint> {
		return this.evm.timestamp()
	}

	requestReceiptKey(commitment: HexString): HexString {
		return this.evm.requestReceiptKey(commitment)
	}

	queryRequestReceipt(commitment: HexString): Promise<HexString | undefined> {
		return this.evm.queryRequestReceipt(commitment)
	}

	async queryProof(message: IMessage, _counterparty: string, at?: bigint): Promise<HexString> {
		if (at === undefined) {
			throw new Error("PolkadotHubChain.queryProof requires an explicit block height `at`")
		}
		const host = this.hostAddress20()
		const storageKeys =
			"Requests" in message
				? message.Requests.map((c) => storageKeyForSlot(hexToBytes(requestCommitmentKey(c).slot1)))
				: message.Responses.map((c) => storageKeyForSlot(hexToBytes(responseCommitmentKey(c))))

		const q = new Map<Uint8Array, Uint8Array[]>()
		q.set(host, storageKeys)
		return this.fetchCombinedProof(at, q)
	}

	async queryStateProof(at: bigint, keys: HexString[], _address?: HexString): Promise<HexString> {
		const keyBytes = keys.map((k) => hexToBytes(k))
		const host = this.hostAddress20()

		if (keyBytes.every((k) => k.length === 32)) {
			const storageKeys = keyBytes.map((slot) => storageKeyForSlot(slot))
			const q = new Map<Uint8Array, Uint8Array[]>()
			q.set(host, storageKeys)
			return this.fetchCombinedProof(at, q)
		}

		if (keyBytes.every((k) => k.length === 52)) {
			const groups = new Map<string, Uint8Array[]>()
			for (const full of keyBytes) {
				const addr = full.subarray(0, 20)
				const slot = full.subarray(20, 52)
				const h = hexKey(addr)
				const arr = groups.get(h) ?? []
				arr.push(storageKeyForSlot(slot))
				groups.set(h, arr)
			}
			const q = new Map<Uint8Array, Uint8Array[]>()
			for (const [addrHex, sks] of groups) {
				q.set(hexToBytes(addrHex as HexString), sks)
			}
			return this.fetchCombinedProof(at, q)
		}

		throw new Error(
			"PolkadotHubChain.queryStateProof: keys must be either all 32-byte ISMP slots or all 52-byte (20-byte address + 32-byte slot) entries",
		)
	}

	encode(message: IIsmpMessage): HexString {
		return this.evm.encode(message)
	}

	latestStateMachineHeight(stateMachineId: StateMachineIdParams): Promise<bigint> {
		return this.evm.latestStateMachineHeight(stateMachineId)
	}

	challengePeriod(stateMachineId: StateMachineIdParams): Promise<bigint> {
		return this.evm.challengePeriod(stateMachineId)
	}

	stateMachineUpdateTime(stateMachineHeight: StateMachineHeight): Promise<bigint> {
		return this.evm.stateMachineUpdateTime(stateMachineHeight)
	}

	getHostNonce(): Promise<bigint> {
		return this.evm.getHostNonce()
	}

	quoteNative(request: IPostRequest | IGetRequest, fee: bigint): Promise<bigint> {
		return this.evm.quoteNative(request, fee)
	}

	getFeeTokenWithDecimals(): Promise<{ address: HexString; decimals: number }> {
		return this.evm.getFeeTokenWithDecimals()
	}

	getPlaceOrderCalldata(txHash: string, intentGatewayAddress: string): Promise<HexString> {
		return this.evm.getPlaceOrderCalldata(txHash, intentGatewayAddress)
	}

	estimateGas(request: IPostRequest): Promise<{ gas: bigint; postRequestCalldata: HexString }> {
		return this.evm.estimateGas(request)
	}

	broadcastTransaction(signedTransaction: HexString): Promise<TransactionReceipt> {
		return this.evm.broadcastTransaction(signedTransaction)
	}

	getTransactionReceipt(hash: HexString): Promise<TransactionReceipt> {
		return this.evm.getTransactionReceipt(hash)
	}
}
