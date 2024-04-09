// Purpose: Contains all the constants used in the application.

// Chains indexed for Hyperbridge
export enum SupportedChains {
  ETHEREUM_SEPOLIA,
  BASE_SEPOLIA,
}

// Chain IDs
export const CHAIN_IDS = {
  [SupportedChains.ETHEREUM_SEPOLIA]: "11155111",
  [SupportedChains.BASE_SEPOLIA]: "84532",
};

// Start blocks for indexing
export const START_BLOCKS = {
  [SupportedChains.ETHEREUM_SEPOLIA]: 4085662,
  [SupportedChains.BASE_SEPOLIA]: 4085662,
};

// Contract addresses
export const CONTRACT_ADDRESSES = {
  [SupportedChains.ETHEREUM_SEPOLIA]: {
    EthereumHost: "0xe4226c474A6f4BF285eA80c2f01c0942B04323e5",
  },
  [SupportedChains.BASE_SEPOLIA]: {
    EthereumHost: "0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9",
  },
};

export const HYPERBRIDGE_METRICS_ENTITY_ID = "1";
