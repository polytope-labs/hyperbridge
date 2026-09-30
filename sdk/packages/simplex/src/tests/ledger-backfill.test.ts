import type { HexString } from "@hyperbridge/sdk"
import { encodeAbiParameters, encodeEventTopics, type Log } from "viem"
import { describe, expect, it } from "vitest"
import { ERC4626_ABI } from "@/config/abis/Erc4626"
import { backfillVaultLedger } from "@/data/ledger-backfill"
import { MemoryDataStore } from "@/data/memory"
import { vaultMovementsFromLogs } from "@/funding/vault/ledger"
import { getLogger } from "@/services/Logger"

const VAULT = "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6" as HexString
const OTHER = "0xa82A3531021317240Fb32E67f9c7bC091F737D3b" as HexString
const SOLVER = "0x21426D68a9E5Df153FE75cE0fEd20173EBcb80eF" as HexString

function depositLog(address: HexString, assets: bigint, shares: bigint): Log {
	return {
		address,
		topics: encodeEventTopics({ abi: ERC4626_ABI, eventName: "Deposit", args: { sender: SOLVER, owner: SOLVER } }),
		data: encodeAbiParameters([{ type: "uint256" }, { type: "uint256" }], [assets, shares]),
	} as unknown as Log
}

describe("vault ledger backfill", () => {
	it("reads deposits into configured vaults out of a receipt and ignores other logs", () => {
		const logs = [
			depositLog(VAULT, 178_219_990n, 155_700_947n),
			depositLog("0x000000000000000000000000000000000000dEaD" as HexString, 1n, 1n),
			{ address: VAULT, topics: ["0x" + "ab".repeat(32)], data: "0x" } as unknown as Log,
		]
		expect(vaultMovementsFromLogs(logs, [VAULT.toLowerCase(), OTHER])).toEqual([
			{ kind: "sweep", vault: VAULT, assets: 178_219_990n, shares: 155_700_947n },
		])
	})

	it("fills a legacy sweep row from the described receipt and adds rows for extra vaults", async () => {
		const store = new MemoryDataStore()
		await store.activity.recordWalletTx({ kind: "sweep", chainId: 8453, token: null, amount: null, to: null, txHash: "0xc080", sponsored: true })
		await store.activity.recordWalletTx({ kind: "sweep", chainId: 8453, token: "USDC", amount: "1", to: VAULT, txHash: "0xnew", sponsored: true })
		const result = await backfillVaultLedger({
			store: store.activity,
			describe: async (_chainId, txHash) =>
				txHash === "0xc080"
					? [
							{ kind: "sweep", vault: VAULT, symbol: "USDC", amount: "178.21999", shareSymbol: "stataUSDC", shares: "155.700947" },
							{ kind: "sweep", vault: OTHER, symbol: "cNGN", amount: "1000", shareSymbol: "ycNGN", shares: "998" },
						]
					: [],
			logger: getLogger("test"),
		})
		expect(result).toEqual({ rows: 1 })
		const rows = await store.activity.walletTxs(10)
		expect(rows.map((row) => [row.token, row.amount, row.tokenIn, row.amountIn, row.to])).toEqual([
			["cNGN", "1000", "ycNGN", "998", OTHER],
			["USDC", "1", null, null, VAULT],
			["USDC", "178.21999", "stataUSDC", "155.700947", VAULT],
		])
		expect(await store.activity.walletTxsWithoutAmounts()).toEqual([])
	})
})
