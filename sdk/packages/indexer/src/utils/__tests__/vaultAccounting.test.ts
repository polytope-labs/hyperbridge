jest.mock("@/constants", () => ({ ENV_CONFIG: { "EVM-8453": "https://base.example" } }))
jest.mock("@/utils/rpc.helpers", () => ({ replaceWebsocketWithHttp: (url: string) => url }))
jest.mock("@/utils/safeFetch", () => ({ safeFetch: jest.fn() }))

import { ethers } from "ethers"
import Erc4626Abi from "@/configs/abis/Erc4626.abi.json"
import { readVaultBlockMovements, vaultCapitalMovements } from "@/utils/vaultAccounting"
import { safeFetch } from "@/utils/safeFetch"

const abi = new ethers.utils.Interface(Erc4626Abi)
const FROM = "0x1111111111111111111111111111111111111111"
const TO = "0x2222222222222222222222222222222222222222"
const VAULT = "0xc768c589647798a6ee01a91fde98ef2ed046dbd6"
const tx = `0x${"ab".repeat(32)}`
const log = (name: string, values: any[]) => ({
	...abi.encodeEventLog(abi.getEvent(name), values),
	transactionHash: tx,
	logIndex: 2,
})

beforeEach(() => jest.resetAllMocks())

it("decodes deposits and withdrawals with the share owner rather than the caller or receiver", () => {
	expect(vaultCapitalMovements(log("Deposit", [FROM, TO, 110, 100]))).toEqual([
		{ transactionHash: tx, logIndex: 2, lp: TO, shares: 100n },
	])
	expect(vaultCapitalMovements(log("Withdraw", [FROM, FROM, TO, 110, 100]))).toEqual([
		{ transactionHash: tx, logIndex: 2, lp: TO, shares: -100n },
	])
})

it("decodes both ordinary transfer sides, excluding mint, burn, self and zero-value transfers", () => {
	expect(vaultCapitalMovements(log("Transfer", [FROM, TO, 100]))).toEqual([
		{ transactionHash: tx, logIndex: 2, lp: FROM, shares: -100n },
		{ transactionHash: tx, logIndex: 2, lp: TO, shares: 100n },
	])
	for (const args of [
		[ethers.constants.AddressZero, TO, 100],
		[FROM, ethers.constants.AddressZero, 100],
		[FROM, FROM, 100],
		[FROM, TO, 0],
	]) {
		expect(vaultCapitalMovements(log("Transfer", args))).toEqual([])
	}
})

it("pins the RPC to the handler block and decodes hex log indexes", async () => {
	jest.mocked(safeFetch).mockResolvedValue({
		ok: true,
		json: async () => ({
			result: [{ ...log("Transfer", [FROM, TO, 100]), address: VAULT, blockNumber: "0x32", logIndex: "0xa" }],
		}),
	} as any)
	const result = await readVaultBlockMovements("EVM-8453", VAULT, 50n)
	expect(result.map((m) => m.logIndex)).toEqual([10, 10])
	const request = JSON.parse(jest.mocked(safeFetch).mock.calls[0][1]!.body as string)
	expect(request).toMatchObject({
		method: "eth_getLogs",
		params: [{ address: VAULT, fromBlock: "0x32", toBlock: "0x32" }],
	})
})

it.each([
	{ error: { message: "rate limited" } },
	{ result: null },
	{ result: [{ ...log("Transfer", [FROM, TO, 100]), address: VAULT, blockNumber: "0x31", logIndex: "0x2" }] },
	{
		result: [
			{
				...log("Transfer", [FROM, TO, 100]),
				address: VAULT,
				blockNumber: "0x32",
				logIndex: "0x2",
				removed: true,
			},
		],
	},
])("rejects incomplete or mismatched RPC data", async (body) => {
	jest.mocked(safeFetch).mockResolvedValue({ ok: true, json: async () => body } as any)
	await expect(readVaultBlockMovements("EVM-8453", VAULT, 50n)).rejects.toThrow()
})

it("rejects duplicate log indexes instead of using them to corrupt an opening balance", async () => {
	const entry = { ...log("Transfer", [FROM, TO, 100]), address: VAULT, blockNumber: "0x32", logIndex: "0x2" }
	jest.mocked(safeFetch).mockResolvedValue({ ok: true, json: async () => ({ result: [entry, entry] }) } as any)
	await expect(readVaultBlockMovements("EVM-8453", VAULT, 50n)).rejects.toThrow("Duplicate")
})

it("rejects HTTP failures even if the payload looks like a valid empty log result", async () => {
	jest.mocked(safeFetch).mockResolvedValue({ ok: false, json: async () => ({ result: [] }) } as any)
	await expect(readVaultBlockMovements("EVM-8453", VAULT, 50n)).rejects.toThrow("Could not read")
})

it("rejects a chain without configured RPC rather than treating its history as empty", async () => {
	await expect(readVaultBlockMovements("EVM-999", VAULT, 50n)).rejects.toThrow("No RPC configured")
	expect(safeFetch).not.toHaveBeenCalled()
})
