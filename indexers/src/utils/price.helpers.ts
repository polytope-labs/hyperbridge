import fetch from "node-fetch";
import { URLSearchParams } from "url";

interface EtherScanEthPriceApiResponse {
  status: string;
  message: string;
  result: {
    ethbtc: string;
    ethbtc_timestamp: string;
    ethusd: string;
    ethusd_timestamp: string;
  };
}

/**
 * Get the current price of Ethereum in USD
 */
export const getCurrentEthPriceInUsd = async (): Promise<number> => {
  try {
    const response = await fetch(
      "https://api.etherscan.io/api?" +
        new URLSearchParams({
          module: "stats",
          action: "ethprice",
          apikey: "KFQDJX6KXMP52YU3YCYJZ57SZ3PKUFMP32",
        }),
    );

    const data: EtherScanEthPriceApiResponse =
      (await response.json()) as EtherScanEthPriceApiResponse;
    return Number(data.result.ethusd);
  } catch (e) {
    throw new Error(`Failed to get current ETH price: ${e}`);
  }
};
