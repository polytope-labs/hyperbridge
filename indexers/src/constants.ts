// Purpose: Contains all the constants used in the application.

import { SupportedChain } from "./types/enums";

// Chain IDs
export const CHAIN_IDS = {
  [SupportedChain.ETHEREUM_SEPOLIA]: "11155111",
  [SupportedChain.BASE_SEPOLIA]: "84532",
};

// Start blocks for indexing
export const START_BLOCKS = {
  [SupportedChain.ETHEREUM_SEPOLIA]: 4085662,
  [SupportedChain.BASE_SEPOLIA]: 4085662,
};

// Contract addresses
export const CONTRACT_ADDRESSES = {
  [SupportedChain.ETHEREUM_SEPOLIA]: {
    EthereumHost: "0xe4226c474A6f4BF285eA80c2f01c0942B04323e5",
  },
  [SupportedChain.BASE_SEPOLIA]: {
    EthereumHost: "0x7b79995e5f793A07Bc00c21412e50Ecae098E7f9",
  },
};

export const HYPERBRIDGE_METRICS_ENTITY_ID = "1";
