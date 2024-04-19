import { CHAIN_IDS } from "../constants";
import { SupportedChain } from "../types/enums";

export const getSupportedChainByChainId = (
  chainId: string,
): SupportedChain | undefined => {
  return Object.keys(CHAIN_IDS).find(
    (key) => CHAIN_IDS[key as SupportedChain] === chainId,
  ) as SupportedChain;
};

export const hexToDecimal = (hexString: string): string => {
  return parseInt(hexString, 16).toString();
};
