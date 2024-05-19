import { SubstrateEvent } from "@subql/types";

export async function handleIsmpStateMachineUpdated(
  event: SubstrateEvent,
): Promise<void> {
  const {
    event: {
      data: [account, balance],
    },
  } = event;

  logger.info(
    `Handling ISMP StateMachineUpdatedEvent: ${JSON.stringify(account)}`,
  );
}
