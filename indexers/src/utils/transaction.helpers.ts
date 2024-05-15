import { EthereumTransaction } from "@subql/types-ethereum";
import { ethers } from "ethers";

/**
 * Get an ether.js transaction request from an Ethereum transaction
 */
export const getEthersTransactionRequest = async (
  transaction: EthereumTransaction,
): Promise<ethers.providers.TransactionRequest> => {
  const { effectiveGasPrice } = await transaction.receipt();

  return {
    to: transaction.to,
    from: transaction.from,
    nonce: transaction.nonce,
    gasPrice: effectiveGasPrice,
    value: transaction.value,
    chainId: Number(transaction.chainId),
    data: transaction.input,
    maxPriorityFeePerGas: transaction.maxPriorityFeePerGas,
    maxFeePerGas: transaction.maxFeePerGas,
    type: Number(transaction.type),
    // gasLimit?: "",
    // accessList: transaction.accessList,
  };
};
