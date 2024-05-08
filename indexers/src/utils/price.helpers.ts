import { ethers } from "ethers";
import { CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES } from "../constants";
import { SupportedChain } from "../types";

/**
 * Get the current price of the native chain currency in USD
 */
export const getNativeCurrencyPrice = async (
  chain: SupportedChain,
): Promise<number> => {
  const chainLinkAggregatorV3InterfaceABI = [
    {
      inputs: [],
      name: "decimals",
      outputs: [{ internalType: "uint8", name: "", type: "uint8" }],
      stateMutability: "view",
      type: "function",
    },
    {
      inputs: [],
      name: "description",
      outputs: [{ internalType: "string", name: "", type: "string" }],
      stateMutability: "view",
      type: "function",
    },
    {
      inputs: [{ internalType: "uint80", name: "_roundId", type: "uint80" }],
      name: "getRoundData",
      outputs: [
        { internalType: "uint80", name: "roundId", type: "uint80" },
        { internalType: "int256", name: "answer", type: "int256" },
        { internalType: "uint256", name: "startedAt", type: "uint256" },
        { internalType: "uint256", name: "updatedAt", type: "uint256" },
        { internalType: "uint80", name: "answeredInRound", type: "uint80" },
      ],
      stateMutability: "view",
      type: "function",
    },
    {
      inputs: [],
      name: "latestRoundData",
      outputs: [
        { internalType: "uint80", name: "roundId", type: "uint80" },
        { internalType: "int256", name: "answer", type: "int256" },
        { internalType: "uint256", name: "startedAt", type: "uint256" },
        { internalType: "uint256", name: "updatedAt", type: "uint256" },
        { internalType: "uint80", name: "answeredInRound", type: "uint80" },
      ],
      stateMutability: "view",
      type: "function",
    },
    {
      inputs: [],
      name: "version",
      outputs: [{ internalType: "uint256", name: "", type: "uint256" }],
      stateMutability: "view",
      type: "function",
    },
  ];

  let priceFeedAddress = "";
  switch (chain) {
    case SupportedChain.ETHEREUM_SEPOLIA:
    case SupportedChain.BASE_SEPOLIA:
    case SupportedChain.ARBITRUM_SEPOLIA:
    case SupportedChain.OPTIMISM_SEPOLIA:
    case SupportedChain.BSC_CHAPEL:
      priceFeedAddress = CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES[chain];
      break;
    default:
      throw Error(`Native price not supported for chain: ${chain}`);
  }

  const priceFeedContract = new ethers.Contract(
    priceFeedAddress,
    chainLinkAggregatorV3InterfaceABI,
    api,
  );

  const roundData = await priceFeedContract.latestRoundData();
  const decimals = await priceFeedContract.decimals();
  const price = parseFloat(roundData[1]) / Math.pow(10, Number(decimals));

  return price;
};
