/**
 * Extended phantom-order E2E — exercises the REAL simplex IntentFiller end to end.
 *
 * Several real `IntentFiller` instances (one per solver, each with its own FX price policy) connect
 * to the simnode, WATCH for the phantom order via their own phantom-bidding subscription, quote
 * USDC→cNGN with the FX strategy, build a fillOrder UserOp through the full ContractInteractionService
 * pipeline, and submit a bid — i.e. the complete simplex bid-submission path, not a hand-rolled
 * UserOp. The test then asserts every bid landed and is discoverable via `intents_getBidsForOrder`.
 *
 * Gated out of the default run (`*.simnode.test.ts`); run with `pnpm --filter @hyperbridge/simplex test:phantom-filler-e2e`.
 *
 * Requires:
 *   - a hyperbridge simnode (manual seal):
 *       ./target/debug/hyperbridge simnode --chain gargantua-1000 --rpc-port 9990 --tmp \
 *         --rpc-methods=unsafe --rpc-cors=all --pool-type=single-state
 *   - anvil forking Base mainnet (real IntentGateway + USDC + cNGN + eth_simulateV1):
 *       anvil --fork-url https://base-mainnet.g.alchemy.com/v2/<KEY> --port 8545
 *
 * Override endpoints via SIMNODE_URL / ANVIL_URL.
 */
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { encodeAbiParameters, keccak256, toHex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { describe, it, expect, beforeAll, afterAll } from "vitest"
import { IntentFiller } from "@/core/filler"
import {
	BidStorageService,
	CacheService,
	ChainClientManager,
	ContractInteractionService,
	FillerConfigService,
	type ResolvedChainConfig,
	type FillerConfig as FillerServiceConfig,
} from "@/services"
import { createSimplexSigner, SignerType } from "@/services/wallet"
import { FXFiller, type TradingPair } from "@/strategies/fx"
import { AssetRegistry } from "@/config/asset-registry"
import { Decimal } from "decimal.js"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import {
	ChainConfigService,
	IntentsCoprocessor,
	type ChainConfig,
	type FillerConfig,
	type HexString,
} from "@hyperbridge/sdk"
import {
	aggregatePhantomBids,
	fetchBidsForOrder,
	decodeUserOpScale,
	extractFillData,
} from "@hyperbridge/sdk/intents-helpers"

/** Builds an exotic-pair set + registry for tests: `token1` addresses traded against USDC and USDT. */
function exoticPairs(
	resolver: FillerConfigService,
	token1: Record<string, HexString>,
	maxOrderSize: number,
	bidPricePolicy?: FillerPricePolicy,
	askPricePolicy?: FillerPricePolicy,
): { pairs: TradingPair[]; registry: AssetRegistry } {
	const registry = new AssetRegistry(resolver, { EXOTIC: token1 })
	const pairs: TradingPair[] = ["USDC", "USDT"].map((token0) => ({
		token0,
		token1: "EXOTIC",
		maxOrderSize: new Decimal(maxOrderSize),
		bidPricePolicy,
		askPricePolicy,
	}))
	return { pairs, registry }
}

const SIMNODE_URL = process.env.SIMNODE_URL || "ws://127.0.0.1:9990"
const ANVIL_URL = process.env.ANVIL_URL || "http://127.0.0.1:8545"

const BASE_STATE_MACHINE = "EVM-8453"
const BASE_CHAIN_ID = 8453
const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" as HexString
const CNGN_BASE = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F" as HexString
const CNGN_BALANCE_SLOT = 201n // cNGN _balances slot on Base (see indexer TOKEN_SLOT_OVERRIDES)
const ETH0_CONSENSUS_ID = "0x45544830"
const STANDARD_AMOUNT = 1_000_000n // 1 USDC (6 decimals)
// The real SolverAccount on Base, already deployed in the fork; solvers delegate to it and the
// aggregation only counts bids from senders that do.
const SOLVER_ACCOUNT = new ChainConfigService().getSolverAccountAddress(BASE_STATE_MACHINE)!

// One IntentFiller per solver. Distinct substrate keys (to place independent bids) and EVM keys
// (distinct solver addresses/liquidity), and distinct FX prices so there is a real range to reduce.
// //Alice is reserved for the driver's sudo/sealing, so fillers use other dev accounts to avoid
// nonce contention.
const FILLERS = [
	{
		suri: "//Bob",
		evmKey: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString,
		cngnPerUsd: "1500",
	},
	{
		suri: "//Charlie",
		evmKey: "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" as HexString,
		cngnPerUsd: "1510",
	},
	{
		suri: "//Dave",
		evmKey: "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6" as HexString,
		cngnPerUsd: "1520",
	},
]

// ─── simnode driving (manual seal) ──────────────────────────────────────────────────────────────

async function rpc(api: ApiPromise, method: string, params: unknown[] = []): Promise<any> {
	return (api as any)._rpcCore.provider.send(method, params)
}
async function createBlock(api: ApiPromise): Promise<void> {
	const block = await rpc(api, "engine_createBlock", [true, false])
	await rpc(api, "engine_finalizeBlock", [block.hash])
}
async function submitAndSeal(api: ApiPromise, extrinsic: any, signer: any): Promise<void> {
	await extrinsic.signAndSend(signer)
	await new Promise((r) => setTimeout(r, 200))
	await createBlock(api)
}
async function sudoAndSeal(api: ApiPromise, call: any): Promise<void> {
	const alice = new Keyring({ type: "sr25519" }).addFromUri("//Alice")
	await submitAndSeal(api, api.tx.sudo.sudo(call), alice)
}
async function seedStateMachineHeight(api: ApiPromise, chainId: number, height: bigint): Promise<void> {
	const id = { state_id: { Evm: chainId }, consensus_state_id: ETH0_CONSENSUS_ID }
	const key = api.query.ismp.latestStateMachineHeight.key(id)
	await sudoAndSeal(api, api.tx.system.setStorage([[key, api.createType("u64", height).toHex()]]))
}
async function setPhantomOrderConfig(api: ApiPromise): Promise<void> {
	const config = {
		chain: { state_id: { Evm: BASE_CHAIN_ID }, consensus_state_id: ETH0_CONSENSUS_ID },
		token_pairs: [{ token_a: USDC_BASE, token_b: CNGN_BASE, standard_amount: STANDARD_AMOUNT }],
		interval_blocks: 10,
	}
	await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomOrderConfig(config))
}
async function getActivePhantomCommitment(api: ApiPromise): Promise<HexString | null> {
	const raw: any = await api.rpc.state.getStorage(api.query.intentsCoprocessor.currentPhantomOrder.key())
	const hex: string | undefined = raw?.toHex()
	if (!hex || hex === "0x" || hex.length < 66) return null
	return `0x${hex.slice(2, 66)}` as HexString
}
// Configures both directions of the pair (USDC→cNGN and cNGN→USDC) so the generated order carries
// two legs — exercises the filler quoting every pair in one phantom order.
async function setBothPhantomPairs(api: ApiPromise): Promise<void> {
	const config = {
		chain: { state_id: { Evm: BASE_CHAIN_ID }, consensus_state_id: ETH0_CONSENSUS_ID },
		token_pairs: [
			{ token_a: USDC_BASE, token_b: CNGN_BASE, standard_amount: STANDARD_AMOUNT },
			{ token_a: CNGN_BASE, token_b: USDC_BASE, standard_amount: STANDARD_AMOUNT },
		],
		interval_blocks: 10,
	}
	await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomOrderConfig(config))
}
// The active phantom commitment. CurrentPhantomOrder is a (H256, PhantomOrderInfo), so toJSON gives
// [commitment, info]. Every configured pair rides in that one order, so there is at most one.
async function getActivePhantomCommitments(api: ApiPromise): Promise<HexString[]> {
	const active = await api.query.intentsCoprocessor.currentPhantomOrder()
	const entry = active.toJSON() as [HexString, unknown] | null
	return Array.isArray(entry) ? [entry[0]] : []
}

// ─── anvil ──────────────────────────────────────────────────────────────────────────────────────

async function anvilRpc(method: string, params: unknown[]): Promise<void> {
	await fetch(ANVIL_URL, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ id: 1, jsonrpc: "2.0", method, params }),
	})
}

async function fundCngn(holder: HexString, amount: bigint): Promise<void> {
	const slot = keccak256(encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [holder, CNGN_BALANCE_SLOT]))
	await anvilRpc("anvil_setStorageAt", [CNGN_BASE, slot, toHex(amount, { size: 32 })])
}

// Writes the EIP-7702 delegation indicator our solvers carry in production, pointing at the real
// SolverAccount already deployed on the forked Base. aggregatePhantomBids only counts a bid whose
// sender delegates to it, so without this every filler's bid is (correctly) rejected as a stranger's.
async function delegateToSolverAccount(solver: HexString): Promise<void> {
	await anvilRpc("anvil_setCode", [solver, `0xef0100${SOLVER_ACCOUNT.slice(2)}`])
}

// ─── real IntentFiller bootstrap (mirrors createFxOnlyIntentFiller, redirected at simnode + anvil) ─

async function buildPhantomFiller(opts: {
	suri: string
	evmKey: HexString
	cngnPerUsd: string
}): Promise<{ filler: IntentFiller; solver: HexString; gateway: HexString }> {
	const resolvedChains: ResolvedChainConfig[] = [
		{ chainId: BASE_CHAIN_ID, rpcUrls: [ANVIL_URL], bundlerUrl: `${ANVIL_URL}/bundler` },
	]
	const serviceConfig: FillerServiceConfig = {
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: SIMNODE_URL,
		substratePrivateKey: opts.suri,
	}
	const configService = new FillerConfigService(resolvedChains, serviceConfig)
	const chainConfigs: ChainConfig[] = [configService.getChainConfig(BASE_STATE_MACHINE)]
	const fillerConfig: FillerConfig = {
		maxConcurrentOrders: 5,
		pendingQueueConfig: { maxRechecks: 10, recheckDelayMs: 30_000 },
	}

	const signer = await createSimplexSigner({ type: SignerType.PrivateKey, key: opts.evmKey })
	const chainClientManager = new ChainClientManager(configService, signer)
	const contractService = new ContractInteractionService(
		chainClientManager,
		configService,
		signer,
		new CacheService(),
	)

	// Phantom quoting uses the ask, so each filler's competitive rate stays
	// exactly opts.cngnPerUsd; the bid sits 2% above it because a crossed or
	// zero-spread book (bid ≤ ask) is rejected at construction.
	const askPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: opts.cngnPerUsd },
			{ amount: "10000", price: opts.cngnPerUsd },
		],
	})
	const bidPrice = (Number(opts.cngnPerUsd) * 1.02).toString()
	const bidPricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: bidPrice },
			{ amount: "10000", price: bidPrice },
		],
	})
	const legacy = exoticPairs(configService, { [BASE_STATE_MACHINE]: CNGN_BASE }, 5000, bidPricePolicy, askPricePolicy)
	const fxStrategy = new FXFiller(
		signer,
		configService,
		chainClientManager,
		contractService,
		legacy.pairs,
		legacy.registry,
	)

	const filler = new IntentFiller(
		chainConfigs,
		[fxStrategy],
		fillerConfig,
		configService,
		chainClientManager,
		contractService,
		signer,
		undefined,
		new BidStorageService(configService.getDataDir()),
	)
	await filler.initialize()
	filler.start()
	return {
		filler,
		solver: signer.account.address as HexString,
		// The gateway the filler targets in its fillOrder call — the aggregation must filter on the same one.
		gateway: configService.getIntentGatewayAddress(BASE_STATE_MACHINE) as HexString,
	}
}

// ─── test ───────────────────────────────────────────────────────────────────────────────────────

describe("Phantom filler E2E (real IntentFillers + simnode + anvil-forked Base)", () => {
	let api: ApiPromise
	let driver: IntentsCoprocessor
	let gateway: HexString
	const fillers: IntentFiller[] = []

	beforeAll(async () => {
		api = await ApiPromise.create({
			provider: new WsProvider(SIMNODE_URL),
			typesBundle: { spec: { gargantua: { hasher: keccakAsU8a } } },
		})
		driver = IntentsCoprocessor.fromApi(api, "//Alice")

		// Fund the dev accounts that aren't in genesis so they can reserve bid deposits.
		const keyring = new Keyring({ type: "sr25519" })
		const alice = keyring.addFromUri("//Alice")
		for (const suri of ["//Charlie", "//Dave"]) {
			const addr = keyring.addFromUri(suri).address
			await submitAndSeal(api, api.tx.balances.transferKeepAlive(addr, 10_000_000_000_000_000_000n), alice)
		}

		// Bid window must be shorter than the config's interval_blocks (10), enforced on-chain.
		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(5))
		await seedStateMachineHeight(api, BASE_CHAIN_ID, 1_000_000n)

		// Each solver needs forked cNGN liquidity — that is what they pay out and what the snapshot
		// records — and the delegation that marks its bid as ours rather than a stranger's.
		for (const f of FILLERS) {
			const solver = privateKeyToAccount(f.evmKey).address as HexString
			await fundCngn(solver, 1_000_000_000_000n)
			await delegateToSolverAccount(solver)
		}

		for (const f of FILLERS) {
			const { filler, gateway: gw } = await buildPhantomFiller(f)
			fillers.push(filler)
			gateway = gw
		}
	}, 180_000)

	afterAll(async () => {
		await Promise.all(fillers.map((f) => f.stop().catch(() => {})))
		await api?.disconnect()
	})

	it("real fillers watch + submit USDC→cNGN bids that the SDK aggregation reduces to a snapshot", async () => {
		// Register the phantom order; the fillers' subscriptions pick it up and submit bids.
		await setPhantomOrderConfig(api)
		await createBlock(api)

		const commitment = (await getActivePhantomCommitment(api))!
		expect(commitment).toBeTruthy()

		// Give the fillers time to fetch the order, quote, build the UserOp, and submit, then seal the
		// block that includes their bids.
		await new Promise((r) => setTimeout(r, 6_000))
		await createBlock(api)
		await new Promise((r) => setTimeout(r, 2_000))
		await createBlock(api)

		// Submission half: every filler's bid landed on-chain.
		const bids = await driver.getBidsForOrder(commitment)
		expect(bids.length).toBe(FILLERS.length)

		// Log each solver's quoted cNGN output (decoded from the submitted UserOp).
		const nodeUrl = SIMNODE_URL.replace(/^ws/, "http")
		const rawBids = await fetchBidsForOrder(nodeUrl, commitment)
		console.log(`\n[phantom-e2e] ${rawBids.length} bids for ${commitment}:`)
		for (const b of rawBids) {
			const decoded = decodeUserOpScale(b.user_op as HexString)
			const fd = extractFillData(decoded.callData as HexString, gateway)
			const quoted = fd?.legs.map((leg) => leg.solverAmount).join(", ")
			console.log(`[phantom-e2e]   solver ${decoded.sender} quoted ${quoted} cNGN`)
		}

		// Aggregation half: the SDK's aggregatePhantomBids (same code the indexer runs) measures each
		// solver's cNGN liquidity against the forked Base and reduces the quotes to a weighted snapshot.
		const result = await aggregatePhantomBids({
			nodeUrl,
			evmRpcUrls: { [BASE_STATE_MACHINE]: ANVIL_URL },
			chain: BASE_STATE_MACHINE,
			gatewayAddress: gateway,
			commitment,
			// Liquidity is swept per configured token per chain. cNGN with no vaults => raw balance
			// only (the funded amount); proves balances come from the config sweep, not the bid output.
			yieldVaults: { [BASE_STATE_MACHINE]: { [CNGN_BASE.toLowerCase()]: [] } },
			solverAccount: SOLVER_ACCOUNT,
		})

		console.log("\n[phantom-e2e] aggregation snapshot:")
		for (const leg of result?.legs ?? []) {
			console.log(`[phantom-e2e]   leg ${leg.pairIndex} bidCount:     ${leg.bidCount}`)
			console.log(`[phantom-e2e]   leg ${leg.pairIndex} lowestPrice:  ${leg.lowestPrice}`)
			console.log(`[phantom-e2e]   leg ${leg.pairIndex} medianPrice:  ${leg.medianPrice}  (liquidity-weighted)`)
			console.log(`[phantom-e2e]   leg ${leg.pairIndex} highestPrice: ${leg.highestPrice}`)
		}
		for (const lp of result?.lpBalances ?? []) {
			console.log(`[phantom-e2e]   LP ${lp.solver} on ${lp.chain} token ${lp.tokenAddress}: ${lp.balance}`)
		}

		expect(result).not.toBeNull()
		// The config prices a single pair, so the order carries one leg.
		expect(result!.legs).toHaveLength(1)
		const leg = result!.legs[0]
		expect(leg.bidCount).toBe(FILLERS.length)
		// Real cNGN quotes — guards against the fillers quoting 0 (e.g. the overfill cap collapsing
		// to the phantom order's zero requested output).
		expect(leg.lowestPrice).toBeGreaterThan(0n)
		expect(leg.lowestPrice).toBeLessThanOrEqual(leg.medianPrice)
		expect(leg.medianPrice).toBeLessThanOrEqual(leg.highestPrice)
		// One swept balance per filler: cNGN on Base (the single configured token/chain).
		expect(result!.lpBalances.length).toBe(FILLERS.length)
		for (const lp of result!.lpBalances) {
			expect(lp.chain).toBe(BASE_STATE_MACHINE)
			expect(lp.tokenAddress.toLowerCase()).toBe(CNGN_BASE.toLowerCase())
			expect(lp.balance).toBeGreaterThan(0n)
		}
	}, 180_000)

	it("quotes every pair of a multi pair phantom order in one bid", async () => {
		// Both directions of the pair (USDC→cNGN and cNGN→USDC) ride in a single order.
		await setBothPhantomPairs(api)
		await createBlock(api)

		const commitments = await getActivePhantomCommitments(api)
		expect(commitments.length).toBe(1)
		const commitment = commitments[0]

		// Let the fillers quote + submit their bids, then seal the blocks carrying them.
		await new Promise((r) => setTimeout(r, 6_000))
		await createBlock(api)
		await new Promise((r) => setTimeout(r, 2_000))
		await createBlock(api)

		const bids = await driver.getBidsForOrder(commitment)
		console.log(`[phantom-multi] ${bids.length} bids for ${commitment}`)
		expect(bids.length).toBe(FILLERS.length)

		// The point of bundling: one bid carries a quote per leg, so both directions get priced from
		// the single order rather than one of them being dropped.
		const nodeUrl = SIMNODE_URL.replace(/^ws/, "http")
		for (const bid of await fetchBidsForOrder(nodeUrl, commitment)) {
			const decoded = decodeUserOpScale(bid.user_op as HexString)
			const fillData = extractFillData(decoded.callData as HexString, gateway)
			expect(fillData).not.toBeNull()
			expect(fillData!.legs).toHaveLength(2)
			expect(fillData!.legs.every((leg) => leg.solverAmount > 0n)).toBe(true)
		}
	}, 180_000)
})
