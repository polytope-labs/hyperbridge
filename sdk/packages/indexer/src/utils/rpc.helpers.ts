import fetch from "node-fetch"
import { u64 } from "scale-ts"
import { Hex, hexToBytes, keccak256, stringToBytes } from "viem"
import { Option as PolkadotOption } from "@polkadot/types"
import { Codec } from "@polkadot/types/types"
import { StorageData } from "@polkadot/types/interfaces"

import { safeArray } from "./data.helper"
import { ENV_CONFIG } from "@/constants"

/**
 * Replace Websocket with HTTP is a function that replaces a websocket URL with an HTTP URL.
 * @param url The URL to replace
 */
export function replaceWebsocketWithHttp(url: string): string {
	if (url.startsWith("ws://")) {
		return url.replace("ws://", "http://")
	} else if (url.startsWith("wss://")) {
		return url.replace("wss://", "https://")
	}
	return url
}

/**
 * Get Block Timestamp is a function that retrieves the timestamp of a block given its hash and chain.
 * @param blockHash
 */
export async function getBlockTimestamp(blockhash: string, chain: string): Promise<bigint> {
	if (chain.startsWith("EVM")) {
		return getEvmBlockTimestamp(blockhash, chain)
	}

	return getSubstrateBlockTimestamp(blockhash)
}

interface ETHGetBlockByHashResponse {
	jsonrpc: "2.0"
	id: 1
	error?: {
		message: string
	}
	result: {
		timestamp: bigint
		hash: `0x${string}`
	}
}

interface CallTracerCall {
	from: string
	to: string
	gas: string
	gasUsed: string
	input: string
	output?: string
	value?: string
	type: string
	calls?: CallTracerCall[]
}

interface DebugTraceTransactionResponse {
	jsonrpc: "2.0"
	id: 1
	error?: {
		message: string
	}
	result: CallTracerCall
}

/**
 * Get EVM Block Timestamp is a function that retrieves the timestamp of a block given its hash and chain.
 * @param blockHash The hash of the block
 * @param chain The chain identifier
 * @returns The timestamp as a bigint
 * @throws Error if the RPC call fails or returns an unexpected response
 */
export async function getEvmBlockTimestamp(blockHash: string, chain: string): Promise<bigint> {
	const rpcUrl = replaceWebsocketWithHttp(ENV_CONFIG[chain] || "")
	if (!rpcUrl) {
		throw new Error(`No RPC URL found for chain: ${chain}`)
	}

	const getBlockByHash = await fetch(rpcUrl, {
		method: "POST",
		headers: { accept: "application/json", "content-type": "application/json" },
		body: JSON.stringify({
			id: 1,
			jsonrpc: "2.0",
			method: "eth_getBlockByHash",
			params: [blockHash, false],
		}),
	})

	const block: ETHGetBlockByHashResponse = await getBlockByHash.json()

	// Check for JSON-RPC errors
	if (block.error) {
		throw new Error(`RPC error: ${block.error.message || JSON.stringify(block.error)}`)
	}

	// Validate the response contains a result with a timestamp
	if (!block.result || block.result.timestamp === undefined) {
		throw new Error(`Unexpected response: No timestamp found in response ${JSON.stringify(block)}`)
	}

	return BigInt(block.result.timestamp)
}

/**
 * Recursively searches through call tracer response to find a call matching the target contract address.
 * Only searches in nested calls, not the top-level call.
 * @param call The call object to search
 * @param targetContractAddress The target contract address to find
 * @returns The input (calldata) if found, null otherwise
 */
function findCallInputByAddress(call: CallTracerCall, targetContractAddress: string): string | null {
	const normalizedTarget = targetContractAddress.toLowerCase()

	// Recursively search nested calls only (skip the current call itself)
	if (call.calls && Array.isArray(call.calls)) {
		for (const nestedCall of call.calls) {
			// Check if this nested call matches the target
			if (nestedCall.to.toLowerCase() === normalizedTarget) {
				return nestedCall.input
			}
			// Recursively search deeper nested calls
			const result = findCallInputByAddress(nestedCall, targetContractAddress)
			if (result !== null) {
				return result
			}
		}
	}

	return null
}

/**
 * Get Contract Call Input is a function that retrieves the input (calldata) used to call a target contract
 * within a transaction by using debug_traceTransaction with callTracer.
 * Only searches in nested calls, not the direct transaction call. Returns null if the transaction
 * directly calls the target contract or if the target is not found in nested calls.
 * @param txHash The transaction hash
 * @param targetContractAddress The target contract address to find the call for
 * @param chain The chain identifier (e.g., "EVM-56", "EVM-1")
 * @returns The input (calldata) as a hex string, or null if the transaction directly calls the target or target not found in nested calls
 * @throws Error if the RPC call fails or returns an unexpected response
 */
export async function getContractCallInput(
	txHash: string,
	targetContractAddress: string,
	chain: string,
): Promise<Hex | null> {
	const rpcUrl = replaceWebsocketWithHttp(ENV_CONFIG[chain] || "")
	if (!rpcUrl) {
		throw new Error(`No RPC URL found for chain: ${chain}`)
	}

	const traceResponse = await fetch(rpcUrl, {
		method: "POST",
		headers: { accept: "application/json", "content-type": "application/json" },
		body: JSON.stringify({
			id: 1,
			jsonrpc: "2.0",
			method: "debug_traceTransaction",
			params: [
				txHash,
				{
					tracer: "callTracer",
					tracerConfig: {
						disableCode: true,
					},
				},
			],
		}),
	})

	const trace: DebugTraceTransactionResponse = await traceResponse.json()

	if (trace.error) {
		throw new Error(`RPC error: ${trace.error.message || JSON.stringify(trace.error)}`)
	}

	if (!trace.result) {
		throw new Error(`Unexpected response: No result found in response ${JSON.stringify(trace)}`)
	}

	const normalizedTarget = targetContractAddress.toLowerCase()

	// If the transaction directly calls the target contract, return null
	if (trace.result.to.toLowerCase() === normalizedTarget) {
		return null
	}

	// Search for the target contract in nested calls only
	const input = findCallInputByAddress(trace.result, targetContractAddress)

	return input ? (input as Hex) : null
}

/**
 * Get Substrate Block Timestamp is a function that retrieves the timestamp of a block given its hash and chain.
 * @param storageKey The storage key for the state item
 * @param blockHash The hash of the block
 * @param chain The chain identifier
 * @returns The timestamp as a bigint
 * @throws Error if the RPC call fails or returns an unexpected response
 */
export async function getSubstrateBlockTimestamp(blockHash: string): Promise<bigint> {
	const STORAGE_KEY = "0xf0c365c3cf59d671eb72da0e7a4113c49f1f0515f462cdcf84e0f1d6045dfcbb"

	try {
		const storageValue = await api.rpc.state.getStorage<PolkadotOption<Codec>>(STORAGE_KEY, blockHash)

		if (!storageValue.isSome) {
			throw new Error(`Unexpected response: No storage found in response ${JSON.stringify(storageValue)}`)
		}

		return u64.dec(hexToBytes(storageValue.value.toHex()))
	} catch (err) {
		throw new Error(`RPC error: ${(err as Error).message}`)
	}
}
