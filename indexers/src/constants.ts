// Purpose: Contains all the constants used in the application.

import { SupportedChain, SupportedAssets } from "./types/enums";

export interface ITokenPriceFeedDetails {
  name: SupportedAssets;
  address: string;
  chain_link_price_feed: string;
}

// Chain IDs
export const CHAIN_IDS = {
  [SupportedChain.ETHE]: "11155111",
  [SupportedChain.BASE]: "84532",
  [SupportedChain.OPTI]: "11155420",
  [SupportedChain.ARBI]: "421614",
  [SupportedChain.BSC]: "97",
  [SupportedChain.POLY]: "80002",
  [SupportedChain.HYPERBRIDGE]:
    "0xb2bd3bcf03701f26ae353430c98c01a4acc334a199baa37b207298cad9d6229b",
};

// Start blocks for indexing
export const START_BLOCKS = {
  [SupportedChain.ETHE]: 5659633,
  [SupportedChain.BASE]: 8464600,
  [SupportedChain.OPTI]: 8906802,
  [SupportedChain.ARBI]: 20034995,
  [SupportedChain.BSC]: 38301829,
  [SupportedChain.HYPERBRIDGE]: 1,
};

// Contract addresses
export const CONTRACT_ADDRESSES = {
  [SupportedChain.ETHE]: {
    EthereumHost: "0x92F217a5e965EAa2aD356678D537A0A9ccC0AF41",
  },
  [SupportedChain.BASE]: {
    EthereumHost: "0xB72759815CF029EFDb957A676C3593Ec762CFD4e",
  },
  [SupportedChain.OPTI]: {
    EthereumHost: "0x27D689e361ab92aCab04Ea21c1B3F507A94a9DAd",
  },
  [SupportedChain.ARBI]: {
    EthereumHost: "0x15Ba7e42BC2c3e8FeDEb30D13CEE611D97315E7F",
  },
  [SupportedChain.BSC]: {
    EthereumHost: "0x0cac3dF856aD8939955086AADd243a28f35988BE",
  },
};

export const CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES = {
  [SupportedChain.ETHE]: "0x694AA1769357215DE4FAC081bf1f309aDC325306",
  [SupportedChain.BASE]: "0x4aDC67696bA383F43DD60A9e78F2C97Fbbfc7cb1",
  [SupportedChain.OPTI]: "0x61Ec26aA57019C486B10502285c5A3D4A4750AD7",
  [SupportedChain.ARBI]: "0xd30e2101a97dcbAeBCBC04F14C3f624E67A35165",
  [SupportedChain.BSC]: "0x2514895c72f50D8bd4B4F9b1110F0D6bD2c97526",
};

// Host addresses for the IsmpHost
export const HOST_ADDRESSES = [
  CONTRACT_ADDRESSES[SupportedChain.ETHE].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.BASE].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.OPTI].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.ARBI].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.BSC].EthereumHost,
];

export const ETHEREUM_L2_SUPPORTED_CHAINS = [
  SupportedChain.BASE,
  SupportedChain.OPTI,
];

export const SUPPORTED_ASSETS_CONTRACT_ADDRESSES: Record<
  SupportedChain,
  Array<ITokenPriceFeedDetails>
> = {
  [SupportedChain.ETHE]: [
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
  [SupportedChain.ARBI]: [],
  [SupportedChain.OPTI]: [],
  [SupportedChain.BASE]: [],
  [SupportedChain.BSC]: [],
  [SupportedChain.POLY]: [],
  [SupportedChain.HYPERBRIDGE]: [],
};
