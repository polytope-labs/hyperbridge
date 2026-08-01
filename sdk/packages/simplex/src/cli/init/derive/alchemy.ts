import { chainByChainId } from "../chains"

export interface ParsedAlchemyUrl {
	subdomain: string
	apiKey: string
}

/** Matches `https://<subdomain>.g.alchemy.com/v2/<apiKey>`. */
export function parseAlchemyUrl(url: string): ParsedAlchemyUrl | null {
	let parsed: URL
	try {
		parsed = new URL(url)
	} catch {
		return null
	}
	if (parsed.protocol !== "https:") return null
	if (!parsed.hostname.endsWith(".g.alchemy.com")) return null

	const subdomain = parsed.hostname.slice(0, -".g.alchemy.com".length)
	if (!subdomain || subdomain.includes(".")) return null

	const match = parsed.pathname.match(/^\/v2\/([^/]+)$/)
	if (!match) return null

	return { subdomain, apiKey: match[1] }
}

export function isAlchemyUrl(url: string): boolean {
	return parseAlchemyUrl(url) !== null
}

/** Builds the Alchemy RPC URL for a chain, or null when Alchemy doesn't serve it. */
export function deriveAlchemyRpc(apiKey: string, chainId: number): string | null {
	const chain = chainByChainId(chainId)
	if (!chain?.alchemySubdomain) return null
	return `https://${chain.alchemySubdomain}.g.alchemy.com/v2/${apiKey}`
}
