// Purpose: Contains all the constants used in the application.

import { SupportedAssets } from "./types/enums";

export const HYPERBRIDGE = "KUSAMA-4009";

export interface ITokenPriceFeedDetails {
  name: SupportedAssets;
  address: string;
  chain_link_price_feed: string;
}

export const SUPPORTED_ASSETS_CONTRACT_ADDRESSES: Record<
  string,
  Array<ITokenPriceFeedDetails>
> = {
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
};
