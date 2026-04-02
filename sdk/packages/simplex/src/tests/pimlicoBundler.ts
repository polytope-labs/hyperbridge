/**
 * Resolves the Pimlico API key for test chain configs.
 * Prefer `BUNDLER_URL` (e.g. https://api.pimlico.io/v2/8453/rpc?apikey=...) and fall back to `BUNDLER_API_KEY`.
 */
export function getPimlicoBundlerApiKey(): string | undefined {
	const fromEnvUrl = process.env.BUNDLER_URL
	if (fromEnvUrl) {
		try {
			const url = new URL(fromEnvUrl)
			const key = url.searchParams.get("apikey") ?? url.searchParams.get("apiKey")
			if (key) {
				return key
			}
		} catch {
			// ignore invalid URL
		}
	}
	return process.env.BUNDLER_API_KEY
}

/** Pimlico v2 RPC URL for `chainId`, or `undefined` if no API key is configured. */
export function pimlicoBundlerUrlForChain(chainId: number): string | undefined {
	const apiKey = getPimlicoBundlerApiKey()
	return apiKey ? `https://api.pimlico.io/v2/${chainId}/rpc?apikey=${apiKey}` : undefined
}
