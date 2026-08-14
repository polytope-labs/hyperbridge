// Uniswap V4 deployments the phantom snapshot reads declared positions from.
//
// Only chains where a solver can actually park liquidity in V4 need an entry: a bid that declares
// positions on a chain absent from this map has the declaration ignored and is weighted by its
// plain token balance, exactly as before. Base is the only chain carrying a configured V4 pool
// (see `uniswapV4Pools` in sdk/src/configs/chain.ts), so it is the only one listed.
//
// Addresses mirror `UniswapV4PositionManager` and `UniswapV4StateView` in that same chain registry.

export interface UniswapV4Deployment {
	positionManager: string
	stateView: string
}

export const UNISWAP_V4_ADDRESSES: Record<string, UniswapV4Deployment> = {
	"EVM-8453": {
		positionManager: "0x7c5f5a4bbd8fd63184577525326123b519429bdc",
		stateView: "0xd13dd3d6e93f276fafc9db9e6bb47c1180aee0c4",
	},
}
