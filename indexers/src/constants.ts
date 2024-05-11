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
    EthereumHost: "0x92F217a5e965EAa2aD356678D537A0A9ccC0AF41",
  },
  [SupportedChain.BASE_SEPOLIA]: {
    EthereumHost: "0xB72759815CF029EFDb957A676C3593Ec762CFD4e",
  },
  [SupportedChain.OPTIMISM_SEPOLIA]: {
    EthereumHost: "0x27D689e361ab92aCab04Ea21c1B3F507A94a9DAd",
  },
  [SupportedChain.ARBITRUM_SEPOLIA]: {
    EthereumHost: "0x15Ba7e42BC2c3e8FeDEb30D13CEE611D97315E7F",
  },
  [SupportedChain.BSC_CHAPEL]: {
    EthereumHost: "0x0cac3dF856aD8939955086AADd243a28f35988BE",
  },
};

export const CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES = {
  [SupportedChain.ETHEREUM_SEPOLIA]:
    "0x694AA1769357215DE4FAC081bf1f309aDC325306",
  [SupportedChain.BASE_SEPOLIA]: "0x4aDC67696bA383F43DD60A9e78F2C97Fbbfc7cb1",
  [SupportedChain.OPTIMISM_SEPOLIA]:
    "0x61Ec26aA57019C486B10502285c5A3D4A4750AD7",
  [SupportedChain.ARBITRUM_SEPOLIA]:
    "0xd30e2101a97dcbAeBCBC04F14C3f624E67A35165",
  [SupportedChain.BSC_CHAPEL]: "0x143db3CEEfbdfe5631aDD3E50f7614B6ba708BA7",
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
