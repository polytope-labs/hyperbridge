import { subqlTest } from "@subql/testing";
import { Relayer, Transfer } from "../../../types";

const existingEntities = [
  Relayer.create({
    id: "0xbC50b90751bfCccbFa4c7220261909d0f528b00f",
  }),
];

subqlTest(
  "transferEventHandler correctly computes totalTransfersIn & feesPayedOutToRelayers",
  5854079,
  existingEntities,
  [
    Transfer.create({
      id: "0xbf94293f2dbd11e71510d9add4338d5f362a67c69df6055c1c4c9bc965ac1f31",
      chain: "EVM-11155111",
      amount: BigInt("24000000000000000000000"),
      from: "0x92F217a5e965EAa2aD356678D537A0A9ccC0AF41",
      to: "0xbC50b90751bfCccbFa4c7220261909d0f528b00f",
    }),
  ],
  "handleTransferEvent"
);
