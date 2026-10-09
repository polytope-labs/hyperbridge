import { hexToBigInt, maxUint256, pad, toHex, type PublicClient } from "viem"
import { ABI as IntentGatewayV2ABI } from "@/abis/IntentGatewayV2"
import type { HexString } from "@/types"
import { LEGACY_SELECT_SOLVER_TYPEHASH, SELECT_SOLVER_TYPEHASH } from "./CryptoUtils"

/**
 * What the order's session key signs to select a solver: the bid's userOpHash
 * (`SelectSolver(bytes32 commitment,bytes32 userOpHash)`) or the solver account's
 * address (`SelectSolver(bytes32 commitment,address solver)`).
 */
export type SelectionFormat = "userOpHash" | "address"

type ReadClient = Pick<PublicClient, "readContract">

const formats = new Map<string, Promise<SelectionFormat>>()

/**
 * The selection format an IntentGatewayV2 verifies, from its `SELECT_SOLVER_TYPEHASH()`. Read once
 * per chain and gateway and kept until {@link refreshSelectionFormat} drops it; a failed read is
 * not cached.
 *
 * @param client - Public client of the chain the gateway is deployed on.
 * @param chainId - Chain id of that chain.
 * @param gateway - IntentGatewayV2 address.
 * @throws If the read fails or the gateway reports a typehash of neither format.
 */
export function readSelectionFormat(
	client: ReadClient,
	chainId: bigint | number,
	gateway: HexString,
): Promise<SelectionFormat> {
	const key = `${chainId}:${gateway.toLowerCase()}`
	const cached = formats.get(key)
	if (cached) return cached

	const format = client
		.readContract({ address: gateway, abi: IntentGatewayV2ABI, functionName: "SELECT_SOLVER_TYPEHASH" })
		.then((typehash) => {
			switch (typehash.toLowerCase()) {
				case SELECT_SOLVER_TYPEHASH:
					return "userOpHash" as const
				case LEGACY_SELECT_SOLVER_TYPEHASH:
					return "address" as const
				default:
					throw new Error(
						`IntentGatewayV2 ${gateway} on chain ${chainId} has unknown SELECT_SOLVER_TYPEHASH ${typehash}`,
					)
			}
		})
	formats.set(key, format)
	format.catch(() => {
		if (formats.get(key) === format) formats.delete(key)
	})
	return format
}

/**
 * Drops `stale`, a selection format {@link readSelectionFormat} returned, and reads the format
 * again. A gateway upgraded in place can change its format, so a bid that does not pair with the
 * cached one is checked against a fresh read. When another caller has already replaced `stale`,
 * its read is reused.
 *
 * @param client - Public client of the chain the gateway is deployed on.
 * @param chainId - Chain id of that chain.
 * @param gateway - IntentGatewayV2 address.
 * @param stale - The cached read being replaced.
 * @throws If the read fails or the gateway reports a typehash of neither format.
 */
export function refreshSelectionFormat(
	client: ReadClient,
	chainId: bigint | number,
	gateway: HexString,
	stale: Promise<SelectionFormat>,
): Promise<SelectionFormat> {
	const key = `${chainId}:${gateway.toLowerCase()}`
	if (formats.get(key) === stale) formats.delete(key)
	return readSelectionFormat(client, chainId, gateway)
}

/** The IntentGatewayV2 params slot that packs the call dispatcher with `solverSelection` in the byte above it. */
const SELECTION_PARAMS_SLOT = pad(toHex(5n), { size: 32 }) as HexString

const SOLVER_SELECTION_CLEARED = maxUint256 ^ (0xffn << 160n)

/**
 * The storage write that turns an IntentGatewayV2's solver selection off: its params slot as the
 * gateway holds it, with only the `solverSelection` byte cleared. A fill simulated under it skips
 * the selection check, so it needs neither the order's session key nor a `select` call, and runs
 * outside a bundle, where a gateway that selects by userOpHash has no operation to check.
 *
 * @param client - Public client of the gateway's chain.
 * @param gateway - IntentGatewayV2 address.
 * @throws If the slot cannot be read.
 */
export async function selectionOffStateDiff(
	client: Pick<PublicClient, "getStorageAt">,
	gateway: HexString,
): Promise<{ slot: HexString; value: HexString }> {
	const word = await client.getStorageAt({ address: gateway, slot: SELECTION_PARAMS_SLOT })
	return {
		slot: SELECTION_PARAMS_SLOT,
		value: toHex(hexToBigInt(word ?? "0x0") & SOLVER_SELECTION_CLEARED, { size: 32 }),
	}
}
