import { getContractCallInput, getContractCallInputs } from "@/utils/rpc.helpers"
import { safeFetch } from "@/utils/safeFetch"
jest.mock("@/utils/safeFetch", () => ({ safeFetch: jest.fn() }))
jest.mock("@/constants", () => ({ ENV_CONFIG: { "EVM-56": "https://rpc.example" } }))
const gateway = "0x1111111111111111111111111111111111111111"
const call = (to?: string, input = "0x1234", extra = {}) => ({ to, input, ...extra })

it("collects successful nested calls in order, skipping reverted subtrees and missing recipients", async () => {
	jest.mocked(safeFetch).mockResolvedValue({
		json: async () => ({
			result: call("0xouter", "0x", {
				calls: [
					call(gateway, "0xaaaa"),
					call("0xreverted", "0x", { error: "execution reverted", calls: [call(gateway, "0xffff")] }),
					call(undefined),
					call("0xwrapper", "0x", { calls: [call(gateway, "0xbbbb")] }),
				],
			}),
		}),
	} as any)
	expect(await getContractCallInputs("0xtx", gateway, "EVM-56")).toEqual(["0xaaaa", "0xbbbb"])
	expect(await getContractCallInput("0xtx", gateway, "EVM-56")).toBe("0xaaaa")
})

it("preserves the single-call helper's direct-call contract", async () => {
	jest.mocked(safeFetch).mockResolvedValue({ json: async () => ({ result: call(gateway) }) } as any)
	expect(await getContractCallInput("0xtx", gateway, "EVM-56")).toBeNull()
})

it("propagates trace RPC failures to the caller", async () => {
	jest.mocked(safeFetch).mockResolvedValue({ json: async () => ({ error: { message: "not supported" } }) } as any)
	await expect(getContractCallInputs("0xtx", gateway, "EVM-56")).rejects.toThrow("not supported")
})
