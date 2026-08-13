/**
 * Extended phantom-order E2E — exercises the REAL simplex IntentFiller end to end.
 *
 * Several real `IntentFiller` instances (one per solver, each with its own FX price policy) connect
 * to the simnode, WATCH for the phantom orders via their own phantom-bidding subscription, quote
 * every leg with the FX strategy, build a fillOrder UserOp through the full
 * ContractInteractionService pipeline, and submit a bid per chain — i.e. the complete simplex
 * bid-submission path, not a hand-rolled UserOp. The pallet is configured with TWO chains carrying
 * TWO token pairs each, so the test asserts the whole surface: per-chain orders, per-leg quotes,
 * per-leg liquidity weights, and the accepted-source route declarations.
 *
 * The second chain is another anvil fork of Base whose chain id is overridden to Optimism's. That
 * works because the protocol contracts (IntentGateway, SolverAccount, EntryPoint) are deployed via
 * CREATE2 at the same address on every chain, so the forked Base state has them where the EVM-10
 * config expects them; the token addresses are overridden per chain in the test's AssetRegistry.
 *
 * Gated out of the default run (`*.simnode.test.ts`); run with `pnpm --filter @hyperbridge/simplex test:phantom-filler-e2e`.
 *
 * Requires:
 *   - a hyperbridge simnode (manual seal):
 *       ./target/debug/hyperbridge simnode --chain gargantua-1000 --rpc-port 9990 --tmp \
 *         --rpc-methods=unsafe --rpc-cors=all --pool-type=single-state
 *   - anvil forking Base mainnet (real IntentGateway + USDC + cNGN + eth_simulateV1):
 *       anvil --fork-url https://base-mainnet.g.alchemy.com/v2/<KEY> --port 8545
 *   - a second anvil forking Base presented as EVM-10:
 *       anvil --fork-url https://base-mainnet.g.alchemy.com/v2/<KEY> --port 8546 --chain-id 10
 *
 * Override endpoints via SIMNODE_URL / ANVIL_URL / ANVIL2_URL.
 */
import { stubOrderStream } from "./helpers/stub-stream"
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api"
import { hexToU8a, u8aToString } from "@polkadot/util"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { encodeAbiParameters, keccak256, toHex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { describe, it, expect, beforeAll, afterAll } from "vitest"
import { IntentFiller } from "@/core/filler"
import {
	CacheService,
	ChainClientManager,
	ContractInteractionService,
	FillerConfigService,
	type ResolvedChainConfig,
	type FillerConfig as FillerServiceConfig,
} from "@/services"
import { SqliteDataStore } from "@/data/sqlite"
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

/**
 * Builds the exotic-pair set + registry for the fillers: cNGN traded against USDC and USDT on
 * both chains. The EVM-10 fork carries Base state, so its tokens live at the Base addresses —
 * the user asset layer overrides the SDK registry's real Optimism addresses per chain.
 */
function exoticPairs(
	resolver: FillerConfigService,
	maxOrderSize: number,
	bidPricePolicy?: FillerPricePolicy,
	askPricePolicy?: FillerPricePolicy,
): { pairs: TradingPair[]; registry: AssetRegistry } {
	const registry = new AssetRegistry(resolver, {
		EXOTIC: { [BASE_STATE_MACHINE]: CNGN_BASE, [CHAIN2_STATE_MACHINE]: CNGN_BASE },
		USDC: { [CHAIN2_STATE_MACHINE]: USDC_BASE },
		USDT: { [CHAIN2_STATE_MACHINE]: USDT_BASE },
	})
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
const ANVIL2_URL = process.env.ANVIL2_URL || "http://127.0.0.1:8546"

const BASE_STATE_MACHINE = "EVM-8453"
const BASE_CHAIN_ID = 8453
const CHAIN2_STATE_MACHINE = "EVM-10"
const CHAIN2_CHAIN_ID = 10
const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" as HexString
const USDT_BASE = "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2" as HexString
const CNGN_BASE = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F" as HexString
const CNGN_BALANCE_SLOT = 201n // cNGN _balances slot on Base (see indexer TOKEN_SLOT_OVERRIDES)
const USDC_BALANCE_SLOT = 9n // FiatTokenProxy layout (see chain.ts tokenStorageSlots)
const USDT_BALANCE_SLOT = 0n
const ETH0_CONSENSUS_ID = "0x45544830"
const STANDARD_AMOUNT = 1_000_000n // 1 whole unit of any of the three 6-decimal tokens
// The real SolverAccount, already deployed in the forks; solvers delegate to it and the
// aggregation only counts bids from senders that do. CREATE2 puts it at the same address on
// every chain, which is also what lets the EVM-10 fork reuse Base state.
const SOLVER_ACCOUNT = new ChainConfigService().getSolverAccountAddress(BASE_STATE_MACHINE)!

// Both anvil forks carry identical Base state, so token addresses and balance slots are the same
// on each; what differs is the chain id the fork reports and the state machine id the pallet and
// fillers use for it.
const CHAINS = [
	{ stateMachine: BASE_STATE_MACHINE, chainId: BASE_CHAIN_ID, anvilUrl: ANVIL_URL },
	{ stateMachine: CHAIN2_STATE_MACHINE, chainId: CHAIN2_CHAIN_ID, anvilUrl: ANVIL2_URL },
]

// One IntentFiller per solver. Distinct substrate keys (to place independent bids) and EVM keys
// (distinct solver addresses/liquidity), and distinct FX prices so there is a real range to reduce.
// //Alice is reserved for the driver's sudo/sealing, so fillers use other dev accounts to avoid
// nonce contention.
//
// Each filler declares a different accepted-source-chains state so the aggregation must surface
// all three route semantics per bidder: an explicit list, an explicit empty list (accepts
// nothing), and no declaration at all (null — the legacy "all covered chains" default).
const FILLERS = [
	{
		suri: "//Bob",
		evmKey: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString,
		cngnPerUsd: "1500",
		declaredSources: ["EVM-1", "EVM-8453"] as string[] | undefined,
	},
	{
		suri: "//Charlie",
		evmKey: "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" as HexString,
		cngnPerUsd: "1510",
		declaredSources: [] as string[] | undefined,
	},
	{
		suri: "//Dave",
		evmKey: "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6" as HexString,
		cngnPerUsd: "1520",
		declaredSources: undefined as string[] | undefined,
	},
]

// What decodeAcceptedSourceChains must yield for each solver's bid: the declared list survives
// the ride verbatim, an empty declaration stays [] (not null), and no declaration decodes to null.
const EXPECTED_SOURCES = new Map(
	FILLERS.map((f) => [
		privateKeyToAccount(f.evmKey).address.toLowerCase(),
		f.declaredSources ?? null,
	]),
)

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
// Two pairs per chain: both stables against cNGN. Each pair expands into its forward and
// reverse legs, so every chain's order carries four legs.
async function setPhantomOrderConfig(api: ApiPromise): Promise<void> {
	const tokenPairs = [USDC_BASE, USDT_BASE].map((stable) => ({
		token_a: stable,
		token_b: CNGN_BASE,
		standard_amount: STANDARD_AMOUNT,
		// 1 cNGN (6 decimals) — the reverse leg's benchmark quantity.
		standard_amount_b: 1_000_000n,
	}))
	// `chains` maps each state machine id to its own token pairs; the interval is shared.
	const config = {
		chains: new Map(
			CHAINS.map((c) => [{ state_id: { Evm: c.chainId }, consensus_state_id: ETH0_CONSENSUS_ID }, tokenPairs]),
		),
		interval_blocks: 10,
	}
	await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomOrderConfig(config))
}
/**
 * The active phantom commitments keyed by state machine id ("EVM-8453"), decoded off the runtime
 * metadata: `CurrentPhantomOrder` holds one `(commitment, info)` entry per configured chain.
 */
async function getActivePhantomCommitments(api: ApiPromise): Promise<Map<string, HexString>> {
	const raw: any = await api.query.intentsCoprocessor.currentPhantomOrder()
	const entries = (raw?.toJSON() as [string, { chain: string }][] | null) ?? []
	const byChain = new Map<string, HexString>()
	for (const [commitment, info] of entries) {
		byChain.set(u8aToString(hexToU8a(info.chain)), commitment as HexString)
	}
	return byChain
}

// ─── anvil ──────────────────────────────────────────────────────────────────────────────────────

async function anvilRpc(anvilUrl: string, method: string, params: unknown[]): Promise<void> {
	await fetch(anvilUrl, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({ id: 1, jsonrpc: "2.0", method, params }),
	})
}

async function fundToken(
	anvilUrl: string,
	token: HexString,
	balanceSlot: bigint,
	holder: HexString,
	amount: bigint,
): Promise<void> {
	const slot = keccak256(encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [holder, balanceSlot]))
	await anvilRpc(anvilUrl, "anvil_setStorageAt", [token, slot, toHex(amount, { size: 32 })])
}

// Writes the EIP-7702 delegation indicator our solvers carry in production, pointing at the real
// SolverAccount already deployed on the forked Base. aggregatePhantomBids only counts a bid whose
// sender delegates to it, so without this every filler's bid is (correctly) rejected as a stranger's.
async function delegateToSolverAccount(anvilUrl: string, solver: HexString): Promise<void> {
	await anvilRpc(anvilUrl, "anvil_setCode", [solver, `0xef0100${SOLVER_ACCOUNT.slice(2)}`])
}

// ─── real IntentFiller bootstrap (mirrors createFxOnlyIntentFiller, redirected at simnode + anvil) ─

async function buildPhantomFiller(opts: {
	suri: string
	evmKey: HexString
	cngnPerUsd: string
	declaredSources: string[] | undefined
}): Promise<{ filler: IntentFiller; solver: HexString; gateway: HexString }> {
	const resolvedChains: ResolvedChainConfig[] = CHAINS.map((c) => ({
		chainId: c.chainId,
		rpcUrls: [c.anvilUrl],
		bundlerUrl: `${c.anvilUrl}/bundler`,
	}))
	const serviceConfig: FillerServiceConfig = {
		maxConcurrentOrders: 5,
		hyperbridgeWsUrl: SIMNODE_URL,
		substratePrivateKey: opts.suri,
	}
	const configService = new FillerConfigService(resolvedChains, serviceConfig)
	const chainConfigs: ChainConfig[] = CHAINS.map((c) => configService.getChainConfig(c.stateMachine))
	const fillerConfig: FillerConfig = {
		maxConcurrentOrders: 5,
		pendingQueueConfig: { maxRechecks: 10, recheckDelayMs: 30_000 },
		acceptedSourceChains: opts.declaredSources,
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
	const legacy = exoticPairs(configService, 5000, bidPricePolicy, askPricePolicy)
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
		{ orders: stubOrderStream() },
		undefined,
		new SqliteDataStore(configService.getDataDir() || ".simplex-data").bids,
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
		for (const c of CHAINS) {
			await seedStateMachineHeight(api, c.chainId, 1_000_000n)
		}

		// Each solver needs forked liquidity in every leg's output token on every chain — that is
		// what they pay out and what the snapshot weighs — and the delegation that marks its bid
		// as ours rather than a stranger's.
		for (const c of CHAINS) {
			for (const f of FILLERS) {
				const solver = privateKeyToAccount(f.evmKey).address as HexString
				await fundToken(c.anvilUrl, CNGN_BASE, CNGN_BALANCE_SLOT, solver, 1_000_000_000_000n)
				await fundToken(c.anvilUrl, USDC_BASE, USDC_BALANCE_SLOT, solver, 1_000_000_000_000n)
				await fundToken(c.anvilUrl, USDT_BASE, USDT_BALANCE_SLOT, solver, 1_000_000_000_000n)
				await delegateToSolverAccount(c.anvilUrl, solver)
			}
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
	}, 60_000)

	/**
	 * Seals blocks until every commitment has `expected` bids or the deadline
	 * passes. The fill pipeline (offchain-storage fetch → quote against the
	 * fork → UserOp build → submit) is bounded by upstream RPC latency, so a
	 * fixed sleep races it; polling keeps the assertions strict without the race.
	 */
	async function waitForBids(commitments: HexString[], expected: number, timeoutMs = 120_000): Promise<void> {
		const deadline = Date.now() + timeoutMs
		for (;;) {
			await new Promise((r) => setTimeout(r, 2_000))
			await createBlock(api)
			const counts = await Promise.all(commitments.map((c) => driver.getBidsForOrder(c).then((b) => b.length)))
			if (counts.every((n) => n >= expected) || Date.now() > deadline) return
		}
	}

	it("real fillers bid on every chain's order and the SDK aggregation reduces each to a snapshot", async () => {
		// Register the phantom orders; the fillers' subscriptions pick them up and submit one bid
		// per chain.
		await setPhantomOrderConfig(api)
		await createBlock(api)

		const commitments = await getActivePhantomCommitments(api)
		expect([...commitments.keys()].sort()).toEqual(CHAINS.map((c) => c.stateMachine).sort())

		// Seal blocks until every filler's bid lands for every chain's order (or
		// the deadline passes and the assertions below report the shortfall).
		await waitForBids([...commitments.values()], FILLERS.length)

		const nodeUrl = SIMNODE_URL.replace(/^ws/, "http")
		// Two pairs per chain expand into four positional legs, in config order; the output token
		// of each is what that leg's liquidity is measured in.
		const expectedOutputs = [CNGN_BASE, USDC_BASE, CNGN_BASE, USDT_BASE].map((t) => t.toLowerCase())

		for (const c of CHAINS) {
			const commitment = commitments.get(c.stateMachine)!

			// Submission half: every filler's bid landed on-chain for this chain's order.
			const bids = await driver.getBidsForOrder(commitment)
			expect(bids.length).toBe(FILLERS.length)

			// Aggregation half: the SDK's aggregatePhantomBids (same code the indexer runs) measures
			// each solver's liquidity against this chain's fork and reduces the quotes to a snapshot.
			const result = await aggregatePhantomBids({
				nodeUrl,
				evmRpcUrls: { [c.stateMachine]: c.anvilUrl },
				chain: c.stateMachine,
				gatewayAddress: gateway,
				commitment,
				// Liquidity is swept per configured token per chain, with no vaults => raw balances
				// only (the funded amounts); proves balances come from the config sweep, not the
				// bid output.
				yieldVaults: {
					[c.stateMachine]: {
						[CNGN_BASE.toLowerCase()]: [],
						[USDC_BASE.toLowerCase()]: [],
						[USDT_BASE.toLowerCase()]: [],
					},
				},
				solverAccount: SOLVER_ACCOUNT,
			})

			console.log(`\n[phantom-e2e] ${c.stateMachine} aggregation snapshot for ${commitment}:`)
			for (const leg of result?.legs ?? []) {
				console.log(
					`[phantom-e2e]   leg ${leg.legIndex} bids ${leg.bidCount} prices ${leg.lowestPrice}/${leg.medianPrice}/${leg.highestPrice}`,
				)
			}
			for (const lp of result?.lpBalances ?? []) {
				console.log(`[phantom-e2e]   LP ${lp.solver} token ${lp.tokenAddress}: ${lp.balance}`)
			}

			expect(result).not.toBeNull()
			// Both configured pairs expand into forward and reverse legs, positionally.
			expect(result!.legs).toHaveLength(4)
			for (const leg of result!.legs) {
				expect(leg.bidCount).toBe(FILLERS.length)
				// Real quotes on every leg — guards against the fillers quoting 0 (e.g. the overfill
				// cap collapsing to the phantom order's zero requested output).
				expect(leg.lowestPrice).toBeGreaterThan(0n)
				expect(leg.lowestPrice).toBeLessThanOrEqual(leg.medianPrice)
				expect(leg.medianPrice).toBeLessThanOrEqual(leg.highestPrice)
				// Every filler's per-leg weight is its funded balance in that leg's output token.
				expect(leg.bidders).toHaveLength(FILLERS.length)
				for (const bidder of leg.bidders) {
					expect(bidder.weight).toBeGreaterThan(0n)
					// Route indexing: each solver's accepted-source declaration rode inside its
					// bid's paymasterAndData — covered by the solver's signature — and survived
					// SCALE encoding and aggregation with all three semantics intact (explicit
					// list, explicit empty list, null for no declaration).
					expect(EXPECTED_SOURCES.has(bidder.solver.toLowerCase())).toBe(true)
					expect(bidder.acceptedSources).toEqual(EXPECTED_SOURCES.get(bidder.solver.toLowerCase()))
				}
			}
			// One swept balance per filler per configured token on this chain.
			expect(result!.lpBalances.length).toBe(FILLERS.length * 3)
			for (const lp of result!.lpBalances) {
				expect(lp.chain).toBe(c.stateMachine)
				expect(expectedOutputs).toContain(lp.tokenAddress.toLowerCase())
				expect(lp.balance).toBeGreaterThan(0n)
			}
		}
	}, 300_000)

	it("quotes every leg of each chain's expanded phantom order in one bid", async () => {
		// Two configured pairs per chain; the pallet expands them into four legs per order.
		await setPhantomOrderConfig(api)
		await createBlock(api)

		const commitments = await getActivePhantomCommitments(api)
		expect(commitments.size).toBe(CHAINS.length)

		// Seal blocks until the fillers' bids land for every chain's order.
		await waitForBids([...commitments.values()], FILLERS.length)

		const nodeUrl = SIMNODE_URL.replace(/^ws/, "http")
		for (const c of CHAINS) {
			const commitment = commitments.get(c.stateMachine)!
			const bids = await driver.getBidsForOrder(commitment)
			console.log(`[phantom-multi] ${c.stateMachine}: ${bids.length} bids for ${commitment}`)
			expect(bids.length).toBe(FILLERS.length)

			// The point of bundling: one bid carries a quote per leg, so every pair gets priced in
			// both directions from the single order rather than any of them being dropped.
			for (const bid of await fetchBidsForOrder(nodeUrl, commitment)) {
				const decoded = decodeUserOpScale(bid.user_op as HexString)
				const fillData = extractFillData(decoded.callData as HexString, gateway)
				expect(fillData).not.toBeNull()
				expect(fillData!.legs).toHaveLength(4)
				expect(fillData!.legs.every((leg) => leg.solverAmount > 0n)).toBe(true)
			}
		}
	}, 300_000)
})
