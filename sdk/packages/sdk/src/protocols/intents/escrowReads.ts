import { parseAbi, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { HexString } from "@/types"
import { bytes32ToBytes20 } from "@/utils"

/**
 * Getters of gateways that predate per-leg escrow, where `_orders` and `_partialFills` are keyed by
 * token. Those implementations reject an order that repeats an input or an output token, so a token
 * key there identifies exactly one leg.
 */
const TOKEN_KEYED_GATEWAY_ABI = parseAbi([
	"function _orders(bytes32 commitment, address token) view returns (uint256)",
	"function _partialFills(bytes32 commitment, bytes32 token) view returns (uint256)",
])

type Keying = "leg" | "token"

/** The keying each gateway last answered to, per client, so a token-keyed gateway costs one call. */
const keyingByClient = new WeakMap<object, Map<string, Keying>>()

type ReadClient = Pick<PublicClient, "readContract">

/**
 * Escrow held for leg `index` of an order on its source chain: `_orders(commitment, index)`, or
 * `_orders(commitment, token)` on a gateway that has not been upgraded to per-leg escrow.
 *
 * @param client - Public client of the order's source chain.
 * @param gateway - IntentGatewayV2 address on that chain.
 * @param commitment - The order commitment.
 * @param index - Leg index into `order.inputs`.
 * @param token - `order.inputs[index].token`, as bytes32 or a 20-byte address.
 */
export async function readLegEscrow(
	client: ReadClient,
	gateway: HexString,
	commitment: HexString,
	index: number,
	token: HexString,
): Promise<bigint> {
	return readKeyed(client, gateway, (keying) =>
		keying === "leg"
			? client.readContract({
					address: gateway,
					abi: IntentGatewayV2ABI,
					functionName: "_orders",
					args: [commitment, BigInt(index)],
				})
			: client.readContract({
					address: gateway,
					abi: TOKEN_KEYED_GATEWAY_ABI,
					functionName: "_orders",
					args: [commitment, bytes32ToBytes20(token)],
				}),
	)
}

/**
 * Output delivered so far for leg `index` of an order on its destination chain:
 * `_partialFills(commitment, index)`, or `_partialFills(commitment, token)` on a gateway that has not
 * been upgraded to per-leg escrow.
 *
 * @param client - Public client of the order's destination chain.
 * @param gateway - IntentGatewayV2 address on that chain.
 * @param commitment - The order commitment.
 * @param index - Leg index into `order.output.assets`.
 * @param token - `order.output.assets[index].token` as bytes32.
 */
export async function readLegPartialFill(
	client: ReadClient,
	gateway: HexString,
	commitment: HexString,
	index: number,
	token: HexString,
): Promise<bigint> {
	return readKeyed(client, gateway, (keying) =>
		keying === "leg"
			? client.readContract({
					address: gateway,
					abi: IntentGatewayV2ABI,
					functionName: "_partialFills",
					args: [commitment, BigInt(index)],
				})
			: client.readContract({
					address: gateway,
					abi: TOKEN_KEYED_GATEWAY_ABI,
					functionName: "_partialFills",
					args: [commitment, token],
				}),
	)
}

/**
 * Reads with the keying the gateway last answered to (per-leg first), switching to the other only when
 * the call reverts: the getter does not exist on that implementation. Any other error, e.g. a transport
 * failure, is thrown as is. A gateway that is upgraded later reverts on the remembered keying and is
 * switched back on that read.
 */
async function readKeyed(client: ReadClient, gateway: HexString, read: (keying: Keying) => Promise<bigint>) {
	let known = keyingByClient.get(client)
	if (!known) {
		known = new Map()
		keyingByClient.set(client, known)
	}
	const key = gateway.toLowerCase()
	const first = known.get(key) ?? "leg"

	try {
		const value = await read(first)
		known.set(key, first)
		return value
	} catch (error) {
		if (!isRevert(error)) throw error
		const second: Keying = first === "leg" ? "token" : "leg"
		try {
			const value = await read(second)
			known.set(key, second)
			return value
		} catch (secondError) {
			throw isRevert(secondError) ? error : secondError
		}
	}
}

/** Whether a viem contract call failed because the call reverted or returned nothing. */
function isRevert(error: unknown): boolean {
	let current = error
	while (current instanceof Error) {
		if (current.name === "ContractFunctionRevertedError" || current.name === "ContractFunctionZeroDataError") {
			return true
		}
		current = current.cause
	}
	return false
}
