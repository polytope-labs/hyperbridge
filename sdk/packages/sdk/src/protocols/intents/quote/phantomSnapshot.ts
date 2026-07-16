import type { ChainConfigService } from "@/configs/ChainConfigService"
import { Chains } from "@/configs/chain"
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

type PhantomQuoteSymbol = "USDC" | "USDT" | "cNGN"
interface ResolvedPhantomPair {
	tokenA: HexString
	tokenB: HexString
}

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
		const pair = this.resolvePair(params, source.stateMachineId, destination.stateMachineId)
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

		if (params.amountIn !== undefined) return this.quoteExactInput(params.amountIn, protocolFeeBps, snapshot)
		if (params.amountOut !== undefined) return this.quoteExactOutput(params.amountOut, protocolFeeBps, snapshot)
		throw new Error("Quote amount is missing after validation")
	}

	private quoteExactInput(
		amountIn: bigint,
		protocolFeeBps: bigint,
		snapshot: PhantomOrderPriceSnapshot,
	): PhantomSnapshotQuoteIntentResult {
		const netAmountIn = deductProtocolFee(amountIn, protocolFeeBps)
		const amountOut = (netAmountIn * snapshot.medianPrice) / snapshot.standardAmount
		if (amountOut <= 0n) {
			throw new InvalidPhantomSnapshotError(snapshot.commitment, "quote rounds down to zero output")
		}

		return this.buildResult("EXACT_INPUT", amountIn, amountOut, protocolFeeBps, snapshot)
	}

	private quoteExactOutput(
		amountOut: bigint,
		protocolFeeBps: bigint,
		snapshot: PhantomOrderPriceSnapshot,
	): PhantomSnapshotQuoteIntentResult {
		const netAmountIn = divCeil(amountOut * snapshot.standardAmount, snapshot.medianPrice)
		const amountIn = grossUpForProtocolFee(netAmountIn, protocolFeeBps)
		return this.buildResult("EXACT_OUTPUT", amountIn, amountOut, protocolFeeBps, snapshot)
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

	private resolvePair(
		params: Pick<QuoteIntentParams, "tokenIn" | "tokenOut">,
		sourceStateMachineId: string,
		destinationStateMachineId: string,
	): ResolvedPhantomPair | undefined {
		const tokenInSymbol = this.resolveSymbol(params.tokenIn, sourceStateMachineId)
		const tokenOutSymbol = this.resolveSymbol(params.tokenOut, destinationStateMachineId)
		if (!tokenInSymbol || !tokenOutSymbol || !isSupportedPair(tokenInSymbol, tokenOutSymbol)) return

		const tokenA = this.chainConfigService.getAssetAddress(PHANTOM_INTENT_QUOTE_CHAIN, tokenInSymbol)
		const tokenB = this.chainConfigService.getAssetAddress(PHANTOM_INTENT_QUOTE_CHAIN, tokenOutSymbol)
		if (!isConfiguredAddress(tokenA) || !isConfiguredAddress(tokenB)) return

		return { tokenA, tokenB }
	}

	private resolveSymbol(token: HexString, chain: string): PhantomQuoteSymbol | undefined {
		for (const symbol of ["USDC", "USDT", "cNGN"] as const) {
			const configuredAddress = this.chainConfigService.getAssetAddress(chain, symbol)
			if (isConfiguredAddress(configuredAddress) && configuredAddress.toLowerCase() === token.toLowerCase()) {
				return symbol
			}
		}
		return undefined
	}
}

function isSupportedPair(tokenIn: PhantomQuoteSymbol, tokenOut: PhantomQuoteSymbol): boolean {
	return (
		(tokenIn === "cNGN" && (tokenOut === "USDC" || tokenOut === "USDT")) ||
		(tokenOut === "cNGN" && (tokenIn === "USDC" || tokenIn === "USDT"))
	)
}

function isConfiguredAddress(address?: HexString): address is HexString {
	return Boolean(address && address !== "0x" && !/^0x0{40}$/i.test(address))
}
