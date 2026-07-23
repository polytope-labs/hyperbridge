export interface ParsedPimlicoUrl {
	chainId: number
	apiKey: string
}

/** Matches `https://api.pimlico.io/v2/<chainId>/rpc?apikey=<key>`. */
export function parsePimlicoUrl(url: string): ParsedPimlicoUrl | null {
	let parsed: URL
	try {
		parsed = new URL(url)
	} catch {
		return null
	}
	if (parsed.protocol !== "https:") return null
	if (parsed.hostname !== "api.pimlico.io") return null

	const match = parsed.pathname.match(/^\/v2\/([^/]+)\/rpc$/)
	if (!match) return null

	const chainId = Number(match[1])
	if (!Number.isInteger(chainId) || chainId <= 0) return null

	const apiKey = parsed.searchParams.get("apikey")
	if (!apiKey) return null

	return { chainId, apiKey }
}

export function isPimlicoUrl(url: string): boolean {
	return parsePimlicoUrl(url) !== null
}

export function derivePimlicoBundler(apiKey: string, chainId: number): string {
	// parsePimlicoUrl URL-decodes the key (searchParams.get); re-encode so keys
	// containing +/%/space survive the parse -> derive round-trip unchanged.
	return `https://api.pimlico.io/v2/${chainId}/rpc?apikey=${encodeURIComponent(apiKey)}`
}
