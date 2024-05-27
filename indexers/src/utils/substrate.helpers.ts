import { SupportedChain } from "../types";

/**
 * Get the StateMachineID parsing the stringified object which substrate provides
 */
export const extractStateMachineIdFromSubstrateEventData = (
  substrateStateMachineId: string,
): string | undefined => {
  try {
    const stateMachineId = JSON.parse(substrateStateMachineId);
    if (stateMachineId && stateMachineId.stateId) {
      const stateId = stateMachineId.stateId;
      let state_machine_id = "";

      Object.keys(stateId).forEach((key) => {
        state_machine_id =
          stateId[key] === null
            ? key.toUpperCase()
            : stateId[key].toUpperCase();
      });

      switch (state_machine_id) {
        case "EXECUTIONLAYER":
          return SupportedChain.ETHE;
        case "OPTIMISM":
          return SupportedChain.OPTI;
        case "ARBITRUM":
          return SupportedChain.ARBI;
        case "BASE":
          return SupportedChain.BASE;
        case "BSC":
          return SupportedChain.BSC;
        case "POLYGON":
          return SupportedChain.POLY;

        default:
          throw new Error(
            `Unknown state machine ID ${state_machine_id} encountered in extractStateMachineIdFromSubstrateEventData`,
          );
      }
    } else {
      throw new Error(
        `StateId not present in stateMachineId: ${substrateStateMachineId}`,
      );
    }
  } catch (error) {
    logger.error(error);
  }
};
