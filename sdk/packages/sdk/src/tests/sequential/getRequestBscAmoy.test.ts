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
 * `IsmpClient.getRequestStatusStream` and drives it to completion: resolve from the indexer →
 * SOURCE_FINALIZED → self-deliver to Hyperbridge (deliverToHyperbridge logs each proof) →
 * HYPERBRIDGE_FINALIZED, at which point the GetResponse calldata is submitted to the BSC handler,
 * invoking HyperGet.onGetResponse (which RLP-decodes and emits the balance) → DESTINATION.
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
	{
		type: "event",
		name: "BalanceReceived",
		inputs: [
			{ name: "commitment", type: "bytes32", indexed: true },
			{ name: "account", type: "address", indexed: true },
			{ name: "balance", type: "uint256", indexed: false },
		],
	},
] as const

const has = (v?: string) => typeof v === "string" && v.length > 0

// Skipped: state proofs are currently broken on BNB testnet RPCs
describe.skip("GET self-delivery — BSC Chapel → Polygon Amoy (live)", () => {
	it.skipIf(!has(process.env.PRIVATE_KEY) || !has(process.env.BSC_CHAPEL))(
		"reads WMATIC.balanceOf on Amoy via GET and drives it to completion on the source (onGetResponse)",
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

			// 4. Drive the full round-trip via the real entry point. getRequestStatusStream resolves
			//    the request from the indexer, waits for SOURCE_FINALIZED, self-delivers it to
			//    Hyperbridge (deliverToHyperbridge logs each proof), then finalizes the GetResponse.
			//    At HYPERBRIDGE_FINALIZED we submit the response calldata to the BSC handler, which
			//    invokes HyperGet.onGetResponse — completing the GET on the source chain.
			const seen: string[] = []
			let received: { account: HexString; balance: bigint } | undefined
			for await (const update of client.getRequestStatusStream(commitment)) {
				seen.push(update.status)
				console.log(`[4] stream: ${update.status}`)

				if (update.status === RequestStatus.HYPERBRIDGE_FINALIZED) {
					const calldata = (update.metadata as { calldata?: HexString }).calldata
					if (!calldata) throw new Error("HYPERBRIDGE_FINALIZED yielded no calldata")
					const params = (await publicClient.readContract({
						address: HOST,
						abi: EVM_HOST.ABI,
						functionName: "hostParams",
					})) as { handler: HexString }
					console.log(`[5] delivering GetResponse to BSC handler ${params.handler}`)
					const respTx = await walletClient.sendTransaction({ to: params.handler, data: calldata })
					const respReceipt = await publicClient.waitForTransactionReceipt({ hash: respTx })
					const ev = parseEventLogs({ abi: HYPER_GET_ABI, logs: respReceipt.logs }).find(
						(l) => l.eventName === "BalanceReceived",
					) as any
					if (ev) {
						received = { account: ev.args.account, balance: ev.args.balance }
						console.log(`[5] BalanceReceived: account=${received.account} balance=${received.balance}`)
					}
				}

				if (update.status === RequestStatus.DESTINATION) break
			}

			// Completed on the source chain: Hyperbridge delivered + finalized, and the response was
			// received by HyperGet, which RLP-decoded the queried account's WMATIC balance.
			expect(seen).toContain(RequestStatus.HYPERBRIDGE_DELIVERED)
			expect(seen).toContain(RequestStatus.DESTINATION)
			expect(received?.account?.toLowerCase()).toBe(reader.toLowerCase())
			expect(received?.balance).toBeGreaterThan(0n)

			await hyperbridge.disconnect()
		},
		40 * 60 * 1000,
	)
})
