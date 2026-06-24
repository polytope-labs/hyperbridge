// Auto-generated, DO NOT EDIT
// ERC-4626 vault addresses per chain, keyed by underlying token address (lowercase).
// Aave stata token addresses sourced from https://github.com/bgd-labs/aave-address-book
// Values are arrays because multiple vaults may wrap the same underlying token.
// To add or update vaults, edit the "yieldVaults" field in the relevant chain entry
// in src/configs/config-mainnet.json (or config-testnet.json) and re-run codegen.
export const YIELD_VAULT_ADDRESSES: Record<string, Record<string, string[]>> = {
	// EVM-1
	"EVM-1": {
		// USDC → stataUSDC (Aave v3 Ethereum)
		"0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": ["0xD4fa2D31b7968E448877f69A96DE69f5de8cD23E"],
		// USDT → stataUSDT (Aave v3 Ethereum)
		"0xdac17f958d2ee523a2206206994597c13d831ec7": ["0x7Bc3485026Ac48b6cf9BaF0A377477Fff5703Af8"]
	},

	// EVM-42161
	"EVM-42161": {
		// USDC (native Circle) → stataUSDCn (Aave v3 Arbitrum)
		"0xaf88d065e77c8cc2239327c5edb3a432268e5831": ["0x7F6501d3B98eE91f9b9535E4b0ac710Fb0f9e0bc"],
		// USDT → stataUSDT (Aave v3 Arbitrum)
		"0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9": ["0xa6D12574eFB239FC1D2099732bd8b5dC6306897F"]
	},

	// EVM-10
	"EVM-10": {
		// USDC (native Circle) → stataUSDCn (Aave v3 Optimism)
		"0x0b2c639c533813f4aa9d7837caf62653d097ff85": ["0x41B334E9F2C0ED1f30fD7c351874a6071C53a78E"],
		// USDT → stataUSDT (Aave v3 Optimism)
		"0x94b008aa00579c1307b0ef2c499ad98a8ce58e58": ["0x927CfF131fD5B43FC992D071929b2c095d6E4b70"]
	},

	// EVM-8453
	"EVM-8453": {
		// USDC → stataUSDC (Aave v3 Base; no USDT market on Base)
		"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913": ["0xC768c589647798a6EE01A91FdE98EF2ed046DBD6"],
		// cNGN → ycNGN (StreamingYieldVault Base)
		"0x46c85152bfe9f96829aa94755d9f915f9b10ef5f": ["0xa82a3531021317240fb32e67f9c7bc091f737d3b"]
	},

	// EVM-56
	"EVM-56": {
		// USDC (Binance-pegged) → staticAUSDC (Aave v3 BNB Chain)
		"0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d": ["0x3906cDdfb781f02B21f21BD81ed7Fd8DC37075E1"],
		// USDT (BSC-USD) → staticAUSDT (Aave v3 BNB Chain)
		"0x55d398326f99059ff775485246999027b3197955": ["0x0471D185cc7Be61E154277cAB2396cD397663da6"]
	},

	// EVM-137
	"EVM-137": {
		// USDC (native Circle) → stataUSDCn (Aave v3 Polygon)
		"0x3c499c542cef5e3811e1192ce70d8cc03d5c3359": ["0x79261231698B26Ed9085b59ae89d59843Ae925a8"],
		// USDT → stataUSDT0 (Aave v3 Polygon)
		"0xc2132d05d31c914a87c6611c10748aeb04b58e8f": ["0x2eaD203C5C1C00612B1DdbBb20e4180dA822d6ff"]
	},
}
