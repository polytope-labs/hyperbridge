/**
 * Extended phantom-order E2E.
 *
 * Exercises the whole flow in one process:
 *   1. The hyperbridge runtime (simnode) generates a phantom order and emits PhantomOrderRegistered.
 *   2. Several in-process fillers WATCH for it (the same `subscribePhantomOrders` SDK path simplex
 *      uses), each builds a real `fillOrder` UserOp quoting a distinct output amount, and SUBMITS a
 *      bid via `submitBid` (again the SDK path simplex uses).
 *   3. The indexer's aggregation (`aggregatePhantomBids`) reads those bids via
 *      `intents_getBidsForOrder`, simulates each fill against a Base-mainnet fork (anvil, which
 *      supports eth_simulateV1), measures each solver's liquidity, and returns the snapshot — the
 *      same computation the SubQuery handler persists as PhantomOrderPriceSnapshot + LP balances.
 *
 * This test is GATED out of the default `jest` run (see jest.config testPathIgnorePatterns); run it
 * explicitly with `pnpm --filter @hyperbridge/indexer test:phantom-e2e`.
 *
 * Requires two services running locally:
 *   - a hyperbridge simnode (manual seal, NOT --instant):
 *       cargo build -p hyperbridge
 *       ./target/debug/hyperbridge simnode --chain gargantua-1000 --rpc-port 9990 --tmp \
 *         --rpc-methods=unsafe --rpc-cors=all --pool-type=single-state
 *   - anvil forking Base mainnet (provides the real IntentGateway/USDC + eth_simulateV1):
 *       anvil --fork-url https://base-mainnet.g.alchemy.com/v2/<KEY> --port 8545
 *
 * Override endpoints via SIMNODE_URL / ANVIL_URL.
 */
import { ApiPromise, WsProvider, Keyring } from "@polkadot/api"
import { keccakAsU8a } from "@polkadot/util-crypto"
import { encodeFunctionData, encodeAbiParameters, keccak256, toHex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { IntentsCoprocessor, encodeUserOpScale } from "@hyperbridge/sdk"
import type { HexString, PackedUserOperation, Order } from "@hyperbridge/sdk"
import { encodeERC7821ExecuteBatch, IntentGatewayV2 } from "@hyperbridge/sdk/intents-helpers"
import { INTENT_GATEWAY_V3_ADDRESSES } from "@/intent-gateway-v3-addresses"
import { aggregatePhantomBids } from "@/handlers/events/substrateChains/phantom-aggregation"

const SIMNODE_URL = process.env.SIMNODE_URL || "ws://127.0.0.1:9990"
const ANVIL_URL = process.env.ANVIL_URL || "http://127.0.0.1:8545"

// Base mainnet (forked by anvil). Same token for input and output keeps the probe simple: the
// solver only needs a balance of one token (USDC) and the sim's solver→solver transfer validates it.
const BASE_CHAIN_ID = 8453
const BASE_STATE_MACHINE = "EVM-8453"
const USDC_BASE = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" as HexString
const USDC_BALANCE_SLOT = 9n // Circle FiatToken _balances slot on Base (see TOKEN_SLOT_OVERRIDES)
const ETH0_CONSENSUS_ID = "0x45544830"

const STANDARD_AMOUNT = 1_000_000_000n // 1,000 USDC (6 decimals)

// Three fillers, each with a distinct EVM solver account and a distinct quoted output amount so the
// aggregation has a real range to reduce. Substrate keys come from the dev keyring.
const FILLERS = [
	{ suri: "//Bob", key: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" as HexString, price: 1_000_000_000n },
	{ suri: "//Charlie", key: "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a" as HexString, price: 1_010_000_000n },
	{ suri: "//Dave", key: "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6" as HexString, price: 1_020_000_000n },
]

// ─── substrate helpers (mirrors phantom-e2e.simnode.test.ts) ────────────────────────────────────

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
	const value = api.createType("u64", height).toHex()
	await sudoAndSeal(api, api.tx.system.setStorage([[key, value]]))
}

async function setPhantomOrderConfig(api: ApiPromise): Promise<void> {
	const config = {
		chain: { state_id: { Evm: BASE_CHAIN_ID }, consensus_state_id: ETH0_CONSENSUS_ID },
		token_pairs: [{ token_a: USDC_BASE, token_b: USDC_BASE, standard_amount: STANDARD_AMOUNT }],
		interval_blocks: 10,
	}
	await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomOrderConfig(config))
}

// ─── anvil helper ───────────────────────────────────────────────────────────────────────────────

// Gives `holder` a USDC balance on the fork by writing the FiatToken _balances slot directly.
async function fundUsdc(holder: HexString, amount: bigint): Promise<void> {
	const slot = keccak256(
		encodeAbiParameters([{ type: "address" }, { type: "uint256" }], [holder, USDC_BALANCE_SLOT]),
	)
	await fetch(ANVIL_URL, {
		method: "POST",
		headers: { "content-type": "application/json" },
		body: JSON.stringify({
			id: 1,
			jsonrpc: "2.0",
			method: "anvil_setStorageAt",
			params: [USDC_BASE, slot, toHex(amount, { size: 32 })],
		}),
	})
}

// ─── bid construction (the UserOp the indexer decodes; never executed on-chain) ─────────────────

function buildPhantomBidUserOp(solver: HexString, order: Order, gateway: HexString, price: bigint): HexString {
	const outputToken = order.output.assets[0].token
	const fillOptions = { relayerFee: 0n, nativeDispatchFee: 0n, outputs: [{ token: outputToken, amount: price }] }
	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	const fillCalldata = (encodeFunctionData as any)({
		abi: IntentGatewayV2.ABI,
		functionName: "fillOrder",
		args: [order, fillOptions],
	}) as HexString
	const callData = encodeERC7821ExecuteBatch([{ target: gateway, value: 0n, data: fillCalldata }])
	const userOp: PackedUserOperation = {
		sender: solver,
		nonce: 0n,
		initCode: "0x",
		callData,
		accountGasLimits: "0x00000000000000000000000000007530000000000000000000000000000f4240",
		preVerificationGas: 21000n,
		gasFees: "0x00000000000000000000000000000001000000000000000000000000000f4240",
		paymasterAndData: "0x",
		signature: "0x",
	}
	return encodeUserOpScale(userOp)
}

async function getActivePhantomCommitment(api: ApiPromise): Promise<HexString | null> {
	const key = api.query.intentsCoprocessor.currentPhantomOrder.key()
	const raw: any = await api.rpc.state.getStorage(key)
	if (!raw) return null
	const hex: string = raw.toHex()
	if (!hex || hex === "0x" || hex.length < 68) return null
	return `0x${hex.slice(4, 68)}` as HexString
}

// ─── test ───────────────────────────────────────────────────────────────────────────────────────

describe("Phantom indexer E2E (simnode + anvil-forked Base)", () => {
	let api: ApiPromise
	let alice: IntentsCoprocessor
	const gateway = INTENT_GATEWAY_V3_ADDRESSES[BASE_STATE_MACHINE as keyof typeof INTENT_GATEWAY_V3_ADDRESSES] as HexString

	beforeAll(async () => {
		api = await ApiPromise.create({
			provider: new WsProvider(SIMNODE_URL),
			typesBundle: { spec: { gargantua: { hasher: keccakAsU8a } } },
		})
		alice = IntentsCoprocessor.fromApi(api, "//Alice")

		// Fund Charlie/Dave on the simnode so they can reserve bid deposits.
		const keyring = new Keyring({ type: "sr25519" })
		const aliceKey = keyring.addFromUri("//Alice")
		for (const suri of ["//Charlie", "//Dave"]) {
			const addr = keyring.addFromUri(suri).address
			await submitAndSeal(api, api.tx.balances.transferKeepAlive(addr, 10_000_000_000_000_000_000n), aliceKey)
		}

		await sudoAndSeal(api, api.tx.intentsCoprocessor.setPhantomBidWindow(100))
		await seedStateMachineHeight(api, BASE_CHAIN_ID, 1_000_000n)

		// Each solver needs a real (forked) USDC balance — that is the liquidity the snapshot records
		// and the balance the fill simulation's solver→solver transfer checks.
		for (const f of FILLERS) {
			const solver = privateKeyToAccount(f.key).address as HexString
			await fundUsdc(solver, 5_000_000_000n) // 5,000 USDC each (equal weights)
		}
	}, 120_000)

	afterAll(async () => {
		await api?.disconnect()
	})

	it("registers a phantom order, collects multi-filler bids, and aggregates a snapshot", async () => {
		// 1. Generate the phantom order and capture its commitment via the SDK watch path.
		let watched: HexString | null = null
		const unsub = await alice.subscribePhantomOrders((e) => {
			if (e.chain === BASE_STATE_MACHINE) watched = e.commitment
		})

		await setPhantomOrderConfig(api)
		await createBlock(api) // on_initialize generates + emits PhantomOrderRegistered

		const commitment = (await getActivePhantomCommitment(api))!
		expect(commitment).toBeTruthy()
		expect(watched).toBe(commitment) // simplex's watch path saw it

		// 2. Each filler fetches the order, quotes a distinct price, and submits a real fillOrder bid.
		const order = (await alice.fetchPhantomOrder(commitment))!
		expect(order).toBeTruthy()

		await Promise.all(
			FILLERS.map(async (f) => {
				const solver = privateKeyToAccount(f.key).address as HexString
				const userOp = buildPhantomBidUserOp(solver, order, gateway, f.price)
				const filler = IntentsCoprocessor.fromApi(api, f.suri)
				return filler.submitBid(commitment, userOp)
			}),
		)
		await new Promise((r) => setTimeout(r, 400))
		await createBlock(api)
		unsub()

		const bids = await alice.getBidsForOrder(commitment)
		expect(bids.length).toBe(FILLERS.length)

		// 3. Run the indexer aggregation against the simnode + the anvil-forked Base.
		const nodeUrl = SIMNODE_URL.replace(/^ws/, "http")
		const result = await aggregatePhantomBids({
			nodeUrl,
			evmRpcUrl: ANVIL_URL,
			chain: BASE_STATE_MACHINE,
			gatewayAddress: gateway,
			commitment,
			inputToken: order.inputs[0].token,
			standardAmount: STANDARD_AMOUNT,
		})

		expect(result).not.toBeNull()
		expect(result!.bidCount).toBe(FILLERS.length)
		expect(result!.lowestPrice).toBe(1_000_000_000n)
		expect(result!.highestPrice).toBe(1_020_000_000n)
		// Equal weights → lower weighted median = middle quote.
		expect(result!.medianPrice).toBe(1_010_000_000n)
		expect(result!.lpBalances.length).toBe(FILLERS.length)
		for (const lp of result!.lpBalances) {
			expect(lp.tokenAddress.toLowerCase()).toBe(USDC_BASE.toLowerCase())
			expect(lp.balance).toBeGreaterThanOrEqual(5_000_000_000n)
		}
	}, 180_000)
})
