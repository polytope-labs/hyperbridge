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
} from "viem/chains"

import { flatten, zip } from "lodash-es"
import { match } from "ts-pattern"
import type { GetProofParameters, Hex } from "viem"

import EvmHost from "@/abis/evmHost"
import evmHost from "@/abis/evmHost"
import HandlerV1 from "@/abis/handler"
import type { IChain, IIsmpMessage } from "@/chain"
import { ChainConfigService } from "@/configs/ChainConfigService"
import type { HexString, IMessage, IPostRequest, StateMachineHeight, StateMachineIdParams } from "@/types"
import {
	ADDRESS_ZERO,
	EvmStateProof,
	MmrProof,
	SubstrateStateProof,
	calculateMMRSize,
	generateRootWithProof,
	mmrPositionToKIndex,
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
}

/**
 * The default address used as fallback when no address is provided.
 * This represents the zero address in EVM chains.
 */
export const DEFAULT_ADDRESS = "0x0000000000000000000000000000000000000000"

/**
 * Parameters for an EVM chain.
 */
export interface EvmChainParams {
	/**
	 * The chain ID of the EVM chain.
	 */
	chainId: number
	/**
	 * The host address of the EVM chain.
	 */
	host: HexString
	/**
	 * The URL of the EVM chain.
	 */
	url: string
}

/**
 * Encapsulates an EVM chain.
 */
export class EvmChain implements IChain {
	private publicClient: PublicClient
	private chainConfigService: ChainConfigService

	constructor(private readonly params: EvmChainParams) {
		// @ts-ignore
		this.publicClient = createPublicClient({
			// @ts-ignore
			chain: chains[params.chainId],
			transport: http(params.url),
		})
		this.chainConfigService = new ChainConfigService()
	}

	// Expose minimal getters for external helpers/classes
	get client(): PublicClient {
		return this.publicClient
	}

	get host(): HexString {
		return this.params.host
	}

	get config(): ChainConfigService {
		return this.chainConfigService
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
				? message.Requests.map((key) => requestCommitmentKey(key))
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
	 * @param {bigint} at - The block height at which to query the storage proof.
	 * @param {HexString[]} keys - The keys for which to query the storage proof.
	 * @param {HexString} address - Optional contract address to fetch storage proof else default to host contract
	 * @returns {Promise<HexString>} The encoded storage proof.
	 */
	async queryStateProof(at: bigint, keys: HexString[], address?: HexString): Promise<HexString> {
		const config: GetProofParameters = {
			address: address ?? this.params.host,
			storageKeys: keys,
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
						const [[, kIndex]] = mmrPositionToKIndex(
							[leafIndexAndPos?.pos],
							calculateMMRSize(mmrProof.leafCount),
						)
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
							kIndex,
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
					abi: HandlerV1.ABI,
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
				const proof = SubstrateStateProof.dec(timeout.proof.proof).value.storageProof.map((item) =>
					toHex(new Uint8Array(item)),
				)
				const encoded = encodeFunctionData({
					abi: HandlerV1.ABI,
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
						const [[, kIndex]] = mmrPositionToKIndex(
							[leafIndexAndPos?.pos],
							calculateMMRSize(mmrProof.leafCount),
						)
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
							kIndex,
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
					abi: HandlerV1.ABI,
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
			.with({ kind: "GetRequest" }, (message) => {
				throw new Error("GetResponse is not yet supported on Substrate chains")
			})
			.exhaustive()

		return encoded
	}

	/**
	 * Calculates the fee required to send a post request to the destination chain.
	 * The fee is calculated based on the per-byte fee for the destination chain
	 * multiplied by the size of the request body.
	 *
	 * @param request - The post request to calculate the fee for
	 * @returns The total fee in wei required to send the post request
	 */
	async quote(request: IPostRequest): Promise<bigint> {
		const perByteFee = await this.publicClient.readContract({
			address: this.params.host,
			abi: EvmHost.ABI,
			functionName: "perByteFee",
			args: [toHex(request.dest)],
		})

		// Exclude 0x prefix from the body length, and get the byte length
		const bodyByteLength = Math.floor((request.body.length - 2) / 2)
		const length = bodyByteLength < 32 ? 32 : bodyByteLength

		return perByteFee * BigInt(length)
	}

	async quoteNative(request: IPostRequest, fee: bigint): Promise<bigint> {
		const totalFee = (await this.quote(request)) + fee
		const feeToken = await this.getFeeTokenWithDecimals()
		return this.getAmountsIn(totalFee, feeToken.address, request.source)
	}

	private async getAmountsIn(amountOut: bigint, tokenOutForQuote: HexString, chain: string): Promise<bigint> {
		const v2Router = this.config.getUniswapRouterV2Address(chain)
		const WETH = this.config.getWrappedNativeAssetWithDecimals(chain).asset
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

		const { root, proof, index, kIndex, treeSize } = await generateRootWithProof(request, 2n ** 10n)
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
						kIndex,
					},
				],
			},
		] as const

		const postRequestCalldata = encodeFunctionData({
			abi: HandlerV1.ABI,
			functionName: "handlePostRequests",
			args: contractArgs,
		})

		const gas = await this.publicClient.estimateContractGas({
			address: hostParams.handler,
			abi: HandlerV1.ABI,
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

function requestCommitmentKey(key: Hex): Hex {
	// First derive the map key
	const keyBytes = hexToBytes(key)
	const slot = REQUEST_COMMITMENTS_SLOT
	const mappedKey = deriveMapKey(keyBytes, slot)

	// Convert the derived key to BigInt and add 1
	const number = bytesToBigInt(hexToBytes(mappedKey)) + 1n

	// Convert back to 32-byte hex
	return pad(`0x${number.toString(16)}`, { size: 32 })
}

function responseCommitmentKey(key: Hex): Hex {
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
