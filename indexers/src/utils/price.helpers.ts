import { CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES } from "../constants";
import { SupportedChain } from "../types";
import { ChainLinkAggregatorV3Abi__factory } from "../types/contracts";
import optimism from "@eth-optimism/sdk";
import { ethers } from "ethers";

/**
 * Get the current price of the native chain currency in USD
 */
export const getNativeCurrencyPrice = async (
  chain: SupportedChain,
): Promise<bigint> => {
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

  const priceFeedContract = ChainLinkAggregatorV3Abi__factory.connect(
    priceFeedAddress,
    api,
  );

  const roundData = await priceFeedContract.latestRoundData();
  const decimals = await priceFeedContract.decimals();
  let exponent = 18 - decimals;

  // Ensure we convert to the standard 18 decimals used by erc20.
  return roundData.answer.toBigInt() * BigInt(10 ** exponent);
};

/**
 * Estimates the amount of L1 gas cost for a given L2 transaction in wei.
 */
export const getL1GasCostEstimate = async (
  chain: SupportedChain,
  transactionRequest: ethers.providers.TransactionRequest,
): Promise<bigint> => {
  switch (chain) {
    case SupportedChain.OPTIMISM_SEPOLIA:
    case SupportedChain.BASE_SEPOLIA:
      const provider = optimism.asL2Provider(api);
      return (await provider.estimateL1GasCost(transactionRequest)).toBigInt();
    default:
      throw Error(`L1 Gas Cost not supported for chain: ${chain}`);
  }
};
