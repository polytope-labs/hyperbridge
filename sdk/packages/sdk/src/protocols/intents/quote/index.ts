import { ChainConfigService } from "@/configs/ChainConfigService"
import { UniswapV4IntentQuoteStrategy } from "./uniswapV4"
import {
	type IntentQuoteStrategyHandler,
	type QuoteIntentParams,
	type QuoteIntentResult,
	UnsupportedIntentQuoteStrategyError,
} from "./types"

export {
	type IntentQuoteChain,
	type IntentQuoteStrategy,
	type IntentQuoteToken,
	type IntentQuoteTradeType,
	type QuoteIntentParams,
	type QuoteIntentResult,
	UnsupportedIntentQuotePairError,
	UnsupportedIntentQuoteStrategyError,
	type UniswapV4IntentQuoteMetadata,
	type UniswapV4IntentQuoteOptions,
	type UniswapV4PoolKey,
} from "./types"

/**
 * Partner-facing intent quote service.
 *
 * The service is intentionally strategy-based. `uniswap_v4` is the only
 * registered strategy today, but callers can keep using `quoteIntent` as new
 * strategies are added.
 */
export class IntentQuoteService {
	private readonly strategies: Record<string, IntentQuoteStrategyHandler>

	constructor(chainConfigService = new ChainConfigService()) {
		this.strategies = {
			uniswap_v4: new UniswapV4IntentQuoteStrategy(chainConfigService),
		}
	}

	async quoteIntent(params: QuoteIntentParams): Promise<QuoteIntentResult> {
		const strategy = params.strategy ?? "uniswap_v4"
		const handler = this.strategies[strategy]
		if (!handler) throw new UnsupportedIntentQuoteStrategyError(strategy)

		return handler.quote({ ...params, strategy })
	}
}

/**
 * Quotes a Hyperbridge intent. Currently supports Uniswap V4 only.
 */
export async function quoteIntent(params: QuoteIntentParams): Promise<QuoteIntentResult> {
	return new IntentQuoteService().quoteIntent(params)
}
