import type { AdminStrategyDto, ConfigDto } from "../../types"

export const CUSTOM_TOKEN = "__custom__"

export function marketDescription(strategy: AdminStrategyDto): string {
	if (strategy.referenceOnly) return "Reference price only"
	if (strategy.sameToken) return "Cross-chain transfer market"
	return strategy.bid && strategy.ask ? "Buy and sell" : strategy.bid ? "Buy only" : "Sell only"
}

/** Registry + configured asset symbols available to the running filler. */
export function marketSymbols(config: ConfigDto | undefined): string[] {
	const symbols = new Set<string>()
	for (const tokens of Object.values(config?.sendTokens ?? {})) {
		for (const token of tokens) {
			if (token.symbol !== "native") symbols.add(token.symbol)
		}
	}
	return [...symbols].sort()
}
