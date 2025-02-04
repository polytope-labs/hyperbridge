// Purpose: Contains all the constants used in the application.

import { SupportedAssets } from './types/enums';

export const HYPERBRIDGE = {
 testnet: 'KUSAMA-4009',
 mainnet: 'POLKADOT-3367',
};

export const BIFROST = {
 testnet: 'KUSAMA-2030',
 mainnet: 'POLKADOT-2030',
};

export const ETHEREUM = {
 testnet: 'EVM-11155111',
 mainnet: 'EVM-1',
};

export const ARBITRUM = {
 testnet: 'EVM-421614',
 mainnet: 'EVM-42161',
};

export const OPTIMISM = {
 testnet: 'EVM-11155420',
 mainnet: 'EVM-10',
};

export const BASE = {
 testnet: 'EVM-84532',
 mainnet: 'EVM-8453',
};

export const BSC = {
 testnet: 'EVM-97',
 mainnet: 'EVM-56',
};

export const CHAIN_IDS_BY_GENESIS = {
 // Hyperbridge
 '0x5388faf792c5232566d21493929b32c1f20a9c2b03e95615eefec2aa26d64b73':
  'KUSAMA-4009',
 '0x61ea8a51fd4a058ee8c0e86df0a89cc85b8b67a0a66432893d09719050c9f540':
  'POLKADOT-3367',

 // Bifrost
 '0x9f28c6a68e0fc9646eff64935684f6eeeece527e37bbe1f213d22caa1d9d6bed':
  'KUSAMA-2030',
 '0x262e1b2ad728475fd6fe88e62d34c200abe6fd693931ddad144059b1eb884e5b':
  'POLKADOT-2030',

 // BSC
 '97': 'EVM-97',
 '56': 'EVM-56',

 // Ethereum
 '11155111': 'EVM-11155111',
 '1': 'EVM-1',

 // Arbitrum
 '421614': 'EVM-421614',
 '42161': 'EVM-42161',

 // Optimism
 '11155420': 'EVM-11155420',
 '10': 'EVM-10',

 // Base
 '84532': 'EVM-84532',
 '8453': 'EVM-8453',
};

export interface ITokenPriceFeedDetails {
 name: SupportedAssets;
 address: string;
 chain_link_price_feed: string;
}

export const SUPPORTED_ASSETS_CONTRACT_ADDRESSES: Record<
 string,
 Array<ITokenPriceFeedDetails>
> = {
 'EVM-11155111': [
  {
   name: SupportedAssets.WETH,
   address: '0x980B62Da83eFf3D4576C647993b0c1D7faf17c73',
   chain_link_price_feed: '0x694AA1769357215DE4FAC081bf1f309aDC325306',
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
};
