// Purpose: Contains all the constants used in the application.

import { SupportedChain } from "./types/enums";

// Chain IDs
export const CHAIN_IDS = {
  [SupportedChain.ETHEREUM_SEPOLIA]: "11155111",
  [SupportedChain.BASE_SEPOLIA]: "84532",
  [SupportedChain.OPTIMISM_SEPOLIA]: "11155420",
  [SupportedChain.ARBITRUM_SEPOLIA]: "421614",
  [SupportedChain.BSC_CHAPEL]: "97",
  [SupportedChain.HYPERBRIDGE_GARGANTUA]:
    "0xb2bd3bcf03701f26ae353430c98c01a4acc334a199baa37b207298cad9d6229b",
};

// Start blocks for indexing
export const START_BLOCKS = {
  [SupportedChain.ETHEREUM_SEPOLIA]: 5659633,
  [SupportedChain.BASE_SEPOLIA]: 8464600,
  [SupportedChain.OPTIMISM_SEPOLIA]: 8906802,
  [SupportedChain.ARBITRUM_SEPOLIA]: 20034995,
  [SupportedChain.BSC_CHAPEL]: 38301829,
  [SupportedChain.HYPERBRIDGE_GARGANTUA]: 1,
};

// Contract addresses
export const CONTRACT_ADDRESSES = {
  [SupportedChain.ETHEREUM_SEPOLIA]: {
    EthereumHost: "0xcD90465E75479a15f85faCB17B0342e609ef3f5f",
  },
  [SupportedChain.BASE_SEPOLIA]: {
    EthereumHost: "0x6f069107877D7b19f593C861fEa485568D466581",
  },
  [SupportedChain.OPTIMISM_SEPOLIA]: {
    EthereumHost: "0x72f7B1310D7dF9fb859f1a216133598f486b8994",
  },
  [SupportedChain.ARBITRUM_SEPOLIA]: {
    EthereumHost: "0xC4B58437d9A1Aa0eba4f128114110a1054cceB0F",
  },
  [SupportedChain.BSC_CHAPEL]: {
    EthereumHost: "0x338B01874A01D7593F85e2e3c1681A46f2f5Df4a",
  },
};

// Host addresses for the IsmpHost
export const HOST_ADDRESSES = [
  CONTRACT_ADDRESSES[SupportedChain.ETHEREUM_SEPOLIA].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.BASE_SEPOLIA].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.OPTIMISM_SEPOLIA].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.ARBITRUM_SEPOLIA].EthereumHost,
  CONTRACT_ADDRESSES[SupportedChain.BSC_CHAPEL].EthereumHost,
];

export const HYPERBRIDGE_STATS_ENTITY_ID = "1";
