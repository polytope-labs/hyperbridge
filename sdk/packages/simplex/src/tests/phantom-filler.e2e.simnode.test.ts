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
import { FXFiller } from "@/strategies/fx"
import { FillerPricePolicy } from "@/config/interpolated-curve"
import { IntentsCoprocessor, type ChainConfig, type FillerConfig, type HexString } from "@hyperbridge/sdk"
import {
	aggregatePhantomBids,
	fetchBidsForOrder,
	decodeUserOpScale,
	extractFillData,
	type TokenSlotOverrides,
} from "@hyperbridge/sdk/intents-helpers"

const SIMNODE_URL = process.env.SIMNODE_URL || "ws://127.0.0.1:9990"
const ANVIL_URL = process.env.ANVIL_URL || "http://127.0.0.1:8545"

const BASE_STATE_MACHINE = "EVM-8453"
const BASE_CHAIN_ID = 8453
const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" as HexString
const CNGN_BASE = "0x46C85152bFe9f96829aA94755D9f915F9B10EF5F" as HexString
const CNGN_BALANCE_SLOT = 201n // cNGN _balances slot on Base (see indexer TOKEN_SLOT_OVERRIDES)
const ETH0_CONSENSUS_ID = "0x45544830"
const STANDARD_AMOUNT = 1_000_000n // 1 USDC (6 decimals)

// One IntentFiller per solver. Distinct substrate keys (to place independent bids) and EVM keys
// (distinct solver addresses/liquidity), and distinct FX prices so there is a real range to reduce.
// //Alice is reserved for the driver's sudo/sealing, so fillers use other dev accounts to avoid
// nonce contention.
const FILLERS = [
	{ suri: "//Bob", evmKey: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString, cngnPerUsd: "1500" },
	{ suri: "//Charlie", evmKey: "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" as HexString, cngnPerUsd: "1510" },
	{ suri: "//Dave", evmKey: "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6" as HexString, cngnPerUsd: "1520" },
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
	if (!hex || hex === "0x" || hex.length < 68) return null
	return `0x${hex.slice(4, 68)}` as HexString
}

// ─── anvil ──────────────────────────────────────────────────────────────────────────────────────

async function fundCngn(holder: HexString, amount: bigint): Promise<void> {
	const slot = keccak256(encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [holder, CNGN_BALANCE_SLOT]))
	await fetch(ANVIL_URL, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({
			id: 1,
			jsonrpc: "2.0",
			method: "anvil_setStorageAt",
			params: [CNGN_BASE, slot, toHex(amount, { size: 32 })],
		}),
	})
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
	const contractService = new ContractInteractionService(chainClientManager, configService, signer, new CacheService())

	const pricePolicy = new FillerPricePolicy({
		points: [
			{ amount: "1", price: opts.cngnPerUsd },
			{ amount: "10000", price: opts.cngnPerUsd },
		],
	})
	const fxStrategy = new FXFiller(
		signer,
		configService,
		chainClientManager,
		contractService,
		5000,
		{ [BASE_STATE_MACHINE]: CNGN_BASE },
		{ bidPricePolicy: pricePolicy, askPricePolicy: pricePolicy },
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

// Minimal slot map for the two tokens this test touches (Base USDC + cNGN), in place of the
// indexer's generated TOKEN_SLOT_OVERRIDES.
const TEST_TOKEN_SLOTS: TokenSlotOverrides = {
	[USDC_BASE.toLowerCase()]: { balanceSlot: 9n, allowanceSlot: 10n },
	[CNGN_BASE.toLowerCase()]: { balanceSlot: 201n, allowanceSlot: 202n },
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

		// Each solver needs forked cNGN liquidity — that is what they pay out and what the snapshot records.
		for (const f of FILLERS) {
			await fundCngn(privateKeyToAccount(f.evmKey).address as HexString, 1_000_000_000_000n)
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
			console.log(`[phantom-e2e]   solver ${decoded.sender} quoted ${fd?.solverAmount} cNGN`)
		}

		// Aggregation half: the indexer's logic (shared from the SDK) simulates each fill against the
		// forked Base, measures cNGN liquidity, and reduces the quotes to a snapshot.
		const result = await aggregatePhantomBids({
			nodeUrl,
			evmRpcUrl: ANVIL_URL,
			chain: BASE_STATE_MACHINE,
			gatewayAddress: gateway,
			commitment,
			inputToken: USDC_BASE,
			standardAmount: STANDARD_AMOUNT,
			tokenSlotOverrides: TEST_TOKEN_SLOTS,
			yieldVaults: {}, // raw cNGN balance only; no vaults funded in this test
		})

		console.log("\n[phantom-e2e] aggregation snapshot:")
		console.log(`[phantom-e2e]   bidCount:     ${result?.bidCount}`)
		console.log(`[phantom-e2e]   lowestPrice:  ${result?.lowestPrice}`)
		console.log(`[phantom-e2e]   medianPrice:  ${result?.medianPrice}  (liquidity-weighted)`)
		console.log(`[phantom-e2e]   highestPrice: ${result?.highestPrice}`)
		for (const lp of result?.lpBalances ?? []) {
			console.log(`[phantom-e2e]   LP ${lp.solver}: ${lp.balance} cNGN`)
		}

		expect(result).not.toBeNull()
		expect(result!.bidCount).toBe(FILLERS.length)
		// Real cNGN quotes — guards against the fillers quoting 0 (e.g. the overfill cap collapsing
		// to the phantom order's zero requested output).
		expect(result!.lowestPrice).toBeGreaterThan(0n)
		expect(result!.lowestPrice).toBeLessThanOrEqual(result!.medianPrice)
		expect(result!.medianPrice).toBeLessThanOrEqual(result!.highestPrice)
		expect(result!.lpBalances.length).toBe(FILLERS.length)
		for (const lp of result!.lpBalances) {
			expect(lp.tokenAddress.toLowerCase()).toBe(CNGN_BASE.toLowerCase())
			expect(lp.balance).toBeGreaterThan(0n)
		}
	}, 180_000)
})
