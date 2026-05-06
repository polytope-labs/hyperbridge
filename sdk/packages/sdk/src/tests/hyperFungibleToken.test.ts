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

// Testnet deployed addresses
const BSC_HFT = "0x47cf44a9376595a330ada65a1c4a661a7fcf28a9" as const
const POLYGON_HFT = "0xe7abebef23abee9dc4d2789902d3aef3cba1d8e0" as const

const BSC_HOST = "0x8Aa0Dea6D675d785A882967Bf38183f6117C09b7" as const
const POLYGON_HOST = "0x9a2840D050e64Db89c90Ac5857536E4ec66641DE" as const

const PRIVATE_KEY = process.env.PRIVATE_KEY as HexString
const BSC_RPC = process.env.BSC_CHAPEL!
const POLYGON_RPC = process.env.POLYGON_AMOY!

function createChains() {
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

describe("HyperFungibleToken SDK", () => {
	it("should detect HFT is not wrapped", async () => {
		const { source, dest } = createChains()
		const hft = new HyperFungibleToken({ source, dest })

		const isWrapped = await hft.isWrapped(BSC_HFT)
		expect(isWrapped).toBe(false)
	}, 30_000)

	it("should quote fee for BSC → Polygon send", async () => {
		const { source, dest } = createChains()
		const hft = new HyperFungibleToken({ source, dest })

		const account = privateKeyToAccount(PRIVATE_KEY)

		const fee = await hft.quote({
			token: BSC_HFT,
			from: account.address,
			to: account.address as HexString,
			amount: parseEther("1"),
			dest: "EVM-80002",
		})

		console.log("Quote result:", {
			totalNativeCost: fee.totalNativeCost.toString(),
			totalFeeTokenCost: fee.totalFeeTokenCost.toString(),
			relayerFeeInFeeToken: fee.relayerFeeInFeeToken.toString(),
		})

		expect(fee.totalFeeTokenCost).toBeGreaterThan(0n)
		// totalNativeCost may be 0 on testnets without Uniswap router
		console.log("Native available:", fee.totalNativeCost > 0n)
	}, 60_000)

	it("should send and track full lifecycle BSC → Polygon", async () => {
		const { source, dest } = createChains()

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

		const hft = new HyperFungibleToken({ source, dest, ismpClient })

		const account = privateKeyToAccount(PRIVATE_KEY)
		const walletClient = createWalletClient({
			account,
			chain: bscTestnet,
			transport: http(BSC_RPC),
		})

		const publicClient = createPublicClient({
			chain: bscTestnet,
			transport: http(BSC_RPC),
		})

		const destWalletClient = createWalletClient({
			account,
			chain: polygonAmoy,
			transport: http(POLYGON_RPC),
		})

		const destPublicClient = createPublicClient({
			chain: polygonAmoy,
			transport: http(POLYGON_RPC),
		})

		const gen = hft.bridge({
			token: BSC_HFT,
			from: account.address,
			to: account.address as HexString,
			amount: parseEther("0.01"),
			dest: "EVM-80002",
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
					console.log("Submitting HYPERBRIDGE_FINALIZED calldata to dest chain...")

					const hostParams = await destPublicClient.readContract({
						address: POLYGON_HOST,
						abi: EVM_HOST.ABI,
						functionName: "hostParams",
					})

					const hash = await destWalletClient.sendTransaction({
						to: hostParams.handler as `0x${string}`,
						data: step.metadata.calldata,
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

		console.log("Commitment:", commitment)
		console.log("All statuses:", statuses)
		expect(commitment).toBeDefined()
		expect(statuses).toContain("SOURCE_FINALIZED")
		expect(statuses).toContain("HYPERBRIDGE_DELIVERED")
		expect(statuses).toContain("HYPERBRIDGE_FINALIZED")
		expect(statuses).toContain("DESTINATION")

		await hyperbridge.disconnect()
	}, 1_800_000) // 30 min — full cross-chain lifecycle on testnet
})
