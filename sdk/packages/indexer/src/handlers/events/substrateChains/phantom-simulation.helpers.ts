import { decodeFunctionData, keccak256, concat, toHex } from "viem"
import { decodeERC7821ExecuteBatch, IntentGatewayV2 } from "@hyperbridge/sdk/intents-helpers"
import { TOKEN_SLOT_OVERRIDES } from "@/token-slot-overrides"

export type HexString = `0x${string}`

export const FILL_ORDER_ABI = IntentGatewayV2.ABI

// topic0 of OrderFilled(bytes32,address,TokenInfo[],TokenInfo[]); its presence in the simulated call
// logs is what tells us the fill actually went through rather than just not reverting. 
export const ORDER_FILLED_TOPIC = keccak256(
	toHex("OrderFilled(bytes32,address,(bytes32,uint256)[],(bytes32,uint256)[])"),
).toLowerCase()

// A deadline far beyond any real chain head so the simulated order clears the gateway's
// `deadline < block.number` expiry check. The on-chain phantom order keeps its own expired deadline.
export const SIM_DEADLINE = 1n << 48n

export interface FillData {
	order: Record<string, unknown>
	options: Record<string, unknown>
	outputToken: HexString
	solverAmount: bigint
}

export function tokenSlots(address: string): { balanceSlot: bigint; allowanceSlot: bigint } {
	return TOKEN_SLOT_OVERRIDES[address.toLowerCase()] ?? { balanceSlot: 0n, allowanceSlot: 1n }
}

// Whether a token has a configured slot override. Tokens without one fall back to the OZ default
// (0/1), which is wrong for most real tokens, so the caller warns when this returns false.
export function hasTokenSlotOverride(address: string): boolean {
	return address.toLowerCase() in TOKEN_SLOT_OVERRIDES
}

// _orders is mapping(bytes32 => mapping(address => uint256)) at slot 9 in the IntentGateway.
// (PR #988 removed the _admin slot, shifting _orders down from slot 10 to slot 9.)
// The inner mapping is keyed by `address`, so the key must be the token left-padded to 32 bytes
// (abi.encode(address)). `inputToken` may be passed either as a 20-byte address or a 32-byte
// token field; normalise both to the address-as-uint256 form before hashing.
export function ordersStorageSlot(commitment: HexString, inputToken: HexString): HexString {
	const tokenKey = toHex(BigInt(inputToken), { size: 32 })
	const innerSlot = keccak256(concat([commitment, toHex(9n, { size: 32 })]))
	return keccak256(concat([tokenKey, innerSlot]))
}

export function erc20BalanceSlot(holder: HexString, slot: bigint): HexString {
	return keccak256(concat([toHex(BigInt(holder), { size: 32 }), toHex(slot, { size: 32 })]))
}

// _allowances[owner][spender]: inner slot keys on owner, outer on spender.
export function erc20AllowanceSlot(owner: HexString, spender: HexString, slot: bigint): HexString {
	const innerSlot = keccak256(concat([toHex(BigInt(owner), { size: 32 }), toHex(slot, { size: 32 })]))
	return keccak256(concat([toHex(BigInt(spender), { size: 32 }), innerSlot]))
}

// Pulls the inner fillOrder call out of the bid's ERC-7821 execute batch and decodes the order,
// the offered output token, and the solver's quoted amount. Returns null when no matching call
// targets the gateway or the calldata cannot be decoded.
export function extractFillData(callData: HexString, gatewayAddress: string): FillData | null {
	const calls = decodeERC7821ExecuteBatch(callData)
	if (!calls) return null

	const normalized = gatewayAddress.toLowerCase()
	for (const call of calls) {
		if (call.target.toLowerCase() !== normalized) continue
		try {
			const decoded = decodeFunctionData({ abi: FILL_ORDER_ABI, data: call.data as HexString })
			if (decoded.functionName !== "fillOrder" || !decoded.args || decoded.args.length < 2) continue
			const order = decoded.args[0] as Record<string, unknown>
			const options = decoded.args[1] as Record<string, unknown>
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const outputToken = (order as any)?.output?.assets?.[0]?.token as HexString | undefined
			// eslint-disable-next-line @typescript-eslint/no-explicit-any
			const outputs = (options as any)?.outputs as { amount: bigint }[] | undefined
			if (!outputToken || !outputs?.length) continue
			return { order, options, outputToken, solverAmount: outputs[0].amount }
		} catch {
			continue
		}
	}
	return null
}

// Rebuilds the bid's order for simulation. Matching source to destination routes the gateway
// through _fillSameChain (no ISMP dispatch), the future deadline clears the expiry check, and
// pointing the single output at the solver for solverAmount makes _fillSameChain run
// safeTransferFrom(solver -> solver) and read the injected escrow, which is the liquidity we want
// to validate. The phantom order's output amount is zero, so without this the fill is a no-op.
// Session is left as decoded (already the zero address); solver selection is disabled via a
// storage override on the gateway, so the session value is irrelevant here.
// Liquidity-weighted median of solver quotes. Each quote's influence is proportional to `weight`
// — the solver's total balance for the output token across native + vault venues — so a solver
// that can actually deliver size moves the price more than one quoting on thin liquidity.
// Returns the lower weighted median: the smallest price whose cumulative weight reaches half of
// the total. Zero-weight quotes contribute nothing; if every weight is zero (no measurable
// liquidity) it falls back to the unweighted median so a price is still reported.
export function weightedMedian(entries: { price: bigint; weight: bigint }[]): bigint {
	const sorted = [...entries].sort((a, b) => (a.price < b.price ? -1 : a.price > b.price ? 1 : 0))
	const totalWeight = sorted.reduce((acc, e) => (e.weight > 0n ? acc + e.weight : acc), 0n)

	if (totalWeight === 0n) {
		return sorted[Math.floor(sorted.length / 2)].price
	}

	let cumulative = 0n
	for (const entry of sorted) {
		if (entry.weight <= 0n) continue
		cumulative += entry.weight
		if (cumulative * 2n >= totalWeight) return entry.price
	}
	return sorted[sorted.length - 1].price
}

// Rebuilds the bid's order for simulation. Matching source to destination routes the gateway
// through _fillSameChain (no ISMP dispatch), the future deadline clears the expiry check, and
// pointing the single output at the solver for solverAmount makes _fillSameChain run
// safeTransferFrom(solver -> solver) and read the injected escrow, which is the liquidity we want
// to validate. The phantom order's output amount is zero, so without this the fill is a no-op.
// Session is left as decoded (already the zero address); solver selection is disabled via a
// storage override on the gateway, so the session value is irrelevant here.
export function buildSimulationOrder(
	order: Record<string, unknown>,
	solver: string,
	solverAmount: bigint,
): Record<string, unknown> {
	const outputInfo = order.output as {
		beneficiary: HexString
		assets: { token: HexString; amount: bigint }[]
		call: HexString
	}
	return {
		...order,
		source: order.destination,
		deadline: SIM_DEADLINE,
		output: {
			...outputInfo,
			beneficiary: toHex(BigInt(solver), { size: 32 }),
			assets: outputInfo.assets.map((asset, i) => ({
				...asset,
				amount: i === 0 ? solverAmount : asset.amount,
			})),
		},
	}
}
