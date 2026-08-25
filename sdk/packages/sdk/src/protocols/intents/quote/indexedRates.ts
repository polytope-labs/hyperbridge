import type { ChainConfigService } from "@/configs/ChainConfigService"
import { type ConfiguredAssetSymbol, getConfigByStateMachineId } from "@/configs/chain"
import {
	LiquidityEngine,
	UnsupportedLiquidityAssetError,
	UnsupportedLiquidityChainError,
} from "@/protocols/intents/LiquidityEngine"
import type { BuyAndSellRates, HexString, IndexerQueryClient } from "@/types"
import { parseUnits } from "viem"
import { deductProtocolFee, divCeil, grossUpForProtocolFee, readProtocolFeeBps, validateQuoteParams } from "./shared"
import {
	type IndexedRateQuoteIntentResult,
	type IndexedRateSide,
	IndexedRateUnavailableError,
	type IntentQuoteChainContext,
	type IntentQuoteStrategyHandler,
	InvalidIndexedRateError,
	type QuoteIntentParams,
} from "./types"

const INDEXED_RATE_DECIMALS = 18
const INDEXED_RATE_SCALE = 10n ** BigInt(INDEXED_RATE_DECIMALS)

interface ResolvedQuoteAsset {
	symbol: ConfiguredAssetSymbol
	address: HexString
	decimals: number
}

interface SelectedIndexedRate {
	side: IndexedRateSide
	rate: string
	scaledRate: bigint
	updatedAt: Date
}

/** Quotes raw token amounts from the indexer's aggregate directional pool rates. */
export class IndexedRateIntentQuoteStrategy implements IntentQuoteStrategyHandler {
	constructor(
		private readonly chainConfigService: ChainConfigService,
		private readonly getQueryClient: () => IndexerQueryClient,
	) {}

	async quote(
		params: QuoteIntentParams,
		source: IntentQuoteChainContext,
		destination: IntentQuoteChainContext,
	): Promise<IndexedRateQuoteIntentResult> {
		validateQuoteParams(params)
		const sourceConfig = getConfigByStateMachineId(source.stateMachineId)
		const destinationConfig = getConfigByStateMachineId(destination.stateMachineId)
		if (!sourceConfig) throw new UnsupportedLiquidityChainError(source.stateMachineId)
		if (!destinationConfig) throw new UnsupportedLiquidityChainError(destination.stateMachineId)

		const tokenIn = this.resolveAsset(sourceConfig.stateMachineId, params.tokenIn)
		const tokenOut = this.resolveAsset(destinationConfig.stateMachineId, params.tokenOut)
		const [protocolFeeBps, rates] = await Promise.all([
			readProtocolFeeBps(this.chainConfigService, source),
			new LiquidityEngine(this.getQueryClient()).getBuyAndSellRates({
				sourceChain: sourceConfig.stateMachineId,
				destinationChain: destinationConfig.stateMachineId,
				tokenInSymbol: tokenIn.symbol,
				tokenOutSymbol: tokenOut.symbol,
			}),
		])
		if (!rates) {
			throw new IndexedRateUnavailableError({
				source: sourceConfig.stateMachineId,
				destination: destinationConfig.stateMachineId,
				tokenIn: tokenIn.symbol,
				tokenOut: tokenOut.symbol,
			})
		}

		const selectedRate = selectIndexedRate(rates, tokenIn.symbol, tokenOut.symbol)
		return quoteWithIndexedRate(params, tokenIn, tokenOut, selectedRate, rates, protocolFeeBps)
	}

	private resolveAsset(chain: string, address: HexString): ResolvedQuoteAsset {
		const asset = this.chainConfigService.getAssetMetadataByAddress(chain, address)
		if (!asset) throw new UnsupportedLiquidityAssetError(chain, address)
		const { decimals } = asset
		if (decimals === undefined || !Number.isSafeInteger(decimals) || decimals < 0) {
			throw new InvalidIndexedRateError(`decimals are not configured for ${asset.symbol} on ${chain}`)
		}
		return { ...asset, decimals }
	}
}

function selectIndexedRate(
	rates: BuyAndSellRates,
	tokenInSymbol: ConfiguredAssetSymbol,
	tokenOutSymbol: ConfiguredAssetSymbol,
): SelectedIndexedRate {
	if (tokenInSymbol === rates.baseTokenSymbol && tokenOutSymbol === rates.quoteTokenSymbol) {
		return readIndexedRate(
			"buy",
			rates.buyRate,
			rates.buyRateUpdatedAt,
			rates,
			tokenInSymbol,
			tokenOutSymbol,
		)
	}
	if (tokenInSymbol === rates.quoteTokenSymbol && tokenOutSymbol === rates.baseTokenSymbol) {
		return readIndexedRate(
			"sell",
			rates.sellRate,
			rates.sellRateUpdatedAt,
			rates,
			tokenInSymbol,
			tokenOutSymbol,
		)
	}
	throw new InvalidIndexedRateError(
		`indexed pair ${rates.baseTokenSymbol}/${rates.quoteTokenSymbol} does not match ${tokenInSymbol}/${tokenOutSymbol}`,
	)
}

function readIndexedRate(
	side: IndexedRateSide,
	rate: string | null,
	updatedAt: Date | null,
	rates: BuyAndSellRates,
	tokenInSymbol: ConfiguredAssetSymbol,
	tokenOutSymbol: ConfiguredAssetSymbol,
): SelectedIndexedRate {
	if (!rate || !updatedAt) {
		throw new IndexedRateUnavailableError({
			source: rates.sourceChain,
			destination: rates.destinationChain,
			tokenIn: tokenInSymbol,
			tokenOut: tokenOutSymbol,
			side,
		})
	}
	try {
		const scaledRate = parseUnits(rate, INDEXED_RATE_DECIMALS)
		if (scaledRate <= 0n || Number.isNaN(updatedAt.getTime())) throw new Error()
		return { side, rate, scaledRate, updatedAt }
	} catch {
		throw new InvalidIndexedRateError(`${side} rate or timestamp is invalid`)
	}
}

function quoteWithIndexedRate(
	params: QuoteIntentParams,
	tokenIn: ResolvedQuoteAsset,
	tokenOut: ResolvedQuoteAsset,
	selectedRate: SelectedIndexedRate,
	rates: BuyAndSellRates,
	protocolFeeBps: bigint,
): IndexedRateQuoteIntentResult {
	const inputUnit = 10n ** BigInt(tokenIn.decimals)
	const outputUnit = 10n ** BigInt(tokenOut.decimals)
	if (params.amountIn !== undefined) {
		const netAmountIn = deductProtocolFee(params.amountIn, protocolFeeBps)
		const amountOut =
			selectedRate.side === "buy"
				? (netAmountIn * selectedRate.scaledRate * outputUnit) / (inputUnit * INDEXED_RATE_SCALE)
				: (netAmountIn * outputUnit * INDEXED_RATE_SCALE) / (inputUnit * selectedRate.scaledRate)
		if (amountOut <= 0n) throw new InvalidIndexedRateError("quote rounds down to zero output")
		return buildResult("EXACT_INPUT", params.amountIn, amountOut, selectedRate, rates, protocolFeeBps)
	}

	if (params.amountOut === undefined) throw new Error("Quote amount is missing after validation")
	const netAmountIn =
		selectedRate.side === "buy"
			? divCeil(params.amountOut * inputUnit * INDEXED_RATE_SCALE, selectedRate.scaledRate * outputUnit)
			: divCeil(params.amountOut * inputUnit * selectedRate.scaledRate, outputUnit * INDEXED_RATE_SCALE)
	const amountIn = grossUpForProtocolFee(netAmountIn, protocolFeeBps)
	return buildResult("EXACT_OUTPUT", amountIn, params.amountOut, selectedRate, rates, protocolFeeBps)
}

function buildResult(
	tradeType: IndexedRateQuoteIntentResult["tradeType"],
	amountIn: bigint,
	amountOut: bigint,
	selectedRate: SelectedIndexedRate,
	rates: BuyAndSellRates,
	protocolFeeBps: bigint,
): IndexedRateQuoteIntentResult {
	return {
		strategy: "indexed_rates",
		tradeType,
		amountIn,
		amountOut,
		quoteMetadata: {
			sourceChain: rates.sourceChain,
			destinationChain: rates.destinationChain,
			baseTokenSymbol: rates.baseTokenSymbol,
			quoteTokenSymbol: rates.quoteTokenSymbol,
			rateSide: selectedRate.side,
			rate: selectedRate.rate,
			rateUpdatedAt: selectedRate.updatedAt,
			protocolFeeBps,
		},
	}
}
