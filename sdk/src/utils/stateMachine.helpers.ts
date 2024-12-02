import { EthereumTransaction, EthereumResult } from "@subql/types-ethereum";
import { isHexString } from "ethers/lib/utils";

export default class StateMachineHelpers {
  /**
   * Get a state machine ID from an EVM transaction
   */
  static getEvmStateMachineIdFromTransaction(
    transaction: EthereumTransaction<EthereumResult>
  ): string {
    let chainId = transaction.chainId ? transaction.chainId.toString() : null;

    if (chainId === null) {
      logger.info(
        `Encountered null stateMachineId for transaction with hash: ${transaction.hash}`
      );
      throw new Error("stateMachineId is null");
    }

    if (isHexString(chainId)) {
      chainId = parseInt(chainId, 16).toString();
    }

    return `EVM-${chainId}`;
  }
}
