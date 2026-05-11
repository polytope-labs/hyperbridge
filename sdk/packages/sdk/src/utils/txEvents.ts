import IntentGatewayABI from "@/abis/IntentGateway"
import EVM_HOST from "@/abis/evmHost"
import type { DecodedOrderPlacedLog, DecodedPostRequestEvent, DecodedPostResponseEvent, HexString } from "@/types"
import { parseEventLogs, type Hex } from "viem"

/**
 * Extracts the IntentGateway OrderPlaced event from a transaction hash.
 * @param client - A viem PublicClient-compatible instance
 * @param txHash - Transaction hash
 * @returns Decoded OrderPlaced event or undefined if not found
 */
export async function getOrderPlacedFromTx(
	client: { getTransactionReceipt: (args: { hash: Hex }) => Promise<{ logs: any[] }> },
	txHash: HexString,
): Promise<DecodedOrderPlacedLog | undefined> {
	const receipt = await client.getTransactionReceipt({ hash: txHash as Hex })
	const events = parseEventLogs({
		abi: IntentGatewayABI.ABI,
		logs: receipt.logs,
	}) as unknown as DecodedOrderPlacedLog[]
	return events.find((e) => e.eventName === "OrderPlaced")
}

/**
 * Extracts the EvmHost PostRequestEvent from a transaction hash.
 * @param client - A viem PublicClient-compatible instance
 * @param txHash - Transaction hash
 * @returns Decoded PostRequestEvent or undefined if not found
 */
export async function getPostRequestEventFromTx(
	client: { getTransactionReceipt: (args: { hash: Hex }) => Promise<{ logs: any[] }> },
	txHash: HexString,
): Promise<DecodedPostRequestEvent | undefined> {
	const receipt = await client.getTransactionReceipt({ hash: txHash as Hex })
	const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs }) as unknown as DecodedPostRequestEvent[]
	return events.find((e) => e.eventName === "PostRequestEvent")
}

/**
 * Extracts the EvmHost PostResponseEvent from a transaction hash.
 * @param client - A viem PublicClient-compatible instance
 * @param txHash - Transaction hash
 * @returns Decoded PostResponseEvent or undefined if not found
 */
export async function getPostResponseEventFromTx(
	client: { getTransactionReceipt: (args: { hash: Hex }) => Promise<{ logs: any[] }> },
	txHash: HexString,
): Promise<DecodedPostResponseEvent | undefined> {
	const receipt = await client.getTransactionReceipt({ hash: txHash as Hex })
	const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs }) as unknown as DecodedPostResponseEvent[]
	return events.find((e) => e.eventName === "PostResponseEvent")
}
