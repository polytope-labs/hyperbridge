import type { PublicClient, TransactionReceipt } from "viem"
import { bytesToHex, getAddress, hexToBytes } from "viem"
import type { CodecType } from "scale-ts"

import type { IChain, IIsmpMessage } from "@/chain"
import { EvmChain, requestCommitmentKey, responseCommitmentKey, type EvmChainParams } from "@/chains/evm"
import type {
	HexString,
	IGetRequest,
	IMessage,
	IPharosConfig,
	IPostRequest,
	StateMachineHeight,
	StateMachineIdParams,
} from "@/types"
import { replaceWebsocketWithHttp } from "@/utils"
import {
	AccountProofDataCodec,
	NonExistenceProofCodec,
	PharosProofNodeCodec,
	PharosStateProof,
} from "@/utils/pharos"

/**
 * Chain parameters for a Pharos EVM chain. Pharos reuses the EVM JSON-RPC
 * surface for reads, writes and host queries — proof fetching is the only
 * place where the Pharos client deviates from a standard EVM chain.
 */
export type PharosChainParams = EvmChainParams

type PharosProofNode = CodecType<typeof PharosProofNodeCodec>
type NonExistenceProof = CodecType<typeof NonExistenceProofCodec>
type AccountProofData = CodecType<typeof AccountProofDataCodec>

/** Raw JSON-RPC proof-node shape returned by a Pharos node. */
interface RpcProofNode {
	proofNode: string
	nextBeginOffset: number
	nextEndOffset: number
}

interface RpcSiblingProof {
	slotIndex: number
	leftmostLeafKey: string
	proofPath: RpcProofNode[]
}

interface RpcStorageProofEntry {
	key: string
	value: string
	proof: RpcProofNode[]
	isExist: boolean
	siblingLeftmostLeafProofs?: RpcSiblingProof[]
}

interface RpcAccountProof {
	accountProof: RpcProofNode[]
	balance: string
	codeHash: string
	nonce: string
	storageHash: string
	/** RLP-encoded account value returned by Pharos nodes. */
	rawValue: string
	storageProof: RpcStorageProofEntry[]
}

/** Minimal HTTP JSON-RPC client used to reach the Pharos node's non-standard endpoints. */
class EvmHttpRpc {
	constructor(private readonly url: string) {}

	async call<T>(method: string, params: unknown[] = []): Promise<T> {
		const response = await fetch(this.url, {
			method: "POST",
			headers: { "Content-Type": "application/json" },
			body: JSON.stringify({ jsonrpc: "2.0", id: Date.now(), method, params }),
		})

		if (!response.ok) {
			throw new Error(`Pharos RPC HTTP error: ${response.status}`)
		}

		const json = (await response.json()) as { result?: T; error?: { message: string } }
		if (json.error) {
			throw new Error(`Pharos RPC error: ${json.error.message}`)
		}
		if (json.result === undefined) {
			throw new Error("Pharos RPC: missing result")
		}
		return json.result
	}
}

/** Pad a hex-encoded value to exactly 32 bytes, big-endian (mirrors the Rust tesseract client). */
function padTo32Bytes(hex: string): Uint8Array {
	const bytes = hexToBytes(hex as HexString)
	if (bytes.length > 32) {
		throw new Error(`Pharos value exceeds 32 bytes: ${hex}`)
	}
	const out = new Uint8Array(32)
	out.set(bytes, 32 - bytes.length)
	return out
}

function rpcToProofNodes(nodes: RpcProofNode[]): PharosProofNode[] {
	return nodes.map((n) => ({
		proofNode: Array.from(hexToBytes(n.proofNode as HexString)),
		nextBeginOffset: n.nextBeginOffset,
		nextEndOffset: n.nextEndOffset,
	}))
}

function rpcToSiblingProofs(siblings: RpcSiblingProof[] | undefined) {
	return (siblings ?? []).map((s) => ({
		slotIndex: s.slotIndex,
		leftmostLeafKey: Array.from(hexToBytes(s.leftmostLeafKey as HexString)),
		proofPath: rpcToProofNodes(s.proofPath),
	}))
}

function blockTag(at: bigint | undefined): string {
	return at === undefined ? "latest" : `0x${at.toString(16)}`
}

/**
 * Accumulator mirroring the Rust `PharosStateProof`. Kept as plain Maps so we can
 * collect entries per storage query and sort them before SCALE encoding.
 */
interface PharosProofAccumulator {
	storageProof: Map<string, PharosProofNode[]>
	storageValues: Map<string, number[]>
	nonExistenceProofs: Map<string, NonExistenceProof>
	accountProofs: Map<string, AccountProofData>
}

function newAccumulator(): PharosProofAccumulator {
	return {
		storageProof: new Map(),
		storageValues: new Map(),
		nonExistenceProofs: new Map(),
		accountProofs: new Map(),
	}
}

function hexKey(bytes: Uint8Array): string {
	return bytesToHex(bytes)
}

function mapToEntries<V>(m: Map<string, V>): [number[], V][] {
	const entries: [number[], V][] = []
	for (const [k, v] of m.entries()) {
		entries.push([Array.from(hexToBytes(k as HexString)), v])
	}
	return entries
}

function encodeAccumulator(acc: PharosProofAccumulator): HexString {
	const encoded = PharosStateProof.enc({
		storageProof: mapToEntries(acc.storageProof),
		storageValues: mapToEntries(acc.storageValues),
		nonExistenceProofs: mapToEntries(acc.nonExistenceProofs),
		accountProofs: mapToEntries(acc.accountProofs),
	})
	return bytesToHex(encoded) as HexString
}

/**
 * Pharos EVM chain client.
 *
 * Delegates every standard EVM interaction to {@link EvmChain} and overrides
 * {@link PharosChain.queryProof} / {@link PharosChain.queryStateProof} to speak
 * Pharos's custom `eth_getProof` response format — mirroring the Rust
 * `tesseract-messaging-pharos-evm` client.
 */
export class PharosChain implements IChain {
	private readonly evm: EvmChain
	private readonly rpc: EvmHttpRpc

	static fromParams(params: PharosChainParams): PharosChain {
		const evm = EvmChain.fromParams(params)
		return new PharosChain(params, evm)
	}

	/**
	 * Creates a `PharosChain` by auto-detecting the EVM chain ID and `IsmpHost`
	 * address via {@link EvmChain.create}.
	 *
	 * @param rpcUrl - HTTP(S) JSON-RPC URL of the Pharos node
	 * @param bundlerUrl - Optional ERC-4337 bundler URL forwarded to `EvmChain.create`
	 */
	static async create(rpcUrl: string, bundlerUrl?: string): Promise<PharosChain> {
		const evm = await EvmChain.create(rpcUrl, bundlerUrl)
		const chainId = Number.parseInt(evm.config.stateMachineId.replace(/^EVM-/, ""), 10)
		if (!Number.isFinite(chainId)) {
			throw new Error(`Unexpected EVM stateMachineId: ${evm.config.stateMachineId}`)
		}
		const params: PharosChainParams = {
			chainId,
			rpcUrl: evm.config.rpcUrl,
			host: evm.config.host,
			consensusStateId: evm.config.consensusStateId,
			bundlerUrl: evm.bundlerUrl,
		}
		return new PharosChain(params, evm)
	}

	private constructor(
		private readonly params: PharosChainParams,
		evm: EvmChain,
	) {
		this.evm = evm
		this.rpc = new EvmHttpRpc(replaceWebsocketWithHttp(params.rpcUrl))
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

	get config(): IPharosConfig {
		return this.evm.config
	}

	private hostAddress(): HexString {
		return getAddress(this.evm.host) as HexString
	}

	/**
	 * Fetch a storage proof for `slotKeys` under `address` at the given block and
	 * merge the response into the supplied accumulator. Mirrors the membership
	 * proof path of `PharosEvmClient::fetch_pharos_proof` on the Rust side.
	 */
	private async fetchStorageProofInto(
		acc: PharosProofAccumulator,
		at: bigint,
		address: HexString,
		slotKeys: HexString[],
	): Promise<void> {
		const rpcProof = await this.rpc.call<RpcAccountProof>("eth_getProof", [
			address,
			slotKeys,
			blockTag(at),
		])

		for (const sp of rpcProof.storageProof) {
			const slotKey = hexKey(padTo32Bytes(sp.key))
			if (sp.isExist) {
				acc.storageProof.set(slotKey, rpcToProofNodes(sp.proof))
				acc.storageValues.set(slotKey, Array.from(padTo32Bytes(sp.value)))
			} else {
				acc.nonExistenceProofs.set(slotKey, {
					proofNodes: rpcToProofNodes(sp.proof),
					siblingProofs: rpcToSiblingProofs(sp.siblingLeftmostLeafProofs),
				})
			}
		}
	}

	/**
	 * Fetch an account proof (no storage keys) at the given block and record it on
	 * the accumulator. Mirrors `PharosEvmClient::fetch_account_proof`.
	 */
	private async fetchAccountProofInto(
		acc: PharosProofAccumulator,
		at: bigint,
		address: HexString,
	): Promise<void> {
		const rpcProof = await this.rpc.call<RpcAccountProof>("eth_getProof", [
			address,
			[],
			blockTag(at),
		])

		const addrBytes = hexToBytes(address)
		acc.accountProofs.set(hexKey(addrBytes), {
			proofNodes: rpcToProofNodes(rpcProof.accountProof),
			rawValue: Array.from(hexToBytes(rpcProof.rawValue as HexString)),
		})
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

	/**
	 * Query a Pharos state proof for the request/response commitments in `message`.
	 *
	 * Mirrors `PharosEvmClient::query_requests_proof` / `query_responses_proof`:
	 * for every commitment we derive the second storage slot (`slot1` from
	 * {@link requestCommitmentKey} / {@link responseCommitmentKey}) and request it
	 * from the ISMP host via `eth_getProof`.
	 */
	async queryProof(message: IMessage, _counterparty: string, at?: bigint): Promise<HexString> {
		if (at === undefined) {
			throw new Error("PharosChain.queryProof requires an explicit block height `at`")
		}
		const slotKeys: HexString[] =
			"Requests" in message
				? message.Requests.map((c) => requestCommitmentKey(c).slot1 as HexString)
				: message.Responses.map((c) => responseCommitmentKey(c) as HexString)

		const acc = newAccumulator()
		await this.fetchStorageProofInto(acc, at, this.hostAddress(), slotKeys)
		return encodeAccumulator(acc)
	}

	/**
	 * Query a Pharos state proof for arbitrary keys.
	 *
	 * Supports the same key shapes as `PharosEvmClient::query_state_proof`:
	 *   - 32-byte keys: ISMP host storage slots
	 *   - 52-byte keys: `address (20) || slot (32)` grouped by contract
	 *   - 20-byte keys: account queries (proves the account leaf itself)
	 *
	 * If `address` is provided, all keys are treated as 32-byte slots under that
	 * contract (matching the `EvmChain.queryStateProof` contract-override shape).
	 */
	async queryStateProof(at: bigint, keys: HexString[], address?: HexString): Promise<HexString> {
		const acc = newAccumulator()

		if (address !== undefined) {
			await this.fetchStorageProofInto(acc, at, getAddress(address) as HexString, keys)
			return encodeAccumulator(acc)
		}

		const keyBytes = keys.map((k) => hexToBytes(k))
		const hostAddr = this.hostAddress()

		const ismpSlots: HexString[] = []
		const groups = new Map<string, HexString[]>()
		const accountQueries: HexString[] = []

		for (const bytes of keyBytes) {
			if (bytes.length === 32) {
				ismpSlots.push(bytesToHex(bytes) as HexString)
			} else if (bytes.length === 52) {
				const addr = getAddress(bytesToHex(bytes.subarray(0, 20))) as HexString
				const slot = bytesToHex(bytes.subarray(20, 52)) as HexString
				const list = groups.get(addr) ?? []
				list.push(slot)
				groups.set(addr, list)
			} else if (bytes.length === 20) {
				accountQueries.push(getAddress(bytesToHex(bytes)) as HexString)
			} else {
				throw new Error(
					`PharosChain.queryStateProof: unsupported key length ${bytes.length}; expected 20, 32, or 52`,
				)
			}
		}

		if (ismpSlots.length > 0) {
			await this.fetchStorageProofInto(acc, at, hostAddr, ismpSlots)
		}
		for (const [addr, slots] of groups) {
			await this.fetchStorageProofInto(acc, at, addr as HexString, slots)
		}
		for (const addr of accountQueries) {
			await this.fetchAccountProofInto(acc, at, addr)
		}

		return encodeAccumulator(acc)
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
