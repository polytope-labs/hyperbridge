/**
 * The ERC-4337 bundlers Hyperbridge runs, per chain id. They need no API key and serve
 * EntryPoint v0.8 and v0.9. Simplex submits every fill through the one for its chain, and a
 * chain absent here can only be watched.
 */
export const HYPERBRIDGE_BUNDLER_URLS: Readonly<Record<number, string>> = {
	1: "https://bundler.polytope.technology/ethereum",
	56: "https://bundler.polytope.technology/bsc",
	97: "https://bundler.polytope.technology/bsc-chapel",
	137: "https://bundler.polytope.technology/polygon",
	8453: "https://bundler.polytope.technology/base",
	42161: "https://bundler.polytope.technology/arbitrum",
	80002: "https://bundler.polytope.technology/polygon-amoy",
}

/** Hyperbridge's bundler for `chainId`, or undefined where it runs none. */
export function hyperbridgeBundlerUrl(chainId: number): string | undefined {
	return HYPERBRIDGE_BUNDLER_URLS[chainId]
}
