import { CHAINLINK_PRICE_FEED_CONTRACT_ADDRESSES } from "../constants";
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

  return roundData.answer.toBigInt();
};
