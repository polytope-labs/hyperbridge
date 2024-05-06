import { UniswapV2PairAbi__factory } from "../types/contracts";
import { UNISWAP_USDC_ETH_CONTRACT_ADDRESS } from "../constants";

/**
 * Get the current price of Ethereum in USD
 */
export const getCurrentEthPriceInUsd = async (): Promise<number> => {
  const uniswapUsdcContract = UniswapV2PairAbi__factory.connect(
    UNISWAP_USDC_ETH_CONTRACT_ADDRESS,
    api,
  );

  const reserves = await uniswapUsdcContract.getReserves();

  // times 10^12 because usdc only has 6 decimals
  return (Number(reserves._reserve0) / Number(reserves._reserve1)) * 1e12;
};
