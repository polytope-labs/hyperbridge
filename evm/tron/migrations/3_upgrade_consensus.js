/**
 * TronBox Migration: Upgrade consensus contracts
 *
 * Deploys new BeefyV1 and ConsensusRouter,
 * then updates the TronHost's hostParams to use the new router.
 *
 * Required: ParachainProof was changed from `Parachain parachain` (singular)
 * to `Parachain[] parachains` (array) — the on-chain contracts still use the
 * old layout, so we must redeploy to match the current Rust prover encoding.
 */

const HeaderImpl = artifacts.require("HeaderImpl");
const Codec = artifacts.require("Codec");
const BeefyV1 = artifacts.require("BeefyV1");
const ConsensusRouter = artifacts.require("ConsensusRouter");
const TronHost = artifacts.require("TronHost");

const ZERO_ADDRESS_HEX = "0x0000000000000000000000000000000000000000";

const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const BLOCK_TIME = 6000;

module.exports = async function (deployer, network, accounts) {
    const hostAddress = process.env.TRON_HOST;
    if (!hostAddress) {
        console.error("ERROR: Set TRON_HOST env var to the existing TronHost address");
        process.exit(1);
    }

    const tronHost = await TronHost.at(hostAddress);

    console.log("\n╔═══════════════════════════════════════════╗");
    console.log("║   Upgrade Consensus Contracts             ║");
    console.log("╠═══════════════════════════════════════════╣");
    console.log(`║  Network   : ${network.padEnd(28)}║`);
    console.log(`║  TronHost  : ${hostAddress.slice(0, 28).padEnd(28)}║`);
    console.log("╚═══════════════════════════════════════════╝\n");

    // ─── 1. Deploy shared libraries ──────────────────────────────────
    console.log("→ Deploying HeaderImpl library ...");
    await deployer.deploy(HeaderImpl);
    console.log("  ✓ HeaderImpl:", HeaderImpl.address);

    console.log("→ Deploying Codec library ...");
    await deployer.deploy(Codec);
    console.log("  ✓ Codec:", Codec.address);

    // ─── 2. Deploy BeefyV1 (naive proof verifier) ────────────────────
    console.log("→ Linking libraries → BeefyV1 ...");
    await deployer.link(HeaderImpl, BeefyV1);
    await deployer.link(Codec, BeefyV1);

    console.log("→ Deploying BeefyV1 ...");
    await deployer.deploy(BeefyV1);
    const beefyV1 = await BeefyV1.deployed();
    console.log("  ✓ BeefyV1:", beefyV1.address);

    // ─── 3. Deploy ConsensusRouter ───────────────────────────────────
    // SP1Beefy set to zero — not available on TRON (depends on sp1-contracts)
    console.log("→ Deploying ConsensusRouter ...");
    await deployer.deploy(
        ConsensusRouter,
        ZERO_ADDRESS_HEX,              // sp1Beefy — not available on TRON
        beefyV1.address,               // ecdsaBeefy (naive)
    );
    const newRouter = await ConsensusRouter.deployed();
    console.log("  ✓ ConsensusRouter:", newRouter.address);

    // ─── 5. Update TronHost to use new ConsensusRouter ───────────────
    console.log("→ Waiting for block confirmation ...");
    await sleep(BLOCK_TIME);

    console.log("→ Reading current host params ...");
    const currentParams = await tronHost.hostParams();

    const updatedParams = [
        currentParams.defaultTimeout.toString(),
        currentParams.defaultPerByteFee.toString(),
        currentParams.stateCommitmentFee.toString(),
        currentParams.feeToken,
        currentParams.admin,
        currentParams.handler,
        currentParams.hostManager,
        currentParams.uniswapV2,
        currentParams.unStakingPeriod.toString(),
        currentParams.challengePeriod.toString(),
        newRouter.address,               // ← new consensusClient
        currentParams.stateMachines.map(sm => sm.toString()),
        currentParams.perByteFees.map(f => f.toString()),
        currentParams.hyperbridge,
    ];

    console.log("→ Updating host params with new consensus client ...");
    await tronHost.updateHostParams(updatedParams);
    console.log("  ✓ Host params updated");

    // ─── Summary ─────────────────────────────────────────────────────
    console.log("\n╔═══════════════════════════════════════════════════════════╗");
    console.log("║              Upgrade Summary                             ║");
    console.log("╠═══════════════════════════════════════════════════════════╣");
    console.log(`║  BeefyV1             : ${beefyV1.address}`);
    console.log(`║  SP1Beefy            : (not deployed — zero address)`);
    console.log(`║  ConsensusRouter     : ${newRouter.address}`);
    console.log(`║  TronHost (updated)  : ${hostAddress}`);
    console.log("╚═══════════════════════════════════════════════════════════╝\n");
};
