/**
 * Validates raw TOML `[vault.uniswapV4].positions` entries. Pure and
 * browser-safe — shared by `validateConfig` and the funding planner.
 * Throws on missing/invalid required fields.
 */
export function validateUniswapV4Positions(positions: { chain?: string; tokenId?: string }[]): void {
	for (const pos of positions) {
		if (!pos.chain?.trim()) {
			throw new Error("Each UniswapV4 vault position must have a non-empty 'chain' (e.g. EVM-8453)")
		}
		if (!pos.tokenId) {
			throw new Error("Each UniswapV4 position must include a 'tokenId'")
		}
	}
}
