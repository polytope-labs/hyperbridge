// Purpose: Contains all the constants used in the application.

enum SupportedAssets {
	WETH,
	WBTC,
	USDT,
	USDC,
	DAI,
	DOT,
}

export const HYPERBRIDGE = {
	testnet: "KUSAMA-4009",
	mainnet: "POLKADOT-3367",
}

export const BIFROST = {
	testnet: "KUSAMA-2030",
	mainnet: "POLKADOT-2030",
}

export const ETHEREUM = {
	testnet: "EVM-11155111",
	mainnet: "EVM-1",
}

export const ARBITRUM = {
	testnet: "EVM-421614",
	mainnet: "EVM-42161",
}

export const OPTIMISM = {
	testnet: "EVM-11155420",
	mainnet: "EVM-10",
}

export const BASE = {
	testnet: "EVM-84532",
	mainnet: "EVM-8453",
}

export const BSC = {
	testnet: "EVM-97",
	mainnet: "EVM-56",
}

export const GNOSIS = {
	testnet: "EVM-10200",
	mainnet: "EVM-100",
}

export const SONEMIUM = {
	testnet: "EVM-1946",
	mainnet: "EVM-1868",
}

export const SUBSTRATE_RPC_URL = require("./substrate-ws.json")

export const CHAIN_IDS_BY_GENESIS = {
	// Hyperbridge
	"0x5388faf792c5232566d21493929b32c1f20a9c2b03e95615eefec2aa26d64b73": "KUSAMA-4009",
	"0x61ea8a51fd4a058ee8c0e86df0a89cc85b8b67a0a66432893d09719050c9f540": "POLKADOT-3367",

	// Bifrost
	"0xec39b15e5a1945ff19b8e8c0f76990b5758ce19faa4578e8ed57eda33e844452": "KUSAMA-2030",
	"0x262e1b2ad728475fd6fe88e62d34c200abe6fd693931ddad144059b1eb884e5b": "POLKADOT-2030",

	// cere network
	"0x81443836a9a24caaa23f1241897d1235717535711d1d3fe24eae4fdc942c092c": "SUBSTRATE-cere",

	// BSC
	"97": "EVM-97",
	"56": "EVM-56",

	// Ethereum
	"11155111": "EVM-11155111",
	"1": "EVM-1",

	// Arbitrum
	"421614": "EVM-421614",
	"42161": "EVM-42161",

	// Optimism
	"11155420": "EVM-11155420",
	"10": "EVM-10",

	// Base
	"84532": "EVM-84532",
	"8453": "EVM-8453",

	// Gnosis
	"10200": "EVM-10200",
	"100": "EVM-100",

	//Sonemium
	"1946": "EVM-1946",
	"1868": "EVM-1868",
}

export const CHAINS_BY_ISMP_HOST = {
	// Base
	"EVM-84532": "0xD198c01839dd4843918617AfD1e4DDf44Cc3BB4a",
	"EVM-8453": "0x6FFe92e4d7a9D589549644544780e6725E84b248",

	// BSC
	"EVM-97": "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7",
	"EVM-56": "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7",

	// Arbitrum
	"EVM-421614": "0x3435bD7e5895356535459D6087D1eB982DAd90e7",
	"EVM-42161": "0xE05AFD4Eb2ce6d65c40e1048381BD0Ef8b4B299e",

	// Optimism
	"EVM-11155420": "0x6d51b678836d8060d980605d2999eF211809f3C2",
	"EVM-10": "0x78c8A5F27C06757EA0e30bEa682f1FD5C8d7645d",

	// Ethereum
	"EVM-11155111": "0x2EdB74C269948b60ec1000040E104cef0eABaae8",
	"EVM-1": "0x792A6236AF69787C40cF76b69B4c8c7B28c4cA20",

	// Gnosis
	"EVM-10200": "0x58A41B89F4871725E5D898d98eF4BF917601c5eB",
	"EVM-100": "0x50c236247447B9d4Ee0561054ee596fbDa7791b1",

	// Sonemium
	"EVM-1946": "",
	"EVM-1868": "0x7F0165140D0f3251c8f6465e94E9d12C7FD40711",
}

export interface ITokenPriceFeedDetails {
	name: SupportedAssets
	address: string
	chain_link_price_feed: string
}

export const SUPPORTED_ASSETS_CONTRACT_ADDRESSES: Record<string, Array<ITokenPriceFeedDetails>> = {
	"EVM-11155111": [
		{
			name: SupportedAssets.WETH,
			address: "0x980B62Da83eFf3D4576C647993b0c1D7faf17c73",
			chain_link_price_feed: "0x694AA1769357215DE4FAC081bf1f309aDC325306",
		},
		// {
		//   name: SupportedAssets.WBTC,
		//   address: "0x806D0637Fbbfb4EB9efD5119B0895A5C7Cbc66e7",
		// },
		// {
		//   name: SupportedAssets.USDT,
		//   address: "0xaA8E23Fb1079EA71e0a56F48a2aA51851D8433D0",
		// },
		// {
		//   name: SupportedAssets.USDC,
		//   address: "0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238",
		// },
		// {
		//   name: SupportedAssets.DAI,
		//   address: "0x6C7661e66256eaEb3B06d397089cda7C025b61b3s",
		// },
	],
}
