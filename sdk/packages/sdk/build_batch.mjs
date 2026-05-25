import { ApiPromise, WsProvider } from "@polkadot/api";

const HOST = "0x620128E2B19193d6Bd244a3AC8D3bBa0541B19c3";
const HANDLER = "0x2a18AB35DEa43474882E05A661e2F20fe89c0535";
const HOST_MANAGER = "0x59185DAc59B2E0Ade6a414964D039b0474d942c0";
const CONSENSUS = "0x5D4525F59F3d113110E3F8De1A99Ad8dC899A938";
const ADMIN = "0xe8599c331dc16fe8e6ED264E332b0f5007d32B2e";
const BANDWIDTH_MGR = "0x6A67533Ce73756FfaB17c05578A5FBBa5d9B2d8d";
const INTENT_GATEWAY = "0x16F9E57f735bBfF9f6c4E5276330f9c437d0e9E0";
const UNSTAKING = 1814400n;
const CHALLENGE = 0n;
const HYPERBRIDGE = "0x504f4c4b41444f542d33333637"; // "POLKADOT-3367"
const STATE_MACHINES = [3367];

// chainId -> { feeToken, uniswapV2 } (queried live from each EvmHost.hostParams())
const CHAINS = {
  1:         { fee: "0x6B175474E89094C44Da98b954EedeAC495271d0F", uni: "0x7a250d5630B4cF539739dF2C5dAcb4c659F2488D" },
  10:        { fee: "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1", uni: "0x4A7b5Da61326A6379179b40d00F57E5bbDC962c2" },
  42161:     { fee: "0xDA10009cBd5D07dd0CeCc66161FC93D7c9000da1", uni: "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24" },
  8453:      { fee: "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb", uni: "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24" },
  56:        { fee: "0x1AF3F329e8BE154074D8769D1FFa4eE058B1DBc3", uni: "0x4752ba5DBc23f44D87826276BF6Fd6b1C372aD24" },
  100:       { fee: "0xe91D153E0b41518A2Ce8Dd3D7944Fa863463a97d", uni: "0xe91D153E0b41518A2Ce8Dd3D7944Fa863463a97d" },
  137:       { fee: "0x8f3Cf7ad23Cd3CaDbD9735AFf958023239c6A063", uni: "0xedf6066a2b290C185783862C7F4776A2C8077AD1" },
  1868:      { fee: "0xbA9986D2381edf1DA03B0B9c1f8b00dc4AacC369", uni: "0x44c8d3ab2579Dc877955BbbB82E7d298645a8f5A" },
  420420419: { fee: "0x0000053900000000000000000000000001200000", uni: "0x0000000000000000000000000000000000000000" },
};
const chainIds = Object.keys(CHAINS).map(Number);

// Tiers (1024-based bytes, 30-day duration, 18-dec USD prices)
const KB = 1024n, MB = 1024n * 1024n, MONTH = 2592000n, E18 = 10n ** 18n;
const TIERS = [
  { name: "TierOne",   bytes: 100n * KB, price: 50n   * E18 },
  { name: "TierTwo",   bytes: 300n * KB, price: 100n  * E18 },
  { name: "TierThree", bytes: 1n   * MB, price: 250n  * E18 },
  { name: "TierFour",  bytes: 8n   * MB, price: 1000n * E18 },
];

const sm = (id) => ({ Evm: id });

const api = await ApiPromise.create({
  provider: new WsProvider("wss://nexus.rpc.polytope.technology", 1000),
  throwOnConnect: true,
  noInitWarn: true,
});

// sanity: paraId
const paraId = (await api.query.parachainInfo.parachainId()).toNumber();
console.error("Nexus paraId:", paraId);

const calls = [];

// 1. update_evm_hosts: BTreeMap<StateMachine, H160>
const hostMap = new Map(chainIds.map((id) => [sm(id), HOST]));
calls.push(api.tx.hostExecutive.updateEvmHosts(hostMap));

// 2. set_host_params: BTreeMap<StateMachine, HostParam::EvmHostParam>
const paramMap = new Map(chainIds.map((id) => [sm(id), {
  EvmHostParam: {
    feeToken: CHAINS[id].fee,
    admin: ADMIN,
    handler: HANDLER,
    hostManager: HOST_MANAGER,
    uniswapV2: CHAINS[id].uni,
    unStakingPeriod: UNSTAKING,
    challengePeriod: CHALLENGE,
    consensusClient: CONSENSUS,
    stateMachines: STATE_MACHINES,
    hyperbridge: HYPERBRIDGE,
  },
}]));
calls.push(api.tx.hostExecutive.setHostParams(paramMap));

// 3. set_manager + 4. set_allowlist (intent gateway) — per source chain
for (const id of chainIds) calls.push(api.tx.bandwidth.setManager(sm(id), BANDWIDTH_MGR));
for (const id of chainIds) calls.push(api.tx.bandwidth.setAllowlist(sm(id), INTENT_GATEWAY, true));

// 5. set_tier x4 (bytes + duration)
for (const t of TIERS)
  calls.push(api.tx.bandwidth.setTier(t.name, { bytes: t.bytes, durationSecs: MONTH }));

// 6. dispatch_set_tiers x9 (prices -> each EVM BandwidthManager)
const priceUpdates = TIERS.map((t) => [t.name, t.price]);
for (const id of chainIds) calls.push(api.tx.bandwidth.dispatchSetTiers(sm(id), priceUpdates));

// 7. remove uncle rewards: empty reward curve => position 0 full reward, no uncle rewards
calls.push(api.tx.beefyConsensusProofs.setRewardCurve([]));

const batch = api.tx.utility.batch(calls);

// Verify the TierIndex wire byte (read the encoded first arg of a set_tier call)
const tierByte = api.tx.bandwidth.setTier("TierOne", { bytes: 1n, durationSecs: 1n }).args[0].toHex();

console.error("total inner calls:", calls.length);
console.error("TierOne encodes as:", tierByte);
console.error("batch call hash:", batch.method.hash.toHex());
console.log(batch.method.toHex());

await api.disconnect();
