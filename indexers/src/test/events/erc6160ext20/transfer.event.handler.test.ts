import { subqlTest } from "@subql/testing";
import {
  HyperBridgeStats,
  Relayer,
  SupportedChain,
  Transfer,
} from "../../../types";

const existingEntities = [
  Relayer.create({
    id: "0xbC50b90751bfCccbFa4c7220261909d0f528b00f",
    chains: [SupportedChain.ETHEREUM_SEPOLIA],
    totalFeesEarned: BigInt(0),
    totalNumberOfMessagesDelivered: BigInt(0),
    totalNumberOfFailedMessagesDelivered: BigInt(0),
    totalNumberOfSuccessfulMessagesDelivered: BigInt(0),
  }),
  HyperBridgeStats.create({
    id: "HYPERBRIDGE_STATS_ENTITY_ID",
    numberOfMessagesSent: BigInt(0),
    numberOfDeliveredMessages: BigInt(0),
    numberOfFailedDeliveries: BigInt(0),
    numberOfTimedOutMessages: BigInt(0),
    numberOfUniqueRelayers: BigInt(0),
    feesPayedOutToRelayers: BigInt(0),
    protocolFeesEarned: BigInt(0),
    totalTransfersIn: BigInt(0),
  }),
];

subqlTest(
  "transferEventHandler correctly computes totalTransfersIn & feesPayedOutToRelayers",
  5854079,
  existingEntities,
  [
    Transfer.create({
      id: "0xbf94293f2dbd11e71510d9add4338d5f362a67c69df6055c1c4c9bc965ac1f31",
      chain: SupportedChain.ETHEREUM_SEPOLIA,
      amount: BigInt("24000000000000000000000"),
      from: "0x92F217a5e965EAa2aD356678D537A0A9ccC0AF41",
      to: "0xbC50b90751bfCccbFa4c7220261909d0f528b00f",
    }),
    HyperBridgeStats.create({
      id: "HYPERBRIDGE_STATS_ENTITY_ID",
      numberOfMessagesSent: BigInt(0),
      numberOfDeliveredMessages: BigInt(0),
      numberOfFailedDeliveries: BigInt(0),
      numberOfTimedOutMessages: BigInt(0),
      numberOfUniqueRelayers: BigInt(0),
      feesPayedOutToRelayers: BigInt("24000000000000000000000"),
      protocolFeesEarned: BigInt(0),
      totalTransfersIn: BigInt(0),
    }),
  ],
  "handleTransferEvent",
);