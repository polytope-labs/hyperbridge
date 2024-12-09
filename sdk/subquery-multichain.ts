import { MultichainProjectManifest } from "@subql/types-core";

const project: MultichainProjectManifest = {
  specVersion: "1.0.0",
  query: {
    name: "@subql/query",
    version: "*",
  },
  projects: [
    "./src/chains/testnet/ethereum-sepolia.ts",
    // "./src/chains/testnet/arbitrum-sepolia.ts",
    // "./src/chains/testnet/base-sepolia.ts",
    // "./src/chains/testnet/optimism-sepolia.ts",
    // "./src/chains/testnet/bsc-chapel.ts",
    // "./src/chains/testnet/hyperbridge-gargantua.ts",
  ],
};

export default project;
