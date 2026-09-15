import { ethers } from "ethers"

import Erc20Abi from "@/configs/abis/Erc20.abi.json"
import Multicall3Abi from "@/configs/abis/Multicall3.abi.json"
import {
	MULTICALL3_ADDRESS,
	MULTICALL_BATCH_SIZE,
	readContracts,
	resetMulticallCache,
	settle,
	unwrap,
} from "@/utils/multicall"

const CHAIN = "EVM-8453"
const TOKEN = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913"
const MULTICALL3 = MULTICALL3_ADDRESS.toLowerCase()
const erc20 = new ethers.utils.Interface(Erc20Abi)
const multicall3 = new ethers.utils.Interface(Multicall3Abi)

/** Holders whose balanceOf reverts. */
const reverting = new Set<string>()
let deployed = true

/** A holder's balance is its last byte, so every answer can be checked against its request. */
function balanceCall(data: string): string {
	const [holder] = erc20.decodeFunctionData("balanceOf", data)
	if (reverting.has(String(holder).toLowerCase())) throw new Error("execution reverted")
	return erc20.encodeFunctionResult("balanceOf", [parseInt(String(holder).slice(-2), 16)])
}

function answer({ to, data }: { to: string; data: string }): string {
	if (to.toLowerCase() !== MULTICALL3) return balanceCall(data)
	const [calls] = multicall3.decodeFunctionData("aggregate3", data)
	const results = calls.map((call: any) => {
		try {
			return { success: true, returnData: balanceCall(call.callData) }
		} catch {
			return { success: false, returnData: "0x" }
		}
	})
	return multicall3.encodeFunctionResult("aggregate3", [results])
}

const holder = (i: number) => `0x${i.toString(16).padStart(40, "0")}`
const balanceOf = (i: number) => ({ target: TOKEN, abi: erc20, method: "balanceOf", args: [holder(i)] })
const api = () => (global as any).api as { getCode: jest.Mock; call: jest.Mock }

beforeEach(() => {
	reverting.clear()
	deployed = true
	resetMulticallCache()
	;(global as any).api = {
		getCode: jest.fn(async () => (deployed ? "0x6080604052" : "0x")),
		call: jest.fn(async (tx: { to: string; data: string }) => answer(tx)),
	}
})

describe("readContracts", () => {
	test("batches reads into one aggregate3 call per MULTICALL_BATCH_SIZE, keeping their order", async () => {
		const reads = Array.from({ length: MULTICALL_BATCH_SIZE + 1 }, (_, i) => balanceOf(i % 256))

		const outcomes = await readContracts(CHAIN, reads)

		expect(api().call).toHaveBeenCalledTimes(2)
		expect(api().call.mock.calls.every(([tx]) => tx.to === MULTICALL3_ADDRESS)).toBe(true)
		expect(outcomes.map((outcome) => unwrap(outcome)[0].toNumber())).toEqual(reads.map((_, i) => i % 256))
	})

	test("a reverting read fails alone", async () => {
		reverting.add(holder(2))

		const outcomes = await readContracts(CHAIN, [balanceOf(1), balanceOf(2), balanceOf(3)])

		expect(outcomes.map((outcome) => outcome.ok)).toEqual([true, false, true])
		expect(() => unwrap(outcomes[1])).toThrow(`balanceOf on ${TOKEN} reverted`)
	})

	test("a single read is called directly, without checking for Multicall3", async () => {
		const [outcome] = await readContracts(CHAIN, [balanceOf(7)])

		expect(unwrap(outcome)[0].toNumber()).toBe(7)
		expect(api().getCode).not.toHaveBeenCalled()
		expect(api().call).toHaveBeenCalledWith({ to: TOKEN, data: expect.any(String) })
	})

	test("without Multicall3 every read is its own call, and the check is made once per chain", async () => {
		deployed = false
		reverting.add(holder(2))

		const outcomes = await readContracts(CHAIN, [balanceOf(1), balanceOf(2), balanceOf(3)])
		await readContracts(CHAIN, [balanceOf(4), balanceOf(5)])

		expect(outcomes.map((outcome) => outcome.ok)).toEqual([true, false, true])
		expect(api().call).toHaveBeenCalledTimes(5)
		expect(api().call.mock.calls.some(([tx]) => tx.to === MULTICALL3_ADDRESS)).toBe(false)
		expect(api().getCode).toHaveBeenCalledTimes(1)
	})

	test("no reads cost nothing", async () => {
		expect(await readContracts(CHAIN, [])).toEqual([])
		expect(api().getCode).not.toHaveBeenCalled()
		expect(api().call).not.toHaveBeenCalled()
	})

	test("a failed eth_call to Multicall3 throws", async () => {
		api().call.mockRejectedValueOnce(new Error("rpc down"))

		await expect(readContracts(CHAIN, [balanceOf(1), balanceOf(2)])).rejects.toThrow("rpc down")
	})
})

test("settle labels a failure", async () => {
	expect(await settle("getCode of 0xabc", Promise.reject(new Error("timeout")))).toEqual({
		ok: false,
		error: "getCode of 0xabc: timeout",
	})
	expect(await settle("x", Promise.resolve(1))).toEqual({ ok: true, value: 1 })
})
