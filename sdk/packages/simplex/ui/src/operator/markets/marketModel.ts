import type { ConfigDto } from "../../types"

/** Registry + configured asset symbols available to the running filler. */
export function marketSymbols(config: ConfigDto | undefined): string[] {
	const symbols = new Set<string>()
	for (const tokens of Object.values(config?.sendTokens ?? {})) {
		for (const token of tokens) {
			if (token.address !== "native") symbols.add(token.symbol)
		}
	}
	return [...symbols].sort()
}
