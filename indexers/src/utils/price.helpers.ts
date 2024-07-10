import {
  CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES,
  ITokenPriceFeedDetails,
} from "../constants";
import { SupportedChain } from "../types";
import { ChainLinkAggregatorV3Abi__factory } from "../types/contracts";

/**
 * Get the current price of the native chain currency in USD
 */
export const getNativeCurrencyPrice = async (
  chain: SupportedChain,
): Promise<bigint> => {
  let priceFeedAddress = "";
  switch (chain) {
    case SupportedChain.ETHE:
    case SupportedChain.BASE:
    case SupportedChain.ARBI:
    case SupportedChain.OPTI:
    case SupportedChain.BSC:
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
 * Get the current price IN USD for an ERC20 token given it's contract address
 */
export const getTokenPriceInUsd = async (
  priceFeedDetails: ITokenPriceFeedDetails,
): Promise<bigint> => {
  const priceFeedContract = ChainLinkAggregatorV3Abi__factory.connect(
    priceFeedDetails.chain_link_price_feed,
    api,
  );

  const roundData = await priceFeedContract.latestRoundData();
  const decimals = await priceFeedContract.decimals();
  let exponent = 18 - decimals;

  // Ensure we convert to the standard 18 decimals used by erc20.
  return roundData.answer.toBigInt() * BigInt(10 ** exponent);
};
