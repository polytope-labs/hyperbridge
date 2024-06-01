import { CHAIN_IDS } from "../constants";
import { SupportedChain } from "../types/enums";
import { EthereumResult, EthereumTransaction } from "@subql/types-ethereum";

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

export const getEvmChainFromTransaction = (
  transaction: EthereumTransaction<EthereumResult>,
): SupportedChain => {
  const chainId = transaction.chainId
    ? hexToDecimal(transaction.chainId.toString())
    : "";
  const chain = getSupportedChainByChainId(chainId);

  if (!chain) {
    throw new Error(
      `Unsupported chainId ${chainId} for handlePostRequest transaction`,
    );
  }

  return chain;
};
