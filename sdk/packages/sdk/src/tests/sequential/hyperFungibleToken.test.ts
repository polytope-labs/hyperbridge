import { describe, it, expect } from "vitest"
import { createPublicClient, createWalletClient, http, parseEther, encodeFunctionData, getContract, type Hex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet, polygonAmoy } from "viem/chains"
import { EvmChain } from "@/chains/evm"
import { SubstrateChain } from "@/chains/substrate"
import { IsmpClient } from "@/client"
import { createQueryClient } from "@/queryClient"
import { HyperFungibleToken, type BridgeParams, type QuoteResult } from "@/protocols/hyperFungibleToken"
import EVM_HOST from "@/abis/evmHost"
import type { HexString } from "@/types"

// WrappedHFT wrapping WBNB on BSC testnet (lock/unlock)
const BSC_WRAPPED_HFT = "0x56a77F44a08cf357F59Cc3ae3de7aDfDFaa973d8" as const
// HFT on Polygon Amoy (burn/mint) — paired with BSC WrappedHFT
const POLYGON_HFT = "0xa0D8d6E104b92113c7E2815e970cb5626270E8c1" as const

const BSC_HOST = "0xEB944071A9Bf22810757C5BcFf7a2aE9663a311D" as const
const POLYGON_HOST = "0xEB944071A9Bf22810757C5BcFf7a2aE9663a311D" as const

// TokenFaucet addresses (drips 1000 fee tokens per day)
const BSC_FAUCET = "0xcb00f5b86aac5e2fdca9dc7f34d9bfe00b967c18" as const
const POLYGON_FAUCET = "0xcb00f5b86aac5e2fdca9dc7f34d9bfe00b967c18" as const

const FAUCET_ABI = [
	{ type: "function", name: "drip", inputs: [{ name: "token", type: "address" }], outputs: [], stateMutability: "nonpayable" },
] as const

const PRIVATE_KEY = process.env.PRIVATE_KEY as HexString
const BSC_RPC = process.env.BSC_CHAPEL!
const POLYGON_RPC = process.env.POLYGON_AMOY!

function createBscToPolygon() {
	const source = EvmChain.fromParams({
		chainId: 97,
		rpcUrl: BSC_RPC,
		host: BSC_HOST,
		consensusStateId: "BSC0",
	})

	const dest = EvmChain.fromParams({
		chainId: 80002,
		rpcUrl: POLYGON_RPC,
		host: POLYGON_HOST,
		consensusStateId: "POLY",
	})

	return { source, dest }
}

function createPolygonToBsc() {
	const source = EvmChain.fromParams({
		chainId: 80002,
		rpcUrl: POLYGON_RPC,
		host: POLYGON_HOST,
		consensusStateId: "POLY",
	})

	const dest = EvmChain.fromParams({
		chainId: 97,
		rpcUrl: BSC_RPC,
		host: BSC_HOST,
		consensusStateId: "BSC0",
	})

	return { source, dest }
}

async function createIsmpClient(source: EvmChain, dest: EvmChain) {
	console.log("[createIsmpClient] connecting to Hyperbridge:", process.env.HYPERBRIDGE_GARGANTUA)
	const hyperbridge = await SubstrateChain.connect({
		wsUrl: process.env.HYPERBRIDGE_GARGANTUA || "wss://gargantua.rpc.polytope.technology",
		consensusStateId: "PAS0",
		hasher: "Keccak",
		stateMachineId: "KUSAMA-4009",
	})
	console.log("[createIsmpClient] connected to Hyperbridge")

	console.log("[createIsmpClient] creating query client")
	const queryClient = createQueryClient({
		url: process.env.INDEXER_URL || "https://gargantua.indexer.polytope.technology",
	})

	console.log("[createIsmpClient] constructing IsmpClient")
	const ismpClient = new IsmpClient({
		queryClient,
		source,
		dest,
		hyperbridge,
		pollInterval: 5_000,
	})
	console.log("[createIsmpClient] done")

	return { ismpClient, hyperbridge }
}

/**
 * Ensures the account has enough fee tokens on the source chain by calling the TokenFaucet.
 * The faucet drips 1000 fee tokens per day. Silently skips if already dripped today.
 */
async function ensureFeeTokens(params: {
	chain: typeof bscTestnet | typeof polygonAmoy
	rpcUrl: string
	host: `0x${string}`
	faucet: `0x${string}`
	account: ReturnType<typeof privateKeyToAccount>
}) {
	const { chain, rpcUrl, host, faucet, account } = params
	console.log(`[ensureFeeTokens] chain=${chain.name} host=${host} faucet=${faucet} account=${account.address}`)
	console.log(`[ensureFeeTokens] creating viem clients on ${rpcUrl}`)
	const publicClient = createPublicClient({ chain, transport: http(rpcUrl) })
	const walletClient = createWalletClient({ account, chain, transport: http(rpcUrl) })

	console.log(`[ensureFeeTokens] reading host.feeToken() from ${host}`)
	const feeToken = await publicClient.readContract({
		address: host,
		abi: EVM_HOST.ABI,
		functionName: "feeToken",
	})
	console.log(`[ensureFeeTokens] feeToken=${feeToken}`)

	console.log(`[ensureFeeTokens] reading balanceOf(${account.address}) from ${feeToken}`)
	const balance = await publicClient.readContract({
		address: feeToken as `0x${string}`,
		abi: [{ type: "function", name: "balanceOf", inputs: [{ name: "account", type: "address" }], outputs: [{ type: "uint256" }], stateMutability: "view" }],
		functionName: "balanceOf",
		args: [account.address],
	})

	console.log(`[ensureFeeTokens] Fee token balance on ${chain.name}: ${balance}`)

	// Drip if balance is below 100 tokens
	if (balance < parseEther("100")) {
		console.log(`[ensureFeeTokens] balance below threshold, dripping from faucet ${faucet}`)
		try {
			const hash = await walletClient.writeContract({
				address: faucet,
				abi: FAUCET_ABI,
				functionName: "drip",
				args: [feeToken as `0x${string}`],
			})
			console.log(`[ensureFeeTokens] Faucet drip tx: ${hash} — waiting for receipt`)
			await publicClient.waitForTransactionReceipt({ hash })
			console.log("[ensureFeeTokens] Faucet drip confirmed")
		} catch (e) {
			console.log("[ensureFeeTokens] Faucet drip skipped (already dripped today or error):", (e as Error).message)
		}
	} else {
		console.log("[ensureFeeTokens] balance sufficient, skipping faucet")
	}
}

async function runBridgeFlow(params: {
	hft: HyperFungibleToken
	token: `0x${string}`
	account: ReturnType<typeof privateKeyToAccount>
	sourceChain: typeof bscTestnet | typeof polygonAmoy
	destChain: typeof bscTestnet | typeof polygonAmoy
	sourceRpc: string
	destRpc: string
	destHost: `0x${string}`
	amount: bigint
}) {
	const { hft, token, account, sourceChain, destChain, sourceRpc, destRpc, destHost, amount } = params
	console.log(`[runBridgeFlow] token=${token} sourceChain=${sourceChain.name} destChain=${destChain.name} amount=${amount}`)

	console.log(`[runBridgeFlow] creating viem clients (source=${sourceRpc}, dest=${destRpc})`)
	const walletClient = createWalletClient({ account, chain: sourceChain, transport: http(sourceRpc) })
	const publicClient = createPublicClient({ chain: sourceChain, transport: http(sourceRpc) })
	const destWalletClient = createWalletClient({ account, chain: destChain, transport: http(destRpc) })
	const destPublicClient = createPublicClient({ chain: destChain, transport: http(destRpc) })

	const destStateMachine = destChain.id === 97 ? "EVM-97" : "EVM-80002"
	console.log(`[runBridgeFlow] destStateMachine=${destStateMachine}`)

	console.log("[runBridgeFlow] starting hft.bridge() generator")
	const gen = hft.bridge({
		token,
		from: account.address,
		to: account.address as HexString,
		amount,
		dest: destStateMachine,
		timeout: 7200n,
		payInFeeToken: true,
		relayerFee: parseEther("5"),
	})

	const statuses: string[] = []
	let commitment: string | undefined
	console.log("[runBridgeFlow] awaiting first generator step")
	let result = await gen.next()

	let stepCount = 0
	while (!result.done) {
		const step = result.value
		if (!step) {
			console.log(`[runBridgeFlow] step ${stepCount}: empty value, breaking`)
			break
		}
		stepCount++

		console.log(`[runBridgeFlow] step ${stepCount}: type=${step.type}`)

		if (step.type === "approve") {
			console.log(`[runBridgeFlow] step ${stepCount}: sending approve tx to ${step.tx.to}`)
			const hash = await walletClient.sendTransaction({
				to: step.tx.to,
				data: step.tx.data as `0x${string}`,
			})
			console.log(`[runBridgeFlow] step ${stepCount}: Approve tx=${hash}, waiting for receipt`)
			await publicClient.waitForTransactionReceipt({ hash })
			console.log(`[runBridgeFlow] step ${stepCount}: approve confirmed, calling gen.next()`)
			result = await gen.next()
			continue
		}

		if (step.type === "send") {
			console.log(`[runBridgeFlow] step ${stepCount}: sending tx to ${step.tx.to} value=${step.tx.value}`)
			const hash = await walletClient.sendTransaction({
				to: step.tx.to,
				data: step.tx.data as `0x${string}`,
				value: step.tx.value,
			})
			console.log(`[runBridgeFlow] step ${stepCount}: Send tx=${hash}, calling gen.next(hash)`)
			result = await gen.next(hash)
			continue
		}

		if (step.type === "submitted") {
			commitment = step.commitment
			console.log(`[runBridgeFlow] step ${stepCount}: Commitment=${commitment}, calling gen.next()`)
			result = await gen.next()
			continue
		}

		if (step.type === "status") {
			console.log(`[runBridgeFlow] step ${stepCount}: Status=${step.status}`)
			statuses.push(step.status)

			if (step.status === "HYPERBRIDGE_FINALIZED") {
				const calldata = (step.metadata as { calldata: Hex }).calldata
				console.log(`[runBridgeFlow] step ${stepCount}: HYPERBRIDGE_FINALIZED — reading destHost.hostParams() at ${destHost}`)

				const hostParams = await destPublicClient.readContract({
					address: destHost,
					abi: EVM_HOST.ABI,
					functionName: "hostParams",
				})
				console.log(`[runBridgeFlow] step ${stepCount}: destHandler=${hostParams.handler}`)

				try {
					console.log(`[runBridgeFlow] step ${stepCount}: submitting calldata to destHandler`)
					const hash = await destWalletClient.sendTransaction({
						to: hostParams.handler as `0x${string}`,
						data: calldata,
					})
					console.log(`[runBridgeFlow] step ${stepCount}: Dest tx=${hash}, waiting for receipt`)
					await destPublicClient.waitForTransactionReceipt({ hash })
					console.log(`[runBridgeFlow] step ${stepCount}: Dest tx confirmed`)
				} catch (e) {
					console.log(`[runBridgeFlow] step ${stepCount}: dest submit reverted, checking requestReceipts for ${commitment}`)
					const receipt = await destPublicClient.readContract({
						address: destHost,
						abi: EVM_HOST.ABI,
						functionName: "requestReceipts",
						args: [commitment as `0x${string}`],
					})
					if (receipt === "0x0000000000000000000000000000000000000000") {
						console.log(`[runBridgeFlow] step ${stepCount}: no receipt yet, rethrowing`)
						throw e
					}
					console.log(`[runBridgeFlow] step ${stepCount}: Already delivered by: ${receipt}`)
				}
			}

			if (step.status === "DESTINATION" || step.status === "TIMED_OUT") {
				console.log(`[runBridgeFlow] step ${stepCount}: terminal status ${step.status}, breaking loop`)
				break
			}
			console.log(`[runBridgeFlow] step ${stepCount}: calling gen.next()`)
			result = await gen.next()
			continue
		}

		console.log(`[runBridgeFlow] step ${stepCount}: unhandled type ${step.type}, calling gen.next()`)
		result = await gen.next()
	}

	console.log(`[runBridgeFlow] generator done after ${stepCount} steps, statuses=${JSON.stringify(statuses)}`)
	return { commitment, statuses }
}

describe("HyperFungibleToken SDK", () => {
	it("should detect WrappedHFT on BSC", async () => {
		console.log("[test:detect-bsc] start — BSC_WRAPPED_HFT=", BSC_WRAPPED_HFT)
		console.log("[test:detect-bsc] creating BSC->Polygon chains")
		const { source, dest } = createBscToPolygon()
		console.log("[test:detect-bsc] constructing HyperFungibleToken")
		const hft = new HyperFungibleToken({ source, dest })

		console.log("[test:detect-bsc] calling isWrapped()")
		const isWrapped = await hft.isWrapped(BSC_WRAPPED_HFT)
		console.log("[test:detect-bsc] isWrapped =", isWrapped)
		expect(isWrapped).toBe(true)
	}, 30_000)

	it("should detect HFT is not wrapped on Polygon", async () => {
		console.log("[test:detect-polygon] start — POLYGON_HFT=", POLYGON_HFT)
		console.log("[test:detect-polygon] creating Polygon->BSC chains")
		const { source, dest } = createPolygonToBsc()
		console.log("[test:detect-polygon] constructing HyperFungibleToken")
		const hft = new HyperFungibleToken({ source, dest })

		console.log("[test:detect-polygon] calling isWrapped()")
		const isWrapped = await hft.isWrapped(POLYGON_HFT)
		console.log("[test:detect-polygon] isWrapped =", isWrapped)
		expect(isWrapped).toBe(false)
	}, 30_000)

	it("should lock BNB on BSC and mint on Polygon", async () => {
		console.log("[test:lock-bsc] === Lock BNB on BSC → Mint on Polygon ===")
		console.log("[test:lock-bsc] creating BSC->Polygon chains")
		const { source, dest } = createBscToPolygon()
		console.log("[test:lock-bsc] creating IsmpClient (hyperbridge connection)")
		const { ismpClient, hyperbridge } = await createIsmpClient(source, dest)
		console.log("[test:lock-bsc] constructing HyperFungibleToken with client")
		const hft = new HyperFungibleToken({ source, dest, client: ismpClient })
		// Bypass on-chain quote() — it routes through host.uniswapV2Router which isn't configured on the redeployed host.
		hft.quote = async (p: BridgeParams): Promise<QuoteResult> => ({
			totalNativeCost: 0n,
			totalFeeTokenCost: p.relayerFee ?? 0n,
			relayerFeeInFeeToken: p.relayerFee ?? 0n,
		})
		const account = privateKeyToAccount(PRIVATE_KEY)
		console.log("[test:lock-bsc] account =", account.address)

		console.log("[test:lock-bsc] ensuring fee tokens on BSC")
		await ensureFeeTokens({ chain: bscTestnet, rpcUrl: BSC_RPC, host: BSC_HOST, faucet: BSC_FAUCET, account })

		console.log("[test:lock-bsc] starting runBridgeFlow")
		const { commitment, statuses } = await runBridgeFlow({
			hft,
			token: BSC_WRAPPED_HFT,
			account,
			sourceChain: bscTestnet,
			destChain: polygonAmoy,
			sourceRpc: BSC_RPC,
			destRpc: POLYGON_RPC,
			destHost: POLYGON_HOST,
			amount: parseEther("0.001"),
		})

		console.log("[test:lock-bsc] Commitment:", commitment)
		console.log("[test:lock-bsc] All statuses:", statuses)
		expect(commitment).toBeDefined()
		expect(statuses).toContain("SOURCE_FINALIZED")
		expect(statuses).toContain("HYPERBRIDGE_DELIVERED")
		expect(statuses).toContain("HYPERBRIDGE_FINALIZED")
		expect(statuses).toContain("DESTINATION")

		console.log("[test:lock-bsc] disconnecting hyperbridge")
		await hyperbridge.disconnect()
		console.log("[test:lock-bsc] done")
	}, 1_800_000)

	it("should burn on Polygon and unlock BNB on BSC", async () => {
		console.log("[test:burn-polygon] === Burn on Polygon → Unlock BNB on BSC ===")
		console.log("[test:burn-polygon] creating Polygon->BSC chains")
		const { source, dest } = createPolygonToBsc()
		console.log("[test:burn-polygon] creating IsmpClient (hyperbridge connection)")
		const { ismpClient, hyperbridge } = await createIsmpClient(source, dest)
		console.log("[test:burn-polygon] constructing HyperFungibleToken with client")
		const hft = new HyperFungibleToken({ source, dest, client: ismpClient })
		// Bypass on-chain quote() — it routes through host.uniswapV2Router which isn't configured on the redeployed host.
		hft.quote = async (p: BridgeParams): Promise<QuoteResult> => ({
			totalNativeCost: 0n,
			totalFeeTokenCost: p.relayerFee ?? 0n,
			relayerFeeInFeeToken: p.relayerFee ?? 0n,
		})
		const account = privateKeyToAccount(PRIVATE_KEY)
		console.log("[test:burn-polygon] account =", account.address)

		console.log("[test:burn-polygon] ensuring fee tokens on Polygon")
		await ensureFeeTokens({ chain: polygonAmoy, rpcUrl: POLYGON_RPC, host: POLYGON_HOST, faucet: POLYGON_FAUCET, account })

		console.log("[test:burn-polygon] starting runBridgeFlow")
		const { commitment, statuses } = await runBridgeFlow({
			hft,
			token: POLYGON_HFT,
			account,
			sourceChain: polygonAmoy,
			destChain: bscTestnet,
			sourceRpc: POLYGON_RPC,
			destRpc: BSC_RPC,
			destHost: BSC_HOST,
			amount: parseEther("0.001"),
		})

		console.log("[test:burn-polygon] Commitment:", commitment)
		console.log("[test:burn-polygon] All statuses:", statuses)
		expect(commitment).toBeDefined()
		expect(statuses).toContain("SOURCE_FINALIZED")
		expect(statuses).toContain("HYPERBRIDGE_DELIVERED")
		expect(statuses).toContain("HYPERBRIDGE_FINALIZED")
		expect(statuses).toContain("DESTINATION")

		console.log("[test:burn-polygon] disconnecting hyperbridge")
		await hyperbridge.disconnect()
		console.log("[test:burn-polygon] done")
	}, 1_800_000)
})
