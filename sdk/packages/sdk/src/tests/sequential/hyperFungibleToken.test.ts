import { describe, it, expect } from "vitest"
import { createPublicClient, createWalletClient, http, parseEther, type Hex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet, polygonAmoy } from "viem/chains"
import { EvmChain } from "@/chains/evm"
import { SubstrateChain } from "@/chains/substrate"
import { IsmpClient } from "@/client"
import { createQueryClient } from "@/queryClient"
import { HyperFungibleToken } from "@/protocols/hyperFungibleToken"
import EVM_HOST from "@/abis/evmHost"
import type { HexString } from "@/types"

// WrappedHFT wrapping WBNB on BSC testnet (lock/unlock)
const BSC_WRAPPED_HFT = "0x5B1D14417f44D5DcC116bEd1fa50b91B4eF73dda" as const
// HFT on Polygon Amoy (burn/mint) — paired with BSC WrappedHFT
const POLYGON_HFT = "0xc74d342B1907d724CbA584F663c7e180A8b708D3" as const

const BSC_HOST = "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7" as const
const POLYGON_HOST = "0x9a2840D050e64Db89c90Ac5857536E4ec66641DE" as const

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
		consensusStateId: "POL0",
	})

	return { source, dest }
}

function createPolygonToBsc() {
	const source = EvmChain.fromParams({
		chainId: 80002,
		rpcUrl: POLYGON_RPC,
		host: POLYGON_HOST,
		consensusStateId: "POL0",
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
	const hyperbridge = await SubstrateChain.connect({
		wsUrl: process.env.HYPERBRIDGE_GARGANTUA || "wss://gargantua.rpc.polytope.technology",
		consensusStateId: "PAS0",
		hasher: "Keccak",
		stateMachineId: "KUSAMA-4009",
	})

	const queryClient = createQueryClient({
		url: "https://gargantua.indexer.polytope.technology",
	})

	const ismpClient = new IsmpClient({
		queryClient,
		source,
		dest,
		hyperbridge,
		pollInterval: 5_000,
	})

	return { ismpClient, hyperbridge }
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

	const walletClient = createWalletClient({ account, chain: sourceChain, transport: http(sourceRpc) })
	const publicClient = createPublicClient({ chain: sourceChain, transport: http(sourceRpc) })
	const destWalletClient = createWalletClient({ account, chain: destChain, transport: http(destRpc) })
	const destPublicClient = createPublicClient({ chain: destChain, transport: http(destRpc) })

	const destStateMachine = destChain.id === 97 ? "EVM-97" : "EVM-80002"

	const gen = hft.bridge({
		token,
		from: account.address,
		to: account.address as HexString,
		amount,
		dest: destStateMachine,
		timeout: 7200n,
		payInFeeToken: true,
		relayerFee: 0n,
	})

	const statuses: string[] = []
	let commitment: string | undefined
	let result = await gen.next()

	while (!result.done) {
		const step = result.value
		if (!step) break

		console.log(`Step: ${step.type}`)

		if (step.type === "approve") {
			const hash = await walletClient.sendTransaction({
				to: step.tx.to,
				data: step.tx.data as `0x${string}`,
			})
			console.log("Approve tx:", hash)
			await publicClient.waitForTransactionReceipt({ hash })
			result = await gen.next()
			continue
		}

		if (step.type === "send") {
			const hash = await walletClient.sendTransaction({
				to: step.tx.to,
				data: step.tx.data as `0x${string}`,
				value: step.tx.value,
			})
			console.log("Send tx:", hash)
			result = await gen.next(hash)
			continue
		}

		if (step.type === "submitted") {
			commitment = step.commitment
			console.log("Commitment:", commitment)
			result = await gen.next()
			continue
		}

		if (step.type === "status") {
			console.log(`Status: ${step.status}`)
			statuses.push(step.status)

			if (step.status === "HYPERBRIDGE_FINALIZED") {
				const calldata = (step.metadata as { calldata: Hex }).calldata
				console.log("Submitting HYPERBRIDGE_FINALIZED calldata to dest chain...")

				const hostParams = await destPublicClient.readContract({
					address: destHost,
					abi: EVM_HOST.ABI,
					functionName: "hostParams",
				})

				const hash = await destWalletClient.sendTransaction({
					to: hostParams.handler as `0x${string}`,
					data: calldata,
				})
				console.log("Dest tx:", hash)
				await destPublicClient.waitForTransactionReceipt({ hash })
				console.log("Dest tx confirmed")
			}

			if (step.status === "DESTINATION" || step.status === "TIMED_OUT") break
			result = await gen.next()
			continue
		}

		result = await gen.next()
	}

	return { commitment, statuses }
}

describe("HyperFungibleToken SDK", () => {
	it("should detect WrappedHFT on BSC", async () => {
		const { source, dest } = createBscToPolygon()
		const hft = new HyperFungibleToken({ source, dest })

		const isWrapped = await hft.isWrapped(BSC_WRAPPED_HFT)
		expect(isWrapped).toBe(true)
	}, 30_000)

	it("should detect HFT is not wrapped on Polygon", async () => {
		const { source, dest } = createPolygonToBsc()
		const hft = new HyperFungibleToken({ source, dest })

		const isWrapped = await hft.isWrapped(POLYGON_HFT)
		expect(isWrapped).toBe(false)
	}, 30_000)

	it("should lock BNB on BSC and mint on Polygon", async () => {
		const { source, dest } = createBscToPolygon()
		const { ismpClient, hyperbridge } = await createIsmpClient(source, dest)
		const hft = new HyperFungibleToken({ source, dest, client: ismpClient })
		const account = privateKeyToAccount(PRIVATE_KEY)

		console.log("=== Lock BNB on BSC → Mint on Polygon ===")
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

		console.log("Commitment:", commitment)
		console.log("All statuses:", statuses)
		expect(commitment).toBeDefined()
		expect(statuses).toContain("SOURCE_FINALIZED")
		expect(statuses).toContain("HYPERBRIDGE_DELIVERED")
		expect(statuses).toContain("HYPERBRIDGE_FINALIZED")
		expect(statuses).toContain("DESTINATION")

		await hyperbridge.disconnect()
	}, 1_800_000)

	it("should burn on Polygon and unlock BNB on BSC", async () => {
		const { source, dest } = createPolygonToBsc()
		const { ismpClient, hyperbridge } = await createIsmpClient(source, dest)
		const hft = new HyperFungibleToken({ source, dest, client: ismpClient })
		const account = privateKeyToAccount(PRIVATE_KEY)

		console.log("=== Burn on Polygon → Unlock BNB on BSC ===")
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

		console.log("Commitment:", commitment)
		console.log("All statuses:", statuses)
		expect(commitment).toBeDefined()
		expect(statuses).toContain("SOURCE_FINALIZED")
		expect(statuses).toContain("HYPERBRIDGE_DELIVERED")
		expect(statuses).toContain("HYPERBRIDGE_FINALIZED")
		expect(statuses).toContain("DESTINATION")

		await hyperbridge.disconnect()
	}, 1_800_000)
})
