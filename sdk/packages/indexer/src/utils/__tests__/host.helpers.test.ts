import { getHostFeeToken } from "@/utils/host.helpers"
import { EthereumHostAbi__factory, ERC6160Ext20Abi__factory } from "@/configs/src/types/contracts"

jest.mock("@/constants", () => ({ CHAINS_BY_ISMP_HOST: { "EVM-1": "host-1", "EVM-56": "host-56" } }))
const feeToken = jest.fn()
const decimals = jest.fn()

beforeEach(() => {
	;(global as any).api = {}
	jest.spyOn(EthereumHostAbi__factory, "connect").mockReturnValue({ feeToken } as any)
	jest.spyOn(ERC6160Ext20Abi__factory, "connect").mockReturnValue({ decimals } as any)
	feeToken.mockReset().mockResolvedValue("0xABC")
	decimals.mockReset().mockResolvedValue(6)
})
afterEach(() => jest.restoreAllMocks())

it("pins both reads to the placement block and reuses that block's metadata", async () => {
	expect(await getHostFeeToken("EVM-1", "0xblock1")).toEqual({ address: "0xabc", decimals: 6 })
	await getHostFeeToken("EVM-1", "0xblock1")
	expect(feeToken).toHaveBeenCalledTimes(1)
	expect(feeToken).toHaveBeenCalledWith({ blockTag: "0xblock1" })
	expect(decimals).toHaveBeenCalledWith({ blockTag: "0xblock1" })
})

it("refreshes on a new block hash and keeps chains independent", async () => {
	await getHostFeeToken("EVM-1", "0xblock2")
	feeToken.mockResolvedValue("0xDEF")
	decimals.mockResolvedValue(18)
	expect(await getHostFeeToken("EVM-1", "0xreorg")).toEqual({ address: "0xdef", decimals: 18 })
	await getHostFeeToken("EVM-56", "0xreorg")
	expect(feeToken).toHaveBeenCalledTimes(3)
	expect(EthereumHostAbi__factory.connect).toHaveBeenLastCalledWith("host-56", api)
})

it("does not cache failed metadata reads, allowing the block to retry", async () => {
	decimals.mockRejectedValueOnce(new Error("RPC down"))
	await expect(getHostFeeToken("EVM-1", "0xretry")).rejects.toThrow("RPC down")
	expect(await getHostFeeToken("EVM-1", "0xretry")).toEqual({ address: "0xabc", decimals: 6 })
	expect(feeToken).toHaveBeenCalledTimes(2)
})

it("rejects missing host configuration", async () => {
	await expect(getHostFeeToken("EVM-unknown", "0xblock")).rejects.toThrow("No ISMP host")
	expect(feeToken).not.toHaveBeenCalled()
})
