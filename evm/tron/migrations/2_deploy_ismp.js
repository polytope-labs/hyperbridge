/**
 * TronBox Migration: Deploy Hyperbridge ISMP contracts to TRON
 *
 * Deployment order:
 *   1. BeefyV1FiatShamir  (consensus client)
 *   2. MultiProofClient   (consensus router — only FiatShamir is active)
 *   3. HandlerV1          (message handler)
 *   4. HostManager        (cross-chain governance)
 *   5. TronHost           (ISMP host)
 *   6. CallDispatcher     (untrusted call dispatcher)
 *   7. IntentGatewayV2    (intent-based bridging)
 *
 * After deployment the script wires everything together:
 *   - HostManager.setIsmpHost(tronHost)
 *   - TronHost.setConsensusState(...)   [if CONSENSUS_STATE env is set]
 *   - IntentGatewayV2.setParams(...)
 */

const BeefyV1FiatShamir = artifacts.require("BeefyV1FiatShamir");
const MultiProofClient = artifacts.require("MultiProofClient");
const HandlerV1 = artifacts.require("HandlerV1");
const HostManager = artifacts.require("HostManager");
const TronHost = artifacts.require("TronHost");
const CallDispatcher = artifacts.require("CallDispatcher");
const IntentGatewayV2 = artifacts.require("IntentGatewayV2");

const ZERO_ADDRESS = "T9yD14Nj9j7xAB4dbGeiX9h8unkKHxuWwb";
const ZERO_ADDRESS_HEX = "0x0000000000000000000000000000000000000000";
const ZERO_BYTES32 =
  "0x0000000000000000000000000000000000000000000000000000000000000000";

// 2 hours in seconds
const DEFAULT_TIMEOUT = 2 * 60 * 60;
// 21 days in seconds
const UNSTAKING_PERIOD = 21 * 24 * 60 * 60;

/**
 * Encode a UTF-8 string as a 0x-prefixed hex byte string.
 *
 * This mirrors the Solidity expression:
 *   bytes(string.concat("POLKADOT-", Strings.toString(id)))
 *
 * @param {string} str  e.g. "POLKADOT-3367"
 * @returns {string}    e.g. "0x504f4c4b41444f542d33333637"
 */
function stringToHex(str) {
  return "0x" + Buffer.from(str, "utf8").toString("hex");
}

module.exports = async function (deployer, network, accounts) {
  // ── Environment ───────────────────────────────────────────────────────
  const admin = process.env.ADMIN || accounts[0];
  const feeToken = process.env.FEE_TOKEN || ZERO_ADDRESS_HEX;
  const uniswapV2 = process.env.UNISWAP_V2 || ZERO_ADDRESS_HEX;
  const paraId = parseInt(process.env.PARA_ID || "3367", 10);
  const isMainnet = (process.env.NETWORK || "mainnet") === "mainnet";
  const priceOracle = process.env.PRICE_ORACLE || ZERO_ADDRESS_HEX;
  const solverSelection = process.env.SOLVER_SELECTION === "true";
  const surplusShareBps = parseInt(process.env.SURPLUS_SHARE_BPS || "0", 10);
  const protocolFeeBps = parseInt(process.env.PROTOCOL_FEE_BPS || "0", 10);

  // Fee token decimals — default 6 for USDT/USDC on TRON
  const decimals = parseInt(process.env.FEE_TOKEN_DECIMALS || "6", 10);
  const defaultPerByteFee = BigInt(3) * BigInt(10) ** BigInt(decimals - 3); // $0.003/byte
  const stateCommitmentFee = BigInt(1) * BigInt(10) ** BigInt(decimals); // $1

  // Hyperbridge state machine identifier
  // Uses string format: "POLKADOT-{paraId}" or "KUSAMA-{paraId}"
  // encoded as UTF-8 bytes, matching StateMachine.polkadot() / StateMachine.kusama()
  const hyperbridgeStr = isMainnet
    ? `POLKADOT-${paraId}`
    : `KUSAMA-${paraId}`;
  const hyperbridge = stringToHex(hyperbridgeStr);

  console.log("\n╔═══════════════════════════════════════════╗");
  console.log("║   Hyperbridge TRON Deployment             ║");
  console.log("╠═══════════════════════════════════════════╣");
  console.log(`║  Network     : ${network.padEnd(27)}║`);
  console.log(`║  Admin       : ${String(admin).slice(0, 27).padEnd(27)}║`);
  console.log(`║  Para ID     : ${String(paraId).padEnd(27)}║`);
  console.log(`║  Is Mainnet  : ${String(isMainnet).padEnd(27)}║`);
  console.log("╚═══════════════════════════════════════════╝\n");

  // ═════════════════════════════════════════════════════════════════════
  //  1. Deploy BeefyV1FiatShamir
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Deploying BeefyV1FiatShamir ...");
  await deployer.deploy(BeefyV1FiatShamir);
  const beefyV1FiatShamir = await BeefyV1FiatShamir.deployed();
  console.log("  ✓ BeefyV1FiatShamir:", beefyV1FiatShamir.address);

  // ═════════════════════════════════════════════════════════════════════
  //  2. Deploy MultiProofClient
  //     Only BeefyV1FiatShamir is active; SP1Beefy and BeefyV1 are
  //     set to address(0) since they are not used on TRON.
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Deploying MultiProofClient ...");
  await deployer.deploy(
    MultiProofClient,
    ZERO_ADDRESS_HEX, // sp1Beefy  — not deployed on TRON
    ZERO_ADDRESS_HEX, // beefyV1   — not deployed on TRON
    beefyV1FiatShamir.address // beefyV1FiatShamir
  );
  const multiProofClient = await MultiProofClient.deployed();
  console.log("  ✓ MultiProofClient:", multiProofClient.address);

  // ═════════════════════════════════════════════════════════════════════
  //  3. Deploy HandlerV1
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Deploying HandlerV1 ...");
  await deployer.deploy(HandlerV1);
  const handler = await HandlerV1.deployed();
  console.log("  ✓ HandlerV1:", handler.address);

  // ═════════════════════════════════════════════════════════════════════
  //  4. Deploy HostManager
  //     The host address is initially zero; it will be set after TronHost
  //     is deployed, sealing the cyclic dependency.
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Deploying HostManager ...");
  await deployer.deploy(HostManager, [admin, ZERO_ADDRESS_HEX]);
  const hostManager = await HostManager.deployed();
  console.log("  ✓ HostManager:", hostManager.address);

  // ═════════════════════════════════════════════════════════════════════
  //  5. Deploy TronHost
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Deploying TronHost ...");

  const stateMachines = [paraId];
  const perByteFees = []; // empty — uses defaultPerByteFee for all destinations

  // HostParams struct — field order must match the Solidity struct definition
  const hostParams = [
    DEFAULT_TIMEOUT, // defaultTimeout
    defaultPerByteFee.toString(), // defaultPerByteFee
    stateCommitmentFee.toString(), // stateCommitmentFee
    feeToken, // feeToken
    admin, // admin
    handler.address, // handler
    hostManager.address, // hostManager
    uniswapV2, // uniswapV2
    UNSTAKING_PERIOD, // unStakingPeriod
    0, // challengePeriod
    multiProofClient.address, // consensusClient
    stateMachines, // stateMachines
    perByteFees, // perByteFees
    hyperbridge, // hyperbridge
  ];

  await deployer.deploy(TronHost, hostParams);
  const tronHost = await TronHost.deployed();
  console.log("  ✓ TronHost:", tronHost.address);

  // ═════════════════════════════════════════════════════════════════════
  //  5a. Seal the HostManager ↔ TronHost cyclic dependency
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Linking HostManager → TronHost ...");
  await hostManager.setIsmpHost(tronHost.address);
  console.log("  ✓ HostManager.setIsmpHost done");

  // ═════════════════════════════════════════════════════════════════════
  //  5b. Set initial consensus state (if provided)
  // ═════════════════════════════════════════════════════════════════════
  const consensusState = process.env.CONSENSUS_STATE;
  if (consensusState && consensusState.length > 2) {
    console.log("→ Setting initial consensus state ...");
    await tronHost.setConsensusState(
      consensusState,
      [paraId, 1], // StateMachineHeight { stateMachineId, height }
      [
        // StateCommitment { timestamp, overlayRoot, stateRoot }
        Math.floor(Date.now() / 1000),
        ZERO_BYTES32,
        ZERO_BYTES32,
      ]
    );
    console.log("  ✓ Consensus state set");
  } else {
    console.log(
      "  ⚠ CONSENSUS_STATE not provided — skipping setConsensusState"
    );
  }

  // ═════════════════════════════════════════════════════════════════════
  //  6. Deploy CallDispatcher
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Deploying CallDispatcher ...");
  await deployer.deploy(CallDispatcher);
  const callDispatcher = await CallDispatcher.deployed();
  console.log("  ✓ CallDispatcher:", callDispatcher.address);

  // ═════════════════════════════════════════════════════════════════════
  //  7. Deploy IntentGatewayV2
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Deploying IntentGatewayV2 ...");
  await deployer.deploy(IntentGatewayV2, admin);
  const intentGateway = await IntentGatewayV2.deployed();
  console.log("  ✓ IntentGatewayV2:", intentGateway.address);

  // ═════════════════════════════════════════════════════════════════════
  //  7a. Initialize IntentGatewayV2 parameters
  // ═════════════════════════════════════════════════════════════════════
  console.log("→ Initializing IntentGatewayV2 params ...");

  // Params struct — field order must match the Solidity struct definition
  const intentParams = [
    tronHost.address, // host
    callDispatcher.address, // dispatcher
    solverSelection, // solverSelection
    surplusShareBps, // surplusShareBps
    protocolFeeBps, // protocolFeeBps
    priceOracle, // priceOracle
  ];

  await intentGateway.setParams(intentParams);
  console.log("  ✓ IntentGatewayV2 params set");

  // ═════════════════════════════════════════════════════════════════════
  //  Summary
  // ═════════════════════════════════════════════════════════════════════
  console.log(
    "\n╔═══════════════════════════════════════════════════════════╗"
  );
  console.log(
    "║              Deployment Summary                          ║"
  );
  console.log(
    "╠═══════════════════════════════════════════════════════════╣"
  );
  console.log(`║  BeefyV1FiatShamir : ${beefyV1FiatShamir.address}`);
  console.log(`║  MultiProofClient  : ${multiProofClient.address}`);
  console.log(`║  HandlerV1         : ${handler.address}`);
  console.log(`║  HostManager       : ${hostManager.address}`);
  console.log(`║  TronHost          : ${tronHost.address}`);
  console.log(`║  CallDispatcher    : ${callDispatcher.address}`);
  console.log(`║  IntentGatewayV2   : ${intentGateway.address}`);
  console.log(
    "╚═══════════════════════════════════════════════════════════╝\n"
  );
};
