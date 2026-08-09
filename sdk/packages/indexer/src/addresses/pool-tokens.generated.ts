// Auto-generated, DO NOT EDIT
// The tokens that liquidity pools are composed from, keyed by state machine id then
// lowercase token address. Addresses are the "yieldVaults" keys of the relevant chain
// entry in src/configs/config-mainnet.json / config-testnet.json; symbols and decimals
// are read from each token contract by scripts/generate-pool-tokens.ts.
//
// Decimals are per chain because the same symbol is not the same everywhere (BSC stables
// are 18-decimal, most others 6); every rate and depth normalization depends on them.
// To add a token, add it to that chain's "yieldVaults" and run `pnpm codegen:pool-tokens`
// with an RPC endpoint configured for the chain. Committed rather than gitignored: it
// derives from chain state, which nothing else in the repo can reconstruct.
export const POOL_TOKENS: Record<string, Record<string, { symbol: string; decimals: number }>> = {
	"EVM-1": {
		"0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48": { symbol: "USDC", decimals: 6 },
		"0xdac17f958d2ee523a2206206994597c13d831ec7": { symbol: "USDT", decimals: 6 },
		"0x9623dfb044d5612ce0c0f1606973ccaefd03cd05": { symbol: "USDR", decimals: 6 },
		"0x6b175474e89094c44da98b954eedeac495271d0f": { symbol: "DAI", decimals: 18 },
		"0x17cdb2a01e7a34cbb3dd4b83260b05d0274c8dab": { symbol: "cNGN", decimals: 6 },
		"0xb755506531786c8ac63b756bab1ac387bacb0c04": { symbol: "ZARP", decimals: 18 },
		"0x1abaea1f7c830bd89acc67ec4af516284b1bc33c": { symbol: "EURC", decimals: 6 },
		"0x70e8de73ce538da2beed35d14187f6959a8eca96": { symbol: "XSGD", decimals: 6 },
		"0x2c537e5624e4af88a7ae4060c022609376c8d0eb": { symbol: "TRYB", decimals: 6 }
	},
	"EVM-42161": {
		"0xaf88d065e77c8cc2239327c5edb3a432268e5831": { symbol: "USDC", decimals: 6 },
		"0xfd086bc7cd5c481dcc9c85ebe478a1c0b69fcbb9": { symbol: "USDT", decimals: 6 },
		"0xda10009cbd5d07dd0cecc66161fc93d7c9000da1": { symbol: "DAI", decimals: 18 }
	},
	"EVM-10": {
		"0x0b2c639c533813f4aa9d7837caf62653d097ff85": { symbol: "USDC", decimals: 6 },
		"0x94b008aa00579c1307b0ef2c499ad98a8ce58e58": { symbol: "USDT", decimals: 6 },
		"0xda10009cbd5d07dd0cecc66161fc93d7c9000da1": { symbol: "DAI", decimals: 18 }
	},
	"EVM-8453": {
		"0x833589fcd6edb6e08f4c7c32d4f71b54bda02913": { symbol: "USDC", decimals: 6 },
		"0x46c85152bfe9f96829aa94755d9f915f9b10ef5f": { symbol: "cNGN", decimals: 6 },
		"0xfde4c96c8593536e31f229ea8f37b2ada2699bb2": { symbol: "USDT", decimals: 6 },
		"0x3b5f2810fb2168ffa9c73160f97bf9f2461ffa5c": { symbol: "USDR", decimals: 6 },
		"0x50c5725949a6f0c72e6c4a641f24049a917db0cb": { symbol: "DAI", decimals: 18 },
		"0xb755506531786c8ac63b756bab1ac387bacb0c04": { symbol: "ZARP", decimals: 18 },
		"0x60a3e35cc302bfa44cb288bc5a4f316fdb1adb42": { symbol: "EURC", decimals: 6 }
	},
	"EVM-56": {
		"0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d": { symbol: "USDC", decimals: 18 },
		"0x55d398326f99059ff775485246999027b3197955": { symbol: "USDT", decimals: 18 },
		"0x1af3f329e8be154074d8769d1ffa4ee058b1dbc3": { symbol: "DAI", decimals: 18 }
	},
	"EVM-100": {
		"0xe91d153e0b41518a2ce8dd3d7944fa863463a97d": { symbol: "DAI", decimals: 18 },
		"0xddafbb505ad214d7b80b1f830fccc89b60fb7a83": { symbol: "USDC", decimals: 6 },
		"0x4ecaba5870353805a9f068101a40e0f32ed605c6": { symbol: "USDT", decimals: 6 }
	},
	"EVM-1868": {
		"0xba9986d2381edf1da03b0b9c1f8b00dc4aacc369": { symbol: "USDC", decimals: 6 }
	},
	"EVM-137": {
		"0x3c499c542cef5e3811e1192ce70d8cc03d5c3359": { symbol: "USDC", decimals: 6 },
		"0xc2132d05d31c914a87c6611c10748aeb04b58e8f": { symbol: "USDT", decimals: 6 },
		"0x3b5f2810fb2168ffa9c73160f97bf9f2461ffa5c": { symbol: "USDR", decimals: 6 },
		"0x8f3cf7ad23cd3cadbd9735aff958023239c6a063": { symbol: "DAI", decimals: 18 },
		"0x52828daa48c1a9a06f37500882b42daf0be04c3b": { symbol: "cNGN", decimals: 6 },
		"0xb755506531786c8ac63b756bab1ac387bacb0c04": { symbol: "ZARP", decimals: 18 },
		"0xdc3326e71d45186f113a2f448984ca0e8d201995": { symbol: "XSGD", decimals: 6 }
	},
	"EVM-420420419": {
		"0x0000053900000000000000000000000001200000": { symbol: "USDC", decimals: 6 },
		"0x000007c000000000000000000000000001200000": { symbol: "USDT", decimals: 6 }
	},
	"EVM-97": {
		"0xa801da100bf16d07f668f4a49e1f71fc54d05177": { symbol: "USDC", decimals: 18 },
		"0xc043f483373072f7f27420d6e7d7ad269c018e18": { symbol: "USDT", decimals: 18 }
	},
	"EVM-80002": {
		"0xbe97e73126d66188d72fbf99029126d0340a7f18": { symbol: "USDC", decimals: 18 }
	},
	"EVM-420420417": {
		"0x0dc440cf87830f0af564eb8b62b454b7e0c68a4b": { symbol: "USDC", decimals: 18 }
	},
}
