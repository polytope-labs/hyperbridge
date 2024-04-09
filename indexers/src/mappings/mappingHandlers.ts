import { HyperBridgeService } from "../services/hyperbridge-metrics.service";
import { RelayerService } from "../services/relayer.service";
import { PostRequestHandledEvent } from "../types/contracts/EthereumHostAbi";
import { TransferEvent } from "../types/contracts/ERC6160Ext20Abi";
import { TransferService } from "../services/transfer.service";

/**
 * Handles the PostRequestHandled event from Hyperbridge
 */
export async function handlePostRequestHandledEvent(
  event: PostRequestHandledEvent,
): Promise<void> {
  const log_info = {
    message: "Handling PostRequestHandled event",
    event: event,
  };

  logger.debug(JSON.stringify(log_info));

  const { args } = event;
  const { relayer } = args;

  await RelayerService.incrementNumberOfPostRequestsHandled(relayer);
  await HyperBridgeService.incrementNumberOfPostRequestsHandled();
}

/**
 * Handles the Transfer event from Hyper-usd contract
 */
export async function handleTransferEvent(event: TransferEvent): Promise<void> {
  const log_info = {
    message: "Handling Transfer event",
    event: event,
  };

  logger.debug(JSON.stringify(log_info));

  const { args, transactionHash } = event;
  const { from, to, value } = args;

  const transfer = await TransferService.storeTransfer({
    from,
    to,
    value,
    transactionHash,
  });

  await RelayerService.updateFeesEarned(transfer);
}
