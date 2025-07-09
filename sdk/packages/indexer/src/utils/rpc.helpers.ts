import fetch from "node-fetch"
import { Struct, u64 } from "scale-ts"
import { hexToBytes } from "viem"
import { Option as PolkadotOption } from "@polkadot/types"
import { Codec } from "@polkadot/types/types"

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
