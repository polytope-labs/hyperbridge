import { describe, expect, it } from "vitest"
import { createPublicClient, createWalletClient, encodeFunctionData, http, parseEventLogs, stringToHex } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { bscTestnet } from "viem/chains"

import { EvmChain } from "@/chains/evm"
import { SubstrateChain } from "@/chains/substrate"
import { IsmpClient } from "@/client"
import { createQueryClient } from "@/queryClient"
import { getRequestCommitment } from "@/utils"
import EVM_HOST from "@/abis/evmHost"
import { RequestStatus } from "@/types"
import type { HexString } from "@/types"

/**
 * Live end-to-end test of GET self-delivery, BSC Chapel → Polygon Amoy.
 *
 * Uses the deployed `HyperGet` test contract on BSC Chapel to dispatch a GET that reads
 * `WMATIC.balanceOf(reader)` on Polygon Amoy (balance mapping slot 3), then tracks it with
 * `IsmpClient.getRequestStatusStream`. The stream resolves the request from the indexer, waits
 * for SOURCE_FINALIZED, and self-delivers it to Hyperbridge (deliverToHyperbridge — its logs show
 * each proof being built + submitted). The test iterates until HYPERBRIDGE_DELIVERED.
 *
 * Requires funds + endpoints from sdk/.env.local; skipped otherwise. Slow (source finalization).
 *   npx vitest run --sequence.concurrent=false src/tests/sequential/getRequestBscAmoy.test.ts
 */

const HOST = "0x9AA003594d59C62EE17A73A569Fd7B1DbdBd71E1" as HexString
// HyperGet, deployed on BSC Chapel (see src/tests/contracts/HyperGet.sol).
const HYPER_GET = "0xAb1E0F59Dd32D7B4f2AC0A94a78dd9BbDa5510A2" as HexString
// Polygon Amoy WMATIC/WPOL and its balanceOf mapping slot.
const WMATIC = "0x360ad4f9a9A8EFe9A8DCB5f461c4Cc1047E1Dcf9" as HexString
const WMATIC_BALANCE_SLOT = 3n

const HYPER_GET_ABI = [
	{
		type: "function",
		name: "readBalance",
		stateMutability: "nonpayable",
		inputs: [
			{ name: "dest", type: "bytes" },
			{ name: "token", type: "address" },
			{ name: "account", type: "address" },
			{ name: "balanceSlot", type: "uint256" },
			{ name: "height", type: "uint64" },
		],
		outputs: [{ type: "bytes32" }],
	},
] as const

const has = (v?: string) => typeof v === "string" && v.length > 0

describe("GET self-delivery — BSC Chapel → Polygon Amoy (live)", () => {
	it.skipIf(!has(process.env.PRIVATE_KEY) || !has(process.env.BSC_CHAPEL))(
		"reads WMATIC.balanceOf on Amoy via GET and self-delivers to Hyperbridge",
		async () => {
			const BSC = process.env.BSC_CHAPEL!.split(",")[0]
			// The destination needs an ARCHIVE endpoint: by delivery time the GET's (fixed) read
			// height is a few hundred blocks old, so `eth_getProof` must reach historical state.
			// The .env.local Alchemy Amoy endpoint returns "root hash mismatch"; non-archive public
			// nodes prune the state. drpc's Amoy endpoint serves archive proofs.
			const AMOY = process.env.POLYGON_AMOY_ARCHIVE ?? "https://polygon-amoy.drpc.org"
			const GARGANTUA = process.env.HYPERBRIDGE_GARGANTUA!
			// The live gargantua indexer (indexes this testnet + connected chains). Not the CI's
			// local indexer, so default to the public endpoint; override with GARGANTUA_INDEXER_URL.
			const INDEXER = process.env.GARGANTUA_INDEXER_URL ?? "https://gargantua.indexer.polytope.technology"
			const pk = (process.env.PRIVATE_KEY!.startsWith("0x") ? process.env.PRIVATE_KEY! : `0x${process.env.PRIVATE_KEY!}`) as HexString
			const account = privateKeyToAccount(pk)
			const reader = account.address // whose WMATIC balance the GET reads

			const source = EvmChain.fromParams({ chainId: 97, rpcUrl: BSC, host: HOST, consensusStateId: "BSC0" })
			const dest = EvmChain.fromParams({ chainId: 80002, rpcUrl: AMOY, host: HOST, consensusStateId: "POLY" })
			const hyperbridge = await SubstrateChain.connect({ consensusStateId: "PAS0", stateMachineId: "KUSAMA-4009", wsUrl: GARGANTUA, hasher: "Keccak" })
			const client = new IsmpClient({ source, dest, hyperbridge, queryClient: createQueryClient({ url: INDEXER }), pollInterval: 3000 })

			// 1. Latest finalized Amoy height on Hyperbridge — the height the GET reads at.
			const amoyHeight = await hyperbridge.latestStateMachineHeight({ stateId: { Evm: 80002 }, consensusStateId: "POLY" })
			console.log(`[1] read WMATIC.balanceOf(${reader}) on Amoy @ finalized height ${amoyHeight}`)
			expect(amoyHeight).toBeGreaterThan(0n)

			// 2. Dispatch the GET from BSC Chapel via HyperGet.readBalance.
			const data = encodeFunctionData({
				abi: HYPER_GET_ABI,
				functionName: "readBalance",
				args: [stringToHex("EVM-80002"), WMATIC, reader, WMATIC_BALANCE_SLOT, amoyHeight],
			})
			const publicClient = createPublicClient({ chain: bscTestnet, transport: http(BSC) })
			const walletClient = createWalletClient({ account, chain: bscTestnet, transport: http(BSC) })
			const txHash = await walletClient.sendTransaction({ to: HYPER_GET, data })
			console.log(`[2] dispatch tx on BSC Chapel: ${txHash}`)
			const receipt = await publicClient.waitForTransactionReceipt({ hash: txHash })
			console.log(`    included in BSC block ${receipt.blockNumber}`)

			// 3. Compute the request commitment from the host event.
			const events = parseEventLogs({ abi: EVM_HOST.ABI, logs: receipt.logs })
			const evt = events.find((e) => e.eventName === "GetRequestEvent") as any
			if (!evt) throw new Error("GetRequestEvent not found")
			const a = evt.args
			const commitment = getRequestCommitment({
				source: a.source, dest: a.dest, from: a.from, nonce: a.nonce, height: a.height,
				keys: [...a.keys], timeoutTimestamp: a.timeoutTimestamp, context: a.context,
			})
			console.log(`[3] GET: ${a.source} → ${a.dest} nonce=${a.nonce} height=${a.height} commitment=${commitment}`)
			expect(a.dest).toBe("EVM-80002")

			// 4. Track + self-deliver via the real entry point. getRequestStatusStream resolves the
			//    request from the indexer, waits for SOURCE_FINALIZED, and drives deliverToHyperbridge
			//    (whose logs show each proof built + submitted). Iterate until Hyperbridge handles it.
			const seen: string[] = []
			for await (const update of client.getRequestStatusStream(commitment)) {
				seen.push(update.status)
				console.log(`[4] stream: ${update.status}`)
				if (update.status === RequestStatus.HYPERBRIDGE_DELIVERED) break
			}
			expect(seen).toContain(RequestStatus.HYPERBRIDGE_DELIVERED)

			await hyperbridge.disconnect()
		},
		40 * 60 * 1000,
	)
})
