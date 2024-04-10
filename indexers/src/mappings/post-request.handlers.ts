import { HyperBridgeService } from "../services/hyperbridge-metrics.service";
import { RelayerService } from "../services/relayer.service";
import { PostRequestHandledEvent } from "../types/contracts/EthereumHostAbi";
import { SupportedChain } from "../types/enums";

/**
 * Handles the PostRequestHandled event from Hyperbridge
 */
async function handlePostRequestHandledEvent(
  event: PostRequestHandledEvent,
  network: SupportedChain,
): Promise<void> {
  const log_info = {
    message: "Handling PostRequestHandled event",
    network,
    event: event,
  };

  logger.debug(JSON.stringify(log_info));

  const { args } = event;
  const { relayer } = args;

  await RelayerService.incrementNumberOfPostRequestsHandled(relayer, network);
  await HyperBridgeService.incrementNumberOfPostRequestsHandled();
}

// Handle the PostRequestHandled event for the Ethereum Sepolia chain
export async function handleEthereumSepoliaPostRequestHandledEvent(
  event: PostRequestHandledEvent,
): Promise<void> {
  await handlePostRequestHandledEvent(event, SupportedChain.ETHEREUM_SEPOLIA);
}
