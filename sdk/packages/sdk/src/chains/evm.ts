import {
	bytesToBigInt,
	bytesToHex,
	createPublicClient,
	encodeFunctionData,
	hexToBytes,
	http,
	type PublicClient,
	toHex,
	keccak256,
	toBytes,
	pad,
} from "viem"
import {
	mainnet,
	arbitrum,
	arbitrumSepolia,
	optimism,
	optimismSepolia,
	base,
	baseSepolia,
	soneium,
	bsc,
	bscTestnet,
	gnosis,
	gnosisChiado,
} from "viem/chains"

import type { GetProofParameters, Hex } from "viem"
import { zip, flatten } from "lodash-es"
import { match } from "ts-pattern"

import EvmHost from "@/abis/evmHost"
import type { IChain, IIsmpMessage } from "@/chain"
import HandlerV1 from "@/abis/handler"
import { calculateMMRSize, EvmStateProof, mmrPositionToKIndex, MmrProof, SubstrateStateProof } from "@/utils"
import type { HexString, IMessage } from "@/types"

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

	constructor(private readonly params: EvmChainParams) {
		// @ts-ignore
		this.publicClient = createPublicClient({
			// @ts-ignore
			chain: chains[params.chainId],
			transport: http(params.url),
		})
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
	 * @returns {Promise<HexString>} The encoded storage proof.
	 */
	async queryStateProof(at: bigint, keys: HexString[]): Promise<HexString> {
		const config: GetProofParameters = {
			address: this.params.host,
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
			.exhaustive()

		return encoded
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
