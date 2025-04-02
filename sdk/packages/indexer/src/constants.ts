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

import { CHAIN_IDS_BY_GENESIS } from "./chain-ids-by-genesis"
import { CHAINS_BY_ISMP_HOST } from "./chains-by-ismp-host"
export { CHAIN_IDS_BY_GENESIS, CHAINS_BY_ISMP_HOST }

// Replaced by auto-generated version

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
