/**
 * Get the StateMachineID parsing the stringified object which substrate provides
 */
export const extractStateMachineIdFromSubstrateEventData = (
  substrateStateMachineId: string
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
        case "EVM":
          return "EVM-".concat(value);
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
            `Unknown state machine ID ${main_key} encountered in extractStateMachineIdFromSubstrateEventData`
          );
      }
    } else {
      throw new Error(
        `StateId not present in stateMachineId: ${substrateStateMachineId}`
      );
    }
  } catch (error) {
    logger.error(error);
  }
};
