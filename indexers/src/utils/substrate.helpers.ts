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
      let main_key = "";
      let value = "";

      // There will only be one key in the object
      Object.keys(stateId).forEach((key) => {
        main_key = key.toUpperCase();
        value = stateId[key] === null ? "" : stateId[key];
      });

      switch (main_key) {
        case "ETHEREUM":
          switch (value.toUpperCase()) {
            case "EXECUTIONLAYER":
              return SupportedChain.ETHE;
            case "OPTIMISM":
              return SupportedChain.OPTI;
            case "ARBITRUM":
              return SupportedChain.ARBI;
            case "BASE":
              return SupportedChain.BASE;
            default:
              throw new Error(
                `Unknown state machine ID ${value} encountered in extractStateMachineIdFromSubstrateEventData`,
              );
          }
        case "BSC":
          return SupportedChain.BSC;
        case "POLYGON":
          return SupportedChain.POLY;
        case "POLKADOT":
          return "POLKADOT-".concat(value);
        case "KUSAMA":
          return "KUSAMA-".concat(value);
        case "BEEFY":
          return "BEEFY-".concat(value);
        case "GRANDPA":
          return "GRANDPA-".concat(value);
        default:
          throw new Error(
            `Unknown state machine ID ${main_key} encountered in extractStateMachineIdFromSubstrateEventData`,
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
