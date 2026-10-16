import { beforeEach, describe, expect, it, vi } from "vitest"
import { concat, parseUnits, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { fundingShortfall, type ChainFunding } from "@/services/chain-funding"
import { nativeSetupNeed } from "@/services/native-setup"
import { readChainFunding, readChainFundingFromUnverifiedRpcs } from "@/services/read-chain-funding"

vi.mock("@/services/native-setup", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/services/native-setup")>()),
	nativeSetupNeed: vi.fn(),
}))

const WALLET = "0x7aae193f22bb500e38a6f970deafc46f58523c4c" as HexString
const HOST = "0x24B5d421Ec373FcA57325dd2F0C074009Af021F7" as HexString
const SOLVER_ACCOUNT = "0xaAd062555800a97Af062795189e32a3CBd045612" as HexString
const USDT = "0x55d398326f99059ff775485246999027b3197955" as HexString

const config = {
	getHostAddress: () => HOST,
	getSolverAccountContractAddress: () => SOLVER_ACCOUNT,
} as unknown as FillerConfigService

/** A chain whose host charges fees in USDT (18 decimals, as on BNB Chain). */
function client(balance: bigint, code?: HexString) {
	const readContract = vi.fn(async ({ functionName }: { functionName: string }) => {
		if (functionName === "feeToken") return USDT
		if (functionName === "symbol") return "USDT"
		if (functionName === "decimals") return 18
		if (functionName === "balanceOf") return balance
		throw new Error(`Unexpected read: ${functionName}`)
	})
	return {
		readContract,
		getBalance: vi.fn(async () => 0n),
		getCode: vi.fn(async () => code),
	} as unknown as PublicClient & { readContract: typeof readContract; getBalance: ReturnType<typeof vi.fn> }
}

// The fee token is cached per chain, so each test reads a chain of its own.
let chains = 0
const nextChain = () => `EVM-${9000 + ++chains}`

describe("readChainFunding", () => {
	beforeEach(() => {
		vi.mocked(nativeSetupNeed).mockReset()
		vi.mocked(nativeSetupNeed).mockResolvedValue(null)
	})

	it("is funded once the wallet holds the gateway fee token and needs no setup gas", async () => {
		const delegated = concat(["0xef0100", SOLVER_ACCOUNT])
		const funding = await readChainFunding(client(parseUnits("250", 18), delegated), config, WALLET, nextChain())
		expect(funding).toEqual({
			feeToken: { symbol: "USDT", balance: 250 },
			nativeSetup: null,
			funded: true,
			delegated: true,
		})
	})

	it("is not funded with less than five of the fee token", async () => {
		const funding = await readChainFunding(client(parseUnits("4.99", 18)), config, WALLET, nextChain())
		expect(funding).toMatchObject({ feeToken: { balance: 4.99 }, funded: false, delegated: false })
	})

	it("asks for the fee token and the setup native together before either arrives", async () => {
		vi.mocked(nativeSetupNeed).mockResolvedValue({ required: 0.000142, feeToken: "USDT" })
		const funding = await readChainFunding(client(0n), config, WALLET, nextChain())
		// Judged on the fee token itself, since the wallet holds none for the paymaster to pick.
		expect(vi.mocked(nativeSetupNeed).mock.calls[0][5]).toBe(USDT)
		expect(fundingShortfall(funding, "BNB")).toBe("at least 5 USDT and about 0.00014 BNB")
	})

	it("leaves the paymaster to pick its token once the wallet holds the fee token", async () => {
		await readChainFunding(client(parseUnits("5", 18)), config, WALLET, nextChain())
		expect(vi.mocked(nativeSetupNeed).mock.calls[0][5]).toBeUndefined()
	})

	it("is not funded while the paymaster's setup still needs native", async () => {
		vi.mocked(nativeSetupNeed).mockResolvedValue({ required: 0.000142, feeToken: "USDT" })
		const funding = await readChainFunding(client(parseUnits("250", 18)), config, WALLET, nextChain())
		expect(funding).toMatchObject({ funded: false, nativeSetup: { required: 0.000142, feeToken: "USDT" } })
	})

	it("reads the host's fee token once per chain", async () => {
		const chain = nextChain()
		const rpc = client(1n)
		await readChainFunding(rpc, config, WALLET, chain)
		await readChainFunding(rpc, config, WALLET, chain)
		const reads = rpc.readContract.mock.calls.map(([call]) => call.functionName)
		expect(reads.filter((name) => name === "feeToken")).toHaveLength(1)
		expect(reads.filter((name) => name === "balanceOf")).toHaveLength(2)
	})

	describe("through the wizard's unverified RPCs", () => {
		const GOOD = "https://good.example"
		const OTHER = "https://other-chain.example"
		const DEAD = "https://dead.example"

		/** Probes answer by URL; `connect` records the URLs a read goes through. */
		function wizardIo(chainId: number, rpc: ReturnType<typeof client>) {
			const reportedBy: Record<string, number> = { [GOOD]: chainId, [OTHER]: 1 }
			const connected: string[][] = []
			return {
				connected,
				io: {
					probeChainId: async (url: string) => {
						if (!(url in reportedBy)) throw new Error("fetch failed")
						return reportedBy[url]
					},
					connect: (urls: string[]) => {
						connected.push(urls)
						return rpc
					},
				},
			}
		}

		it("refuses a URL that serves another chain, and caches nothing from it", async () => {
			const chainId = 9000 + ++chains
			const rpc = client(1n)
			const { io, connected } = wizardIo(chainId, rpc)

			await expect(readChainFundingFromUnverifiedRpcs([GOOD, OTHER], chainId, config, WALLET, io)).rejects.toThrow(
				`RPC endpoint 2 serves chain 1, not chain ${chainId}`,
			)
			expect(connected).toEqual([])
			expect(rpc.readContract).not.toHaveBeenCalled()

			// The next, correct read still asks the host for its fee token: nothing was pinned.
			await readChainFundingFromUnverifiedRpcs([GOOD], chainId, config, WALLET, io)
			expect(rpc.readContract.mock.calls.map(([call]) => call.functionName)).toContain("feeToken")
		})

		it("reads through the URLs that answered, leaving an unreachable one out", async () => {
			const chainId = 9000 + ++chains
			const { io, connected } = wizardIo(chainId, client(parseUnits("250", 18)))

			const funding = await readChainFundingFromUnverifiedRpcs([DEAD, GOOD], chainId, config, WALLET, io)

			expect(connected).toEqual([[GOOD]])
			expect(funding.feeToken).toEqual({ symbol: "USDT", balance: 250 })
		})

		it("fails clearly when no URL answers", async () => {
			const chainId = 9000 + ++chains
			const { io, connected } = wizardIo(chainId, client(1n))

			await expect(readChainFundingFromUnverifiedRpcs([DEAD], chainId, config, WALLET, io)).rejects.toThrow(
				`None of the RPC endpoints for chain ${chainId} answered`,
			)
			expect(connected).toEqual([])
		})
	})

	it("uses a native balance it is given instead of reading one", async () => {
		const rpc = client(1n)
		await readChainFunding(rpc, config, WALLET, nextChain(), 5n)
		expect(rpc.getBalance).not.toHaveBeenCalled()
		expect(vi.mocked(nativeSetupNeed).mock.calls[0][4]).toBe(5n)
	})
})

describe("fundingShortfall", () => {
	const funding = (balance: number, required?: number): ChainFunding => ({
		feeToken: { symbol: "USDT", balance },
		nativeSetup: required ? { required, feeToken: "USDT" } : null,
		funded: balance >= 5 && !required,
		delegated: false,
	})

	it("is undefined once the chain is funded", () => {
		expect(fundingShortfall(funding(250), "BNB")).toBeUndefined()
	})

	it.each([
		[funding(0), "at least 5 USDT"],
		[funding(4.99), "at least 5 USDT"],
		[funding(250, 0.000142), "about 0.00014 BNB"],
		[funding(0, 0.000142), "at least 5 USDT and about 0.00014 BNB"],
	])("names what is missing", (shortfall, text) => {
		expect(fundingShortfall(shortfall, "BNB")).toBe(text)
	})
})
