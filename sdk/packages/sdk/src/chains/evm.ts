import {
	http,
	type PublicClient,
	bytesToBigInt,
	bytesToHex,
	createPublicClient,
	encodeFunctionData,
	erc20Abi,
	hexToBytes,
	keccak256,
	pad,
	toBytes,
	toFunctionSelector,
	toHex,
	maxUint256,
} from "viem"
import {
	arbitrum,
	arbitrumSepolia,
	base,
	baseSepolia,
	bsc,
	bscTestnet,
	gnosis,
	gnosisChiado,
	mainnet,
	optimism,
	optimismSepolia,
	polygon,
	unichain,
	soneium,
	tron,
} from "viem/chains"
import { chainConfigs, pharosAtlantic, pharosMainnet, polkadotAssetHubPaseo, tronNile } from "@/configs/chain"

import { flatten, zip } from "lodash-es"
import { match } from "ts-pattern"
import type { GetProofParameters, Hex, TransactionReceipt } from "viem"

import EvmHost from "@/abis/evmHost"
import evmHost from "@/abis/evmHost"
import HandlerV2 from "@/abis/handlerV2"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { IChain, IIsmpMessage } from "@/chain"
import { ChainConfigService } from "@/configs/ChainConfigService"
import type {
	HexString,
	IEvmConfig,
	IGetRequest,
	IMessage,
	IPostRequest,
	StateMachineHeight,
	StateMachineIdParams,
} from "@/types"
import {
	ADDRESS_ZERO,
	EvmStateProof,
	getContractCallInputs,
	MmrProof,
	SubstrateStateMachineProof,
	generateRootWithProof,
} from "@/utils"

import UniswapV2Factory from "@/abis/uniswapV2Factory"
import UniswapRouterV2 from "@/abis/uniswapRouterV2"

const chains = {
	[mainnet.id]: mainnet,
	[arbitrum.id]: arbitrum,
	[arbitrumSepolia.id]: arbitrumSepolia,
	[optimism.id]: optimism,
	[optimismSepolia.id]: optimismSepolia,
	[base.id]: base,
	[baseSepolia.id]: baseSepolia,
	[soneium.id]: soneium,
	[bsc.id]: bsc,
	[bscTestnet.id]: bscTestnet,
	[gnosis.id]: gnosis,
	[gnosisChiado.id]: gnosisChiado,
	[polygon.id]: polygon,
	[unichain.id]: unichain,
	[tron.id]: tron,
	[tronNile.id]: tronNile,
	[polkadotAssetHubPaseo.id]: polkadotAssetHubPaseo,
	[pharosMainnet.id]: pharosMainnet,
	[pharosAtlantic.id]: pharosAtlantic,
}

/**
 * The default address used as fallback when no address is provided.
 * This represents the zero address in EVM chains.
 */
export const DEFAULT_ADDRESS = "0x0000000000000000000000000000000000000000"

/**
 * ERC-165 interface ID for IHandlerV2 (bytes4(keccak256("batchCall(bytes[])"))).
 */

/**
 * Parameters for an EVM chain.
 */
export interface EvmChainParams {
	/**
	 * The chain ID of the EVM chain
	 */
	chainId: number
	/**
	 * The RPC URL of the EVM chain
	 */
	rpcUrl: string
	/**
	 * The host address of the EVM chain (IsmpHost contract address)
	 */
	host: HexString
	/**
	 * Consensus state identifier of this chain on hyperbridge
	 */
	consensusStateId?: string
	/**
	 * Optional ERC-4337 bundler URL for account abstraction support
	 */
	bundlerUrl?: string
}

/**
 * Encapsulates an EVM chain.
 */
export class EvmChain implements IChain {
	private publicClient: PublicClient
	private chainConfigService: ChainConfigService

	private constructor(private readonly params: EvmChainParams) {
		// Default consensus state IDs for known chains
		const defaultConsensusStateIds: Record<number, string> = {
			1: "ETH0", // Ethereum Mainnet
			11155111: "ETH0", // Sepolia
			42161: "ETH0", // Arbitrum One
			421614: "ETH0", // Arbitrum Sepolia
			10: "ETH0", // Optimism
			11155420: "ETH0", // Optimism Sepolia
			8453: "ETH0", // Base
			84532: "ETH0", // Base Sepolia
			130: "ETH0", // Unichain
			1868: "ETH0", // Soneium
			137: "POLY", // Polygon Mainnet
			80002: "POLY", // Polygon Amoy
			56: "BSC0", // BSC
			97: "BSC0", // BSC Testnet
			100: "GNO0", // Gnosis
			10200: "GNO0", // Gnosis Chiado
			420420417: "PAS0", // Polkadot Asset Hub (Paseo)
			420420419: "DOT0", // Polkadot Asset Hub (Polkadot)
			688600: "PHAR", // Pharos Mainnet
			688689: "PHAR", // Pharos Atlantic (Testnet)
		}

		// Set default consensusStateId if not provided
		if (!params.consensusStateId) {
			params.consensusStateId = defaultConsensusStateIds[params.chainId]
		}

		// @ts-ignore
		this.publicClient = createPublicClient({
			// @ts-ignore
			chain: chains[params.chainId],
			transport: http(params.rpcUrl),
		})
		this.chainConfigService = new ChainConfigService()
	}

	/**
	 * Creates an `EvmChain` instance directly from a fully-specified config object.
	 * Use this when you already know the chain ID, host address, and other parameters.
	 *
	 * This is the only public way to construct an `EvmChain` with explicit params — the constructor is private.
	 *
	 * @param params - Full EVM chain configuration
	 * @returns An `EvmChain` instance
	 */
	static fromParams(params: EvmChainParams): EvmChain {
		return new EvmChain(params)
	}

	/**
	 * Creates an `EvmChain` instance by auto-detecting the chain ID from the RPC endpoint
	 * and resolving the correct `IsmpHost` contract address for that chain.
	 *
	 * This is the only public way to construct an `EvmChain` — the constructor is private.
	 *
	 * @param rpcUrl - HTTP(S) RPC URL of the EVM node
	 * @param bundlerUrl - Optional ERC-4337 bundler URL for account abstraction support
	 * @returns A fully initialised `EvmChain` ready for use
	 * @throws If the chain ID returned by the RPC is not a known Hyperbridge deployment
	 *
	 * @example
	 * ```typescript
	 * const chain = await EvmChain.create("https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY")
	 * const chain = await EvmChain.create("https://mainnet.base.org", "https://bundler.example.com")
	 * ```
	 */
	static async create(rpcUrl: string, bundlerUrl?: string): Promise<EvmChain> {
		// Use a chainless transport to fetch the chain ID before we know which chain we're on
		const tempClient = createPublicClient({ transport: http(rpcUrl) })
		const chainId = await tempClient.getChainId()

		const host = chainConfigs[chainId]?.addresses?.Host
		if (!host) {
			throw new Error(`No known IsmpHost address for chain ID ${chainId}. Provide the host address explicitly.`)
		}

		return new EvmChain({ chainId, rpcUrl, host, bundlerUrl })
	}

	// Expose minimal getters for external helpers/classes
	get client(): PublicClient {
		return this.publicClient
	}

	get host(): HexString {
		return this.params.host
	}

	get bundlerUrl(): string | undefined {
		return this.params.bundlerUrl
	}

	get config(): IEvmConfig {
		return {
			rpcUrl: this.params.rpcUrl,
			stateMachineId: `EVM-${this.params.chainId}`,
			host: this.params.host,
			consensusStateId: this.params.consensusStateId!,
		}
	}

	get configService(): ChainConfigService {
		return this.chainConfigService
	}

	/**
	 * Returns the current authority set epoch from the Host contract.
	 */
	async currentEpoch(): Promise<bigint> {
		const epoch = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "currentEpoch",
		})
		return BigInt(epoch)
	}

	/**
	 * Derives the key for the request receipt.
	 * @param {HexString} commitment - The commitment to derive the key from.
	 * @returns {HexString} The derived key.
	 */
	requestReceiptKey(commitment: HexString): HexString {
		return deriveMapKey(hexToBytes(commitment), REQUEST_RECEIPTS_SLOT)
	}

	/**
	 * Queries the request receipt.
	 * @param {HexString} commitment - The commitment to query.
	 * @returns {Promise<HexString | undefined>} The relayer address responsible for delivering the request.
	 */
	async queryRequestReceipt(commitment: HexString): Promise<HexString | undefined> {
		const relayer = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "requestReceipts",
			args: [commitment],
		})

		// solidity returns zeroes if the storage slot is empty
		return relayer === DEFAULT_ADDRESS ? undefined : relayer
	}

	/**
	 * Queries the proof of the commitments.
	 * @param {IMessage} message - The message to query.
	 * @param {string} counterparty - The counterparty address.
	 * @param {bigint} [at] - The block number to query at.
	 * @returns {Promise<HexString>} The proof.
	 */
	async queryProof(message: IMessage, counterparty: string, at?: bigint): Promise<HexString> {
		// for each request derive the commitment key collect into a new array
		const commitmentKeys =
			"Requests" in message
				? message.Requests.map((key) => requestCommitmentKey(key).slot1)
				: message.Responses.map((key) => responseCommitmentKey(key))
		const config: GetProofParameters = {
			address: this.params.host,
			storageKeys: commitmentKeys,
		}
		if (!at) {
			config.blockTag = "latest"
		} else {
			config.blockNumber = at
		}
		const proof = await this.publicClient.getProof(config)
		const flattenedProof = Array.from(new Set(flatten(proof.storageProof.map((item) => item.proof))))

		const encoded = EvmStateProof.enc({
			contractProof: proof.accountProof.map((item) => Array.from(hexToBytes(item))),
			storageProof: [
				[Array.from(hexToBytes(this.params.host)), flattenedProof.map((item) => Array.from(hexToBytes(item)))],
			],
		})

		return toHex(encoded)
	}

	/**
	 * Query and return the encoded storage proof for the provided keys at the given height.
	 *
	 * Keys may be either:
	 *  - 32-byte storage slots — read from `address` (or the host contract when omitted), or
	 *  - 52-byte cross-chain GET keys encoded as `address(20) || slot(32)`, where the target
	 *    contract is embedded in the key. These may span multiple contracts.
	 *
	 * @param {bigint} at - The block height at which to query the storage proof.
	 * @param {HexString[]} keys - The keys for which to query the storage proof.
	 * @param {HexString} address - Optional contract address; forces all keys to be read as slots
	 *   of this contract. Omit to let 52-byte keys carry their own contract address.
	 * @returns {Promise<HexString>} The encoded storage proof.
	 */
	async queryStateProof(at: bigint, keys: HexString[], address?: HexString): Promise<HexString> {
		// Group the requested slots by the contract they belong to.
		const slotsByContract = new Map<HexString, HexString[]>()
		for (const key of keys) {
			let contract: HexString
			let slot: HexString
			if (address) {
				contract = address.toLowerCase() as HexString
				slot = key
			} else if ((key.length - 2) / 2 === 52) {
				// address(20) || slot(32)
				contract = key.slice(0, 42).toLowerCase() as HexString
				slot = `0x${key.slice(42)}` as HexString
			} else {
				contract = this.params.host.toLowerCase() as HexString
				slot = key
			}
			const slots = slotsByContract.get(contract) ?? []
			slots.push(slot)
			slotsByContract.set(contract, slots)
		}

		const contracts = Array.from(slotsByContract.entries())
		const proofs = await Promise.all(
			contracts.map(([contract, slots]) => {
				const config: GetProofParameters = { address: contract, storageKeys: slots }
				if (!at) {
					config.blockTag = "latest"
				} else {
					config.blockNumber = at
				}
				return this.publicClient.getProof(config)
			}),
		)

		// Account proofs across contracts share trie nodes near the root; merge + dedupe them into a
		// single account-trie proof. Storage proofs stay keyed per contract.
		const contractProof = Array.from(new Set(flatten(proofs.map((proof) => proof.accountProof))))
		const storageProof = contracts.map(([contract], i) => {
			const flattened = Array.from(new Set(flatten(proofs[i].storageProof.map((item) => item.proof))))
			return [
				Array.from(hexToBytes(contract)),
				flattened.map((item) => Array.from(hexToBytes(item))),
			] as [number[], number[][]]
		})

		const encoded = EvmStateProof.enc({
			contractProof: contractProof.map((item) => Array.from(hexToBytes(item))),
			storageProof,
		})

		return toHex(encoded)
	}

	/**
	 * Returns the current timestamp of the chain.
	 * @returns {Promise<bigint>} The current timestamp.
	 */
	async timestamp(): Promise<bigint> {
		const data = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "timestamp",
		})
		return BigInt(data)
	}

	/**
	 * Get the latest state machine height for a given state machine ID.
	 * @param {StateMachineIdParams} stateMachineId - The state machine ID.
	 * @returns {Promise<bigint>} The latest state machine height.
	 */
	async latestStateMachineHeight(stateMachineId: StateMachineIdParams): Promise<bigint> {
		if (!this.publicClient) throw new Error("API not initialized")
		const id = stateMachineId.stateId.Polkadot || stateMachineId.stateId.Kusama
		if (!id)
			throw new Error(
				"Expected Polakdot or Kusama State machine id when reading latest state machine height on evm",
			)
		const data = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "latestStateMachineHeight",
			args: [BigInt(id)],
		})
		return data
	}

	/**
	 * Get the state machine update time for a given state machine height.
	 * @param {StateMachineHeight} stateMachineHeight - The state machine height.
	 * @returns {Promise<bigint>} The statemachine update time in seconds.
	 */
	async stateMachineUpdateTime(stateMachineHeight: StateMachineHeight): Promise<bigint> {
		if (!this.publicClient) throw new Error("API not initialized")
		const id = stateMachineHeight.id.stateId.Polkadot || stateMachineHeight.id.stateId.Kusama
		if (!id) throw new Error("Expected Polkadot or Kusama State machine id when reading state machine update time")
		const data = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "stateMachineCommitmentUpdateTime",
			args: [{ stateMachineId: BigInt(id), height: stateMachineHeight.height }],
		})
		return data
	}

	/**
	 * Retrieves the placeOrder calldata from a transaction using debug_traceTransaction.
	 * Filters to placeOrder calls by selector so unrelated calls to the gateway in
	 * the same transaction (e.g. quote, fillOrder) do not skew indexing.
	 * When the transaction contains multiple placeOrder calls, `occurrenceIndex`
	 * selects which call's calldata to return (0-indexed in execution order).
	 */
	async getPlaceOrderCalldata(
		txHash: string,
		intentGatewayAddress: string,
		occurrenceIndex: number = 0,
	): Promise<HexString> {
		const callInputs = await getContractCallInputs(
			this.publicClient,
			txHash as HexString,
			intentGatewayAddress,
		)
		const placeOrderSelector = toFunctionSelector(
			IntentGatewayV2ABI.find((item: any) => item.type === "function" && item.name === "placeOrder") as any,
		)
		const placeOrderInputs = callInputs.filter(
			(input) => input.slice(0, 10).toLowerCase() === placeOrderSelector.toLowerCase(),
		)
		if (placeOrderInputs.length === 0) {
			throw new Error(`Failed to extract placeOrder calldata from trace for tx ${txHash}`)
		}
		if (occurrenceIndex >= placeOrderInputs.length) {
			throw new Error(
				`placeOrder occurrence ${occurrenceIndex} out of range for tx ${txHash} (found ${placeOrderInputs.length})`,
			)
		}
		return placeOrderInputs[occurrenceIndex]
	}

	/**
	 * Get the challenge period for a given state machine id.
	 * @param {StateMachineIdParams} stateMachineId - The state machine ID.
	 * @returns {Promise<bigint>} The challenge period in seconds.
	 */
	async challengePeriod(stateMachineId: StateMachineIdParams): Promise<bigint> {
		if (!this.publicClient) throw new Error("API not initialized")
		const id = stateMachineId.stateId.Polkadot || stateMachineId.stateId.Kusama
		if (!id)
			throw new Error(
				"Expected Polkadot or Kusama State machine id when reading latest state machine height on evm",
			)
		const data = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "challengePeriod",
		})
		return data
	}

	/**
	 * Encodes an ISMP message for the EVM chain.
	 * @param {IIsmpMessage} message The ISMP message to encode.
	 * @returns {HexString} The encoded calldata.
	 */
	encode(message: IIsmpMessage): HexString {
		const encoded = match(message)
			.with({ kind: "PostRequest" }, (request) => {
				const mmrProof = MmrProof.dec(request.proof.proof)
				const requests = zip(request.requests, mmrProof.leafIndexAndPos)
					.map(([req, leafIndexAndPos]) => {
						if (!req || !leafIndexAndPos) return
						return {
							request: {
								source: toHex(req.source),
								dest: toHex(req.dest),
								to: req.to,
								from: req.from,
								nonce: req.nonce,
								timeoutTimestamp: req.timeoutTimestamp,
								body: req.body,
							} as any,
							index: leafIndexAndPos?.leafIndex!,
						}
					})
					.filter((item) => !!item)

				const proof = {
					height: {
						stateMachineId: BigInt(Number.parseInt(request.proof.stateMachine.split("-")[1])),
						height: request.proof.height,
					},
					multiproof: mmrProof.items.map((item) => bytesToHex(new Uint8Array(item))),
					leafCount: mmrProof.leafCount,
				}
				const encoded = encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "handlePostRequests",
					args: [
						this.params.host,
						{
							proof,
							requests,
						},
					],
				})

				return encoded
			})
			.with({ kind: "TimeoutPostRequest" }, (timeout) => {
				const proof = SubstrateStateMachineProof.dec(timeout.proof.proof).storageProof.map((item) =>
					toHex(new Uint8Array(item)),
				)
				const encoded = encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "handlePostRequestTimeouts",
					args: [
						this.params.host,
						{
							height: {
								stateMachineId: BigInt(Number.parseInt(timeout.proof.stateMachine.split("-")[1])),
								height: timeout.proof.height,
							},
							timeouts: timeout.requests.map((req) => ({
								source: toHex(req.source),
								dest: toHex(req.dest),
								to: req.to,
								from: req.from,
								nonce: req.nonce,
								timeoutTimestamp: req.timeoutTimestamp,
								body: req.body,
							})),
							proof,
						},
					],
				})

				return encoded
			})
			.with({ kind: "GetResponse" }, (request) => {
				const mmrProof = MmrProof.dec(request.proof.proof)
				const responses = zip(request.responses, mmrProof.leafIndexAndPos)
					.map(([req, leafIndexAndPos]) => {
						if (!req || !leafIndexAndPos) return
						return {
							response: {
								request: {
									source: toHex(req.get.source),
									dest: toHex(req.get.dest),
									from: req.get.from,
									nonce: req.get.nonce,
									timeoutTimestamp: req.get.timeoutTimestamp,
									keys: req.get.keys,
									context: req.get.context,
									height: req.get.height,
								},

								values: req.values,
							} as any,
							index: leafIndexAndPos?.leafIndex!,
						}
					})
					.filter((item) => !!item)

				const proof = {
					height: {
						stateMachineId: BigInt(Number.parseInt(request.proof.stateMachine.split("-")[1])),
						height: request.proof.height,
					},
					multiproof: mmrProof.items.map((item) => bytesToHex(new Uint8Array(item))),
					leafCount: mmrProof.leafCount,
				}
				const encoded = encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "handleGetResponses",
					args: [
						this.params.host,
						{
							proof,
							responses,
						},
					],
				})

				return encoded
			})
			.with({ kind: "Consensus" }, (message) => {
				return encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "handleConsensus",
					args: [this.params.host, message.consensusProof],
				})
			})
			.with({ kind: "BatchConsensusAndPostRequest" }, (request) => {
				const consensusCalls = request.consensusProofs.map((proof) =>
					this.encode({ kind: "Consensus", consensusProof: proof }),
				)

				const mmrProof = MmrProof.dec(request.proof.proof)
				const requests = zip(request.requests, mmrProof.leafIndexAndPos)
					.map(([req, leafIndexAndPos]) => {
						if (!req || !leafIndexAndPos) return
						return {
							request: {
								source: toHex(req.source),
								dest: toHex(req.dest),
								to: req.to,
								from: req.from,
								nonce: req.nonce,
								timeoutTimestamp: req.timeoutTimestamp,
								body: req.body,
							} as any,
							index: leafIndexAndPos?.leafIndex!,
						}
					})
					.filter((item) => !!item)

				const proof = {
					height: {
						stateMachineId: BigInt(Number.parseInt(request.proof.stateMachine.split("-")[1])),
						height: request.proof.height,
					},
					multiproof: mmrProof.items.map((item) => bytesToHex(new Uint8Array(item))),
					leafCount: mmrProof.leafCount,
				}

				const postRequestCall = encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "handlePostRequests",
					args: [
						this.params.host,
						{
							proof,
							requests,
						},
					],
				})

				return encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "batchCall",
					args: [[...consensusCalls, postRequestCall]],
				})
			})
			.with({ kind: "BatchConsensusAndGetResponse" }, (request) => {
				const consensusCalls = request.consensusProofs.map((proof) =>
					this.encode({ kind: "Consensus", consensusProof: proof }),
				)

				const mmrProof = MmrProof.dec(request.proof.proof)
				const responses = zip(request.responses, mmrProof.leafIndexAndPos)
					.map(([req, leafIndexAndPos]) => {
						if (!req || !leafIndexAndPos) return
						return {
							response: {
								request: {
									source: toHex(req.get.source),
									dest: toHex(req.get.dest),
									from: req.get.from,
									nonce: req.get.nonce,
									timeoutTimestamp: req.get.timeoutTimestamp,
									keys: req.get.keys,
									context: req.get.context,
									height: req.get.height,
								},
								values: req.values,
							} as any,
							index: leafIndexAndPos?.leafIndex!,
						}
					})
					.filter((item) => !!item)

				const proof = {
					height: {
						stateMachineId: BigInt(Number.parseInt(request.proof.stateMachine.split("-")[1])),
						height: request.proof.height,
					},
					multiproof: mmrProof.items.map((item) => bytesToHex(new Uint8Array(item))),
					leafCount: mmrProof.leafCount,
				}

				const getResponseCall = encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "handleGetResponses",
					args: [
						this.params.host,
						{
							proof,
							responses,
						},
					],
				})

				return encodeFunctionData({
					abi: HandlerV2.ABI,
					functionName: "batchCall",
					args: [[...consensusCalls, getResponseCall]],
				})
			})
			.with({ kind: "GetRequest" }, (message) => {
				throw new Error("GetResponse is not yet supported on Substrate chains")
			})
			.exhaustive()

		return encoded
	}

	/**
	 * Returns the protocol fee charged by the host on dispatch.
	 *
	 * The per-byte fee model was removed from `EvmHost`; on-chain dispatch
	 * now charges only the relayer fee carried in `DispatchPost.fee`.
	 * Bandwidth is pre-paid out-of-band via `BandwidthManager.purchase()`
	 * and metered on Hyperbridge by `pallet-bandwidth`.
	 */
	async quote(_request: IPostRequest | IGetRequest): Promise<bigint> {
		return 0n
	}

	async quoteNative(request: IPostRequest | IGetRequest, fee: bigint): Promise<bigint> {
		const totalFee = (await this.quote(request)) + fee
		const feeToken = await this.getFeeTokenWithDecimals()
		// Quote against the router the host actually swaps through on dispatch,
		// which may price differently than the canonical Uniswap V2 router.
		const hostRouter = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "uniswapV2Router",
		})
		return this.getAmountsIn(totalFee, feeToken.address, request.source, hostRouter)
	}

	/**
	 * Given a desired output amount of a token, returns how much native is needed as input.
	 * Uses the chain's Uniswap V2 router (or `router` when provided): WETH → tokenOut path.
	 */
	async getAmountsIn(amountOut: bigint, tokenOutForQuote: HexString, chain?: string, router?: HexString): Promise<bigint> {
		const chainId = chain ?? `EVM-${this.params.chainId}`
		const v2Router = router ?? this.configService.getUniswapRouterV2Address(chainId)
		const WETH = this.configService.getWrappedNativeAssetWithDecimals(chainId).asset
		const v2AmountIn = await this.publicClient.simulateContract({
			address: v2Router,
			abi: UniswapRouterV2.ABI,
			// @ts-ignore
			functionName: "getAmountsIn",
			// @ts-ignore
			args: [amountOut, [WETH, tokenOutForQuote]],
		})

		return v2AmountIn.result[0]
	}

	/**
	 * Given an input amount of native token, returns how much of the output token you get.
	 * Uses the chain's Uniswap V2 router: WETH → tokenOut path.
	 */
	async getAmountsOut(amountIn: bigint, tokenOutForQuote: HexString, chain?: string): Promise<bigint> {
		const chainId = chain ?? `EVM-${this.params.chainId}`
		const v2Router = this.configService.getUniswapRouterV2Address(chainId)
		const WETH = this.configService.getWrappedNativeAssetWithDecimals(chainId).asset
		const v2AmountOut = await this.publicClient.simulateContract({
			address: v2Router,
			abi: UniswapRouterV2.ABI,
			// @ts-ignore
			functionName: "getAmountsOut",
			// @ts-ignore
			args: [amountIn, [WETH, tokenOutForQuote]],
		})

		return v2AmountOut.result[1]
	}
	/**
	 * Estimates the gas required for a post request execution on this chain.
	 * This function generates mock proofs for the post request, creates a state override
	 * with the necessary overlay root, and estimates the gas cost for executing the
	 * handlePostRequests transaction on the handler contract.
	 *
	 * @param request - The post request to estimate gas for
	 * @param paraId - The ID of the parachain (Hyperbridge) that will process the request
	 * @returns The estimated gas amount in gas units and the generated calldata
	 */
	async estimateGas(request: IPostRequest): Promise<{ gas: bigint; postRequestCalldata: HexString }> {
		const hostParams = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "hostParams",
		})

		const { root, proof, index, treeSize } = await generateRootWithProof(request, 2n ** 10n)
		const latestStateMachineHeight = 6291991n
		const paraId = 4009n
		const overlayRootSlot = getStateCommitmentFieldSlot(
			paraId, // Hyperbridge chain id
			latestStateMachineHeight, // Hyperbridge chain height
			1, // For overlayRoot
		)
		const postParams = {
			height: {
				stateMachineId: BigInt(paraId),
				height: latestStateMachineHeight,
			},
			multiproof: proof,
			leafCount: treeSize,
		}

		const formattedRequest = {
			...request,
			source: toHex(request.source),
			dest: toHex(request.dest),
		}

		const contractArgs = [
			this.params.host,
			{
				proof: postParams,
				requests: [
					{
						request: formattedRequest,
						index,
					},
				],
			},
		] as const

		const postRequestCalldata = encodeFunctionData({
			abi: HandlerV2.ABI,
			functionName: "handlePostRequests",
			args: contractArgs,
		})

		let gas = await this.publicClient.estimateContractGas({
			address: hostParams.handler,
			abi: HandlerV2.ABI,
			functionName: "handlePostRequests",
			args: contractArgs,
			stateOverride: [
				{
					address: this.params.host,
					stateDiff: [
						{
							slot: overlayRootSlot,
							value: root,
						},
					],
				},
			],
		})

		// Add the cost of consensus verification (~600k gas)
		gas += 600_000n

		return { gas, postRequestCalldata }
	}

	/**
	 * Gets the fee token address and decimals for the chain.
	 * This function gets the fee token address and decimals for the chain.
	 *
	 * @returns The fee token address and decimals
	 */
	async getFeeTokenWithDecimals(): Promise<{ address: HexString; decimals: number }> {
		const hostParams = await this.publicClient.readContract({
			abi: EvmHost.ABI,
			address: this.params.host,
			functionName: "hostParams",
		})
		const feeTokenAddress = hostParams.feeToken
		const feeTokenDecimals = await this.publicClient.readContract({
			address: feeTokenAddress,
			abi: erc20Abi,
			functionName: "decimals",
		})
		return { address: feeTokenAddress, decimals: feeTokenDecimals }
	}

	/**
	 * Gets the nonce of the host.
	 * This function gets the nonce of the host.
	 *
	 * @returns The nonce of the host
	 */
	async getHostNonce(): Promise<bigint> {
		const nonce = await this.publicClient.readContract({
			abi: evmHost.ABI,
			address: this.params.host,
			functionName: "nonce",
		})

		return nonce
	}

	async broadcastTransaction(signedTransaction: HexString): Promise<TransactionReceipt> {
		const txHash = await this.client.sendRawTransaction({
			serializedTransaction: signedTransaction,
		})
		const receipt = await this.client.waitForTransactionReceipt({
			hash: txHash,
			confirmations: 1,
		})

		if (!receipt) {
			throw new Error("Transaction receipt not found")
		}
		return receipt
	}

	async getTransactionReceipt(hash: HexString): Promise<TransactionReceipt> {
		const receipt = await this.client.waitForTransactionReceipt({
			hash,
			confirmations: 1,
		})

		if (!receipt) {
			throw new Error("Transaction receipt not found")
		}
		return receipt
	}
}

/**
 * Factory function for creating EVM chain instances with common defaults
 *
 * @param chainId - The EVM chain ID
 * @param host - The IsmpHost contract address
 * @param options - Optional configuration overrides
 * @returns A new EvmChain instance
 *
 * @example
 * ```typescript
 * // Create with minimal config
 * const ethChain = createEvmChain(1, "0x87ea45..", {
 *   rpcUrl: "https://eth-mainnet.g.alchemy.com/v2/YOUR_KEY"
 * })
 *
 * // Create with custom consensus state ID
 * const arbChain = createEvmChain(42161, "0x87ea42345..", {
 *   rpcUrl: "https://arb-mainnet.g.alchemy.com/v2/YOUR_KEY",
 *   consensusStateId: "ARB_CUSTOM"
 * })
 * ```
 */
export function createEvmChain(
	chainId: number,
	host: HexString,
	options: {
		rpcUrl: string
		consensusStateId?: string
	},
): EvmChain {
	return EvmChain.fromParams({
		chainId,
		host,
		rpcUrl: options.rpcUrl,
		consensusStateId: options.consensusStateId,
	})
}

/**
 * Slot for storing request commitments.
 */
export const REQUEST_COMMITMENTS_SLOT = 0n

/**
 * Slot index for response commitments map
 */
export const RESPONSE_COMMITMENTS_SLOT = 1n

/**
 * Slot index for requests receipts map
 */
export const REQUEST_RECEIPTS_SLOT = 2n

/**
 * Slot index for response receipts map
 */
export const RESPONSE_RECEIPTS_SLOT = 3n

/**
 * Slot index for state commitment map
 */
export const STATE_COMMITMENTS_SLOT = 5n

export function requestCommitmentKey(key: Hex): { slot1: Hex; slot2: Hex } {
	// First derive the map key
	const keyBytes = hexToBytes(key)
	const slot = REQUEST_COMMITMENTS_SLOT
	const mappedKey = deriveMapKey(keyBytes, slot)

	// Convert the derived key to BigInt and add 1
	const number = bytesToBigInt(hexToBytes(mappedKey)) + 1n

	// Convert back to 32-byte hex
	return {
		slot1: pad(`0x${number.toString(16)}`, { size: 32 }),
		slot2: mappedKey,
	}
}

export function responseCommitmentKey(key: Hex): Hex {
	// First derive the map key
	const keyBytes = hexToBytes(key)
	const slot = RESPONSE_COMMITMENTS_SLOT
	const mappedKey = deriveMapKey(keyBytes, slot)

	// Convert the derived key to BigInt and add 1
	const number = bytesToBigInt(hexToBytes(mappedKey)) + 1n

	// Convert back to 32-byte hex
	return pad(`0x${number.toString(16)}`, { size: 32 })
}

function deriveMapKey(key: Uint8Array, slot: bigint): Hex {
	// Convert slot to 32-byte big-endian representation
	const slotBytes = pad(`0x${slot.toString(16)}`, { size: 32 })

	// Combine key and slot bytes
	const combined = new Uint8Array([...key, ...toBytes(slotBytes)])

	// Calculate keccak256 hash
	return keccak256(combined)
}

/**
 * Derives the storage slot for a specific field in the StateCommitment struct
 *
 * struct StateCommitment {
 *   uint256 timestamp;     // slot + 0
 *   bytes32 overlayRoot;   // slot + 1
 *   bytes32 stateRoot;     // slot + 2
 * }
 *
 * @param stateMachineId - The state machine ID
 * @param height - The block height
 * @param field - The field index in the struct (0 for timestamp, 1 for overlayRoot, 2 for stateRoot)
 * @returns The storage slot for the specific field
 */
export function getStateCommitmentFieldSlot(stateMachineId: bigint, height: bigint, field: 0 | 1 | 2): HexString {
	const baseSlot = getStateCommitmentSlot(stateMachineId, height)
	const slotNumber = bytesToBigInt(toBytes(baseSlot)) + BigInt(field)
	return pad(`0x${slotNumber.toString(16)}`, { size: 32 })
}

export function getStateCommitmentSlot(stateMachineId: bigint, height: bigint): HexString {
	// First level mapping: keccak256(stateMachineId . STATE_COMMITMENTS_SLOT)
	const firstLevelSlot = deriveFirstLevelSlot(stateMachineId, STATE_COMMITMENTS_SLOT)

	// Second level mapping: keccak256(height . firstLevelSlot)
	return deriveSecondLevelSlot(height, firstLevelSlot)
}

function deriveFirstLevelSlot(key: bigint, slot: bigint): HexString {
	const keyHex = pad(`0x${key.toString(16)}`, { size: 32 })
	const keyBytes = toBytes(keyHex)

	const slotBytes = toBytes(pad(`0x${slot.toString(16)}`, { size: 32 }))

	const combined = new Uint8Array([...keyBytes, ...slotBytes])

	return keccak256(combined)
}

function deriveSecondLevelSlot(key: bigint, firstLevelSlot: HexString): HexString {
	const keyHex = pad(`0x${key.toString(16)}`, { size: 32 })
	const keyBytes = toBytes(keyHex)

	const slotBytes = toBytes(firstLevelSlot)

	const combined = new Uint8Array([...keyBytes, ...slotBytes])

	return keccak256(combined)
}
