import type { ChainConfigService } from "@/configs/ChainConfigService"
import { Chains, type ConfiguredAssetSymbol } from "@/configs/chain"
import { _queryLatestPhantomOrderPriceSnapshot } from "@/queryClient"
import type { HexString, IndexerQueryClient, PhantomOrderPriceSnapshot } from "@/types"
import {
	InvalidPhantomSnapshotError,
	PhantomSnapshotUnavailableError,
	type IntentQuoteChainContext,
	type IntentQuoteStrategyHandler,
	type PhantomSnapshotQuoteIntentResult,
	type QuoteIntentParams,
	type QuoteIntentResult,
	UnsupportedIntentQuotePairError,
} from "./types"
import { deductProtocolFee, divCeil, grossUpForProtocolFee, readProtocolFeeBps, validateQuoteParams } from "./shared"

export const PHANTOM_INTENT_QUOTE_CHAIN = Chains.BASE_MAINNET

interface ResolvedPhantomPair {
	tokenA: HexString
	tokenB: HexString
}

interface ResolvedBaseSnapshotToken {
	symbol: ConfiguredAssetSymbol
	address: HexString
}

/**
 * Maps order-side configured assets to the canonical Base asset used by the
 * Phantom snapshot feed. Add new source assets here as snapshot coverage grows.
 */
const BASE_SNAPSHOT_ASSET_BY_ORDER_ASSET: Partial<Record<ConfiguredAssetSymbol, ConfiguredAssetSymbol>> = {
	USDC: "USDC",
	USDT: "USDC",
	cNGN: "cNGN",
}

/** Directional Base snapshot pairs currently indexed by Phantom. */
const SUPPORTED_BASE_SNAPSHOT_PAIRS = new Set(["USDC:cNGN", "cNGN:USDC"])

export class PhantomSnapshotIntentQuoteStrategy implements IntentQuoteStrategyHandler {
	constructor(
		private readonly chainConfigService: ChainConfigService,
		private readonly getQueryClient: () => IndexerQueryClient,
	) {}

	async quote(
		params: QuoteIntentParams,
		source: IntentQuoteChainContext,
		destination: IntentQuoteChainContext,
	): Promise<QuoteIntentResult> {
		validateQuoteParams(params)
		const pair = this.resolveBaseSnapshotPair(params, source.stateMachineId, destination.stateMachineId)
		if (!pair) {
			throw new UnsupportedIntentQuotePairError({
				source: source.stateMachineId,
				destination: destination.stateMachineId,
				tokenIn: params.tokenIn,
				tokenOut: params.tokenOut,
				quoteSource: "Phantom snapshot pair",
			})
		}

		const queryClient = this.getQueryClient()
		const [protocolFeeBps, snapshot] = await Promise.all([
			readProtocolFeeBps(this.chainConfigService, source),
			_queryLatestPhantomOrderPriceSnapshot({
				tokenA: pair.tokenA,
				tokenB: pair.tokenB,
				queryClient,
			}),
		])
		if (!snapshot) throw new PhantomSnapshotUnavailableError(pair.tokenA, pair.tokenB)
		this.validateSnapshot(snapshot)
		return this.quoteSnapshot(params, protocolFeeBps, snapshot)
	}

	private quoteSnapshot(
		params: QuoteIntentParams,
		protocolFeeBps: bigint,
		snapshot: PhantomOrderPriceSnapshot,
	): PhantomSnapshotQuoteIntentResult {
		if (params.amountIn !== undefined) {
			const netAmountIn = deductProtocolFee(params.amountIn, protocolFeeBps)
			const amountOut = (netAmountIn * snapshot.medianPrice) / snapshot.standardAmount
			if (amountOut <= 0n) {
				throw new InvalidPhantomSnapshotError(snapshot.commitment, "quote rounds down to zero output")
			}
			return this.buildResult("EXACT_INPUT", params.amountIn, amountOut, protocolFeeBps, snapshot)
		}

		if (params.amountOut === undefined) throw new Error("Quote amount is missing after validation")
		const netAmountIn = divCeil(params.amountOut * snapshot.standardAmount, snapshot.medianPrice)
		const amountIn = grossUpForProtocolFee(netAmountIn, protocolFeeBps)
		return this.buildResult("EXACT_OUTPUT", amountIn, params.amountOut, protocolFeeBps, snapshot)
	}

	private buildResult(
		tradeType: PhantomSnapshotQuoteIntentResult["tradeType"],
		amountIn: bigint,
		amountOut: bigint,
		protocolFeeBps: bigint,
		snapshot: PhantomOrderPriceSnapshot,
	): PhantomSnapshotQuoteIntentResult {
		return {
			strategy: "phantom_snapshot",
			tradeType,
			amountIn,
			amountOut,
			quoteMetadata: {
				quoteChain: PHANTOM_INTENT_QUOTE_CHAIN,
				commitment: snapshot.commitment,
				tokenA: snapshot.tokenA,
				tokenB: snapshot.tokenB,
				standardAmount: snapshot.standardAmount,
				medianPrice: snapshot.medianPrice,
				lowestPrice: snapshot.lowestPrice,
				highestPrice: snapshot.highestPrice,
				blockNumber: snapshot.blockNumber,
				snapshotTime: snapshot.snapshotTime,
				bidCount: snapshot.bidCount,
				protocolFeeBps,
			},
		}
	}

	private validateSnapshot(snapshot: PhantomOrderPriceSnapshot): void {
		if (snapshot.standardAmount <= 0n) {
			throw new InvalidPhantomSnapshotError(snapshot.commitment, "standardAmount must be greater than zero")
		}
		if (snapshot.medianPrice <= 0n) {
			throw new InvalidPhantomSnapshotError(snapshot.commitment, "medianPrice must be greater than zero")
		}
		if (snapshot.bidCount <= 0) {
			throw new InvalidPhantomSnapshotError(snapshot.commitment, "bidCount must be greater than zero")
		}
		if (Number.isNaN(snapshot.snapshotTime.getTime())) {
			throw new InvalidPhantomSnapshotError(snapshot.commitment, "snapshotTime is invalid")
		}
	}

	private resolveBaseSnapshotPair(
		params: Pick<QuoteIntentParams, "tokenIn" | "tokenOut">,
		sourceStateMachineId: string,
		destinationStateMachineId: string,
	): ResolvedPhantomPair | undefined {
		const tokenIn = this.resolveBaseSnapshotToken(sourceStateMachineId, params.tokenIn)
		const tokenOut = this.resolveBaseSnapshotToken(destinationStateMachineId, params.tokenOut)
		if (!tokenIn || !tokenOut || !isSupportedSnapshotPair(tokenIn.symbol, tokenOut.symbol)) return

		return { tokenA: tokenIn.address, tokenB: tokenOut.address }
	}

	private resolveBaseSnapshotToken(chain: string, tokenAddress: HexString): ResolvedBaseSnapshotToken | undefined {
		const orderAsset = this.chainConfigService.getAssetMetadataByAddress(chain, tokenAddress)?.symbol
		if (!orderAsset) return

		const snapshotAsset = BASE_SNAPSHOT_ASSET_BY_ORDER_ASSET[orderAsset]
		if (!snapshotAsset) return
		const address = this.chainConfigService.getAssetAddress(PHANTOM_INTENT_QUOTE_CHAIN, snapshotAsset)
		if (!isConfiguredAddress(address)) return

		return { symbol: snapshotAsset, address }
	}
}

function isSupportedSnapshotPair(tokenA: ConfiguredAssetSymbol, tokenB: ConfiguredAssetSymbol): boolean {
	return SUPPORTED_BASE_SNAPSHOT_PAIRS.has(`${tokenA}:${tokenB}`)
}

function isConfiguredAddress(address?: HexString): address is HexString {
	return Boolean(address && address !== "0x" && !/^0x0{40}$/i.test(address))
}
