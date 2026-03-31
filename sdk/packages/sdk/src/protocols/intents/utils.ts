import { encodeFunctionData, encodeAbiParameters, formatUnits, parseUnits, keccak256 } from "viem"
import { toHex } from "viem"
import IntentGatewayV2 from "@/abis/IntentGatewayV2"
import Decimal from "decimal.js"
import type { Order } from "@/types"
import type { ERC7821Call } from "@/types"
import type { HexString } from "@/types"
import { retryPromise, fetchPrice, bytes20ToBytes32 } from "@/utils"
import ERC7821ABI from "@/abis/erc7281"
import { ERC7821_BATCH_MODE } from "./types"
import type { IntentGatewayContext } from "./types"
import { requestCommitmentKey } from "@/chain"
import type { IEvmChain } from "@/chain"
import type { IProof } from "@/chain"

/** Cache TTL for fee-token entries, in milliseconds (5 minutes). */
const FEE_TOKEN_CACHE_TTL_MS = 5 * 60 * 1000

/**
 * Returns the fee token (address and decimals) for a given chain, using a
 * timed cache to avoid redundant on-chain calls.
 *
 * Re-fetches from the chain whenever the cached entry is missing or older
 * than {@link FEE_TOKEN_CACHE_TTL_MS} (5 minutes).
 *
 * @param ctx - The shared IntentsV2 context containing the fee-token cache.
 * @param chainId - State-machine ID of the chain whose fee token is needed.
 * @param chain - EVM chain client used to fetch a fresh fee token if the cache misses.
 * @returns Resolves with the fee token address and its ERC-20 decimal count.
 */
export async function getFeeToken(
	ctx: IntentGatewayContext,
	chainId: string,
	chain: IEvmChain,
): Promise<{ address: HexString; decimals: number }> {
	const cached = ctx.feeTokenCache.get(chainId)
	if (cached && Date.now() - cached.cachedAt < FEE_TOKEN_CACHE_TTL_MS) {
		return cached
	}

	const fresh = await chain.getFeeTokenWithDecimals()
	ctx.feeTokenCache.set(chainId, { ...fresh, cachedAt: Date.now() })
	return fresh
}

/**
 * Encodes a list of calls into ERC-7821 `execute` function calldata using
 * single-batch mode.
 *
 * This is a standalone utility that can be used outside of the
 * `IntentGatewayV2` class — for example, by filler strategies that need to
 * build custom batch calldata for combined swap-and-fill operations before
 * submitting a UserOperation.
 *
 * @param calls - Ordered list of calls to batch; each specifies a target
 *   address, ETH value, and calldata.
 * @returns ABI-encoded calldata for the ERC-7821 `execute(bytes32,bytes)` function.
 */
export function encodeERC7821ExecuteBatch(calls: ERC7821Call[]): HexString {
	const executionData = encodeAbiParameters(
		[{ type: "tuple[]", components: ERC7821ABI.ABI[1].components }],
		[calls.map((call) => ({ target: call.target, value: call.value, data: call.data }))],
	) as HexString

	return encodeFunctionData({
		abi: ERC7821ABI.ABI,
		functionName: "execute",
		args: [ERC7821_BATCH_MODE, executionData],
	}) as HexString
}

/**
 * Fetches a Merkle/state proof for the given ISMP request commitment on the
 * source chain.
 *
 * Derives the two storage slots from the commitment using
 * `requestCommitmentKey`, then queries the source chain node for a state
 * proof at the given block height.
 *
 * @param commitment - The ISMP request commitment hash to prove.
 * @param source - Source chain client used to query the state proof.
 * @param sourceStateMachine - State-machine ID string of the source chain.
 * @param sourceConsensusStateId - Consensus-state identifier for the source chain.
 * @param sourceHeight - Block height at which to generate the proof.
 * @returns Resolves with an {@link IProof} ready to be submitted to Hyperbridge.
 * @internal
 */
export async function fetchSourceProof(
	commitment: HexString,
	source: IEvmChain,
	sourceStateMachine: string,
	sourceConsensusStateId: string,
	sourceHeight: bigint,
): Promise<IProof> {
	const { slot1, slot2 } = requestCommitmentKey(commitment)
	const proofHex = await source.queryStateProof(sourceHeight, [slot1, slot2])

	return {
		height: sourceHeight,
		stateMachine: sourceStateMachine,
		consensusStateId: sourceConsensusStateId,
		proof: proofHex,
	}
}

/**
 * Strips SDK-only fields from an {@link Order} and normalises all fields to
 * the encoding the IntentGatewayV2 contract ABI expects:
 *
 * - `id` is removed (not part of the on-chain struct).
 * - `source` and `destination` are hex-encoded if currently plain string
 *   state-machine IDs.
 * - `inputs[i].token`, `output.beneficiary`, `output.assets[i].token`, and
 *   `predispatch.assets[i].token` are left-padded from 20-byte addresses to
 *   32-byte values (`0x000…addr`) via {@link bytes20ToBytes32}, matching the
 *   `bytes32(uint256(uint160(addr)))` encoding the contract uses when casting
 *   these fields back to `address`. Values already at 32 bytes are unchanged.
 *
 * @param order - The SDK-level order to transform.
 * @returns A contract-compatible order struct without `id`.
 */
export function transformOrderForContract(order: Order): Omit<Order, "id"> {
	const { id: _id, ...contractOrder } = order
	return {
		...contractOrder,
		source: order.source.startsWith("0x") ? order.source : toHex(order.source),
		destination: order.destination.startsWith("0x") ? order.destination : toHex(order.destination),
		inputs: order.inputs.map((t) => ({ ...t, token: bytes20ToBytes32(t.token) })),
		predispatch: {
			...order.predispatch,
			assets: order.predispatch.assets.map((t) => ({ ...t, token: bytes20ToBytes32(t.token) })),
		},
		output: {
			...order.output,
			beneficiary: bytes20ToBytes32(order.output.beneficiary),
			assets: order.output.assets.map((t) => ({ ...t, token: bytes20ToBytes32(t.token) })),
		},
	}
}

/**
 * Calculates the commitment hash for an order by ABI-encoding the
 * contract-normalised form of the order and hashing it.
 *
 * Calls `transformOrderForContract` before encoding to ensure all address
 * fields are padded to 32 bytes and SDK-only fields are stripped.
 *
 * @param order - The SDK-level order to hash.
 * @returns The keccak256 commitment hash.
 */
export function orderCommitment(order: Order): HexString {
	const placeOrderAbi = IntentGatewayV2.ABI.find(
		(item) => item.type === "function" && "name" in item && item.name === "placeOrder",
	)
	const orderType = placeOrderAbi?.inputs?.[0]
	if (!orderType) throw new Error("Could not find Order type in ABI")

	const encoded = encodeAbiParameters([orderType], [transformOrderForContract(order) as any])
	return keccak256(encoded)
}

/**
 * Converts a gas estimate (in gas units) on a given chain into the
 * equivalent amount of that chain's fee token (e.g. USDC).
 *
 * First attempts to price the gas cost in fee-token units using a Uniswap V2
 * on-chain quote (WETH → fee token). If that quote returns zero or fails,
 * falls back to a price-oracle estimate using the native currency's USD price
 * and assumes the fee token is worth $1.
 *
 * @param ctx - Shared IntentsV2 context.
 * @param gasEstimate - Gas quantity to convert (in gas units, not wei).
 * @param gasEstimateIn - Which chain side the gas estimate belongs to (`"source"` or `"dest"`).
 * @param evmChainID - State-machine ID of the chain on which gas is consumed.
 * @param gasPriceOverride - Optional gas price in wei; fetched on-chain if omitted.
 * @returns Resolves with the fee-token-denominated cost as a bigint, scaled to
 *   the fee token's decimal precision.
 */
export async function convertGasToFeeToken(
	ctx: IntentGatewayContext,
	gasEstimate: bigint,
	gasEstimateIn: "source" | "dest",
	evmChainID: string,
	gasPriceOverride?: bigint,
): Promise<bigint> {
	const chain = ctx[gasEstimateIn]
	const client = chain.client
	const gasPrice =
		gasPriceOverride ??
		((await retryPromise(() => client.getGasPrice(), {
			maxRetries: 3,
			backoffMs: 250,
		})) as bigint)
	const gasCostInWei = gasEstimate * gasPrice
	const wethAddr = chain.configService.getWrappedNativeAssetWithDecimals(evmChainID).asset
	const feeToken = await getFeeToken(ctx, evmChainID, chain)

	try {
		const { amountOut } = await ctx.swap.findBestProtocolWithAmountIn(
			client,
			wethAddr,
			feeToken.address,
			gasCostInWei,
			evmChainID,
			{ selectedProtocol: "v2" },
		)
		if (amountOut === 0n) {
			throw new Error()
		}
		return amountOut
	} catch {
		const nativeCurrency = client.chain?.nativeCurrency
		const chainId = Number.parseInt(evmChainID.split("-")[1])
		const gasCostInToken = new Decimal(formatUnits(gasCostInWei, nativeCurrency?.decimals ?? 18))
		const tokenPriceUsd = await fetchPrice(nativeCurrency?.symbol, chainId)
		const gasCostUsd = gasCostInToken.times(tokenPriceUsd)
		const feeTokenPriceUsd = new Decimal(1)
		const gasCostInFeeToken = gasCostUsd.dividedBy(feeTokenPriceUsd)
		return parseUnits(gasCostInFeeToken.toFixed(feeToken.decimals), feeToken.decimals)
	}
}

/**
 * Converts a fee-token amount on a given chain into the equivalent native wei amount.
 *
 * First attempts a Uniswap V2 quote (fee token -> WETH). If that quote fails or
 * returns zero, falls back to a price-oracle estimate assuming the fee token is $1.
 *
 * @param ctx - Shared IntentsV2 context.
 * @param feeTokenAmount - Amount in fee token units (scaled by fee token decimals).
 * @param feeTokenIn - Which chain side the fee-token amount belongs to (`"source"` or `"dest"`).
 * @param evmChainID - State-machine ID of the chain used for conversion.
 * @returns Native wei-equivalent amount.
 */
export async function convertFeeTokenToWei(
	ctx: IntentGatewayContext,
	feeTokenAmount: bigint,
	feeTokenIn: "source" | "dest",
	evmChainID: string,
): Promise<bigint> {
	const chain = ctx[feeTokenIn]
	const client = chain.client
	const wethAddr = chain.configService.getWrappedNativeAssetWithDecimals(evmChainID).asset
	const feeToken = await getFeeToken(ctx, evmChainID, chain)

	try {
		const { amountOut } = await ctx.swap.findBestProtocolWithAmountIn(
			client,
			feeToken.address,
			wethAddr,
			feeTokenAmount,
			evmChainID,
			{ selectedProtocol: "v2" },
		)
		if (amountOut === 0n) {
			throw new Error()
		}
		return amountOut
	} catch {
		const nativeCurrency = client.chain?.nativeCurrency
		const chainId = Number.parseInt(evmChainID.split("-")[1])
		const feeTokenAmountInToken = new Decimal(formatUnits(feeTokenAmount, feeToken.decimals))
		const nativeTokenPriceUsd = await fetchPrice(nativeCurrency?.symbol, chainId)
		const feeTokenAmountUsd = feeTokenAmountInToken.times(new Decimal(1))
		const nativeAmount = feeTokenAmountUsd.dividedBy(nativeTokenPriceUsd)
		return parseUnits(nativeAmount.toFixed(nativeCurrency?.decimals ?? 18), nativeCurrency?.decimals ?? 18)
	}
}
