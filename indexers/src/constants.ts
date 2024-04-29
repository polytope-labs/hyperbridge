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
    EthereumHost: "0xe4226c474A6f4BF285eA80c2f01c0942B04323e5",
  },
  [SupportedChain.BASE_SEPOLIA]: {
    EthereumHost: "0x1D14e30e440B8DBA9765108eC291B7b66F98Fd09",
  },
  [SupportedChain.OPTIMISM_SEPOLIA]: {
    EthereumHost: "0x39f3D7a7783653a04e2970e35e5f32F0e720daeB",
  },
  [SupportedChain.ARBITRUM_SEPOLIA]: {
    EthereumHost: "0x56101AD00677488B3576C85e9e75d4F0a08BD627",
  },
  [SupportedChain.BSC_CHAPEL]: {
    EthereumHost: "0x4e5bbdd9fE89F54157DDb64b21eD4D1CA1CDf9a6",
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
