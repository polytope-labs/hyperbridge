// Purpose: Contains all the constants used in the application.

import { SupportedAssets } from './types/enums';

export const HYPERBRIDGE = 'KUSAMA-4009';
export const BIFROST = 'KUSAMA-2030';
export const ETHEREUM = 'EVM-11155111';
export const ARBITRUM = 'EVM-421614';
export const OPTIMISM = 'EVM-11155420';
export const BASE = 'EVM-84532';
export const BSC = 'EVM-97';

export const CHAIN_IDS_BY_GENESIS = {
 '0x5388faf792c5232566d21493929b32c1f20a9c2b03e95615eefec2aa26d64b73':
  'KUSAMA-4009',
 '0x9f28c6a68e0fc9646eff64935684f6eeeece527e37bbe1f213d22caa1d9d6bed':
  'KUSAMA-2030',
 '0xd24D7542C74B1f4ee14dC4bD077d5eed47107d51': 'EVM-97',
 '0xfCA0c05bEb9564AC154f55173881B4DD221A18A8': 'EVM-11155111',
 '0xCA5508fB8abCDdeb330eAd57197feFBD62b5cb03': 'EVM-421614',
 '0x37BBd9d3CF34c9143Ae01E33BA1eB59c3AD00a0f': 'EVM-11155420',
 '0xe7C43500e07E0Bb5fC0987db95fE57Ce29B9bb80': 'EVM-84532',
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
