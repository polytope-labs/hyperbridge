import { beforeEach, describe, expect, it, vi } from "vitest"
import type { PublicClient, WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { buildPaymasterAndData } from "@/services/paymaster"
import {
	DEPOSIT_HEADROOM_PERCENT,
	POST_OP_GAS_LIMIT_SIMPLEX,
	VERIFICATION_GAS_LIMIT_PERMIT2,
} from "@/services/paymaster/types"
import { buildSimplexPaymasterData } from "@/services/paymaster/provider/simplex"

vi.mock("@/services/paymaster/provider/simplex", () => ({
	buildSimplexPaymasterData: vi.fn(async () => ({
		paymaster: "0x000000000000000000000000000000000051391e" as HexString,
		paymasterData: "0x" as HexString,
		paymasterVerificationGasLimit: 200_000n,
		paymasterPostOpGasLimit: 40_000n,
		token: "0x000000000000000000000000000000000000a5d0" as HexString,
	})),
}))

const SIMPLEX = "0x000000000000000000000000000000000051391e" as HexString
const ENTRY_POINT = "0x0000000000000000000000000000000000004337" as HexString
const USDC = "0x000000000000000000000000000000000000a5d0" as HexString
const SOLVER = "0x0000000000000000000000000000000000501e13" as HexString

const BASE_GAS = 1_000_000n
const MAX_FEE = 1_000_000n
const PREFUND = { baseGas: BASE_GAS, maxFeePerGas: MAX_FEE }

const requiredFor = (pmGas: bigint) => ((BASE_GAS + pmGas) * MAX_FEE * DEPOSIT_HEADROOM_PERCENT) / 100n
const SIMPLEX_REQUIRED = requiredFor(VERIFICATION_GAS_LIMIT_PERMIT2 + POST_OP_GAS_LIMIT_SIMPLEX)

const AMPLE = SIMPLEX_REQUIRED * 2n

function configService(
	entryPoint: HexString | null = ENTRY_POINT,
	opts: { simplexConfigured?: boolean } = {},
): FillerConfigService {
	return {
		getSimplexPaymasterAddress: () => (opts.simplexConfigured === false ? undefined : SIMPLEX),
		getUsdcAsset: () => USDC,
		getUsdcDecimals: () => 6,
		getEntryPointAddress: () => entryPoint ?? undefined,
	} as unknown as FillerConfigService
}

function client(opts: { simplexDeposit?: bigint; depositError?: boolean }) {
	const readContract = vi.fn(
		async ({ address, functionName, args }: { address: HexString; functionName: string; args: unknown[] }) => {
			if (address === ENTRY_POINT && functionName === "balanceOf") {
				if (opts.depositError) throw new Error("rpc down")
				if (args[0] === SIMPLEX) return opts.simplexDeposit ?? AMPLE
			}
			throw new Error(`unexpected read: ${address}.${functionName}`)
		},
	)
	return { readContract } as unknown as PublicClient & { readContract: ReturnType<typeof vi.fn> }
}

function options(
	publicClient: PublicClient,
	overrides: {
		prefund?: typeof PREFUND
		configService?: FillerConfigService
		logger?: { warn: ReturnType<typeof vi.fn> }
	} = {},
) {
	return {
		chain: "EVM-8453",
		solverAccount: SOLVER,
		publicClient,
		walletClient: {} as unknown as WalletClient,
		signer: { signTypedData: async () => "0x" as HexString },
		configService: overrides.configService ?? configService(),
		prefund: "prefund" in overrides ? overrides.prefund : PREFUND,
		logger: overrides.logger,
	}
}

beforeEach(() => {
	vi.clearAllMocks()
})

describe("buildPaymasterAndData deposit-aware selection", () => {
	it("picks Simplex when its deposit suffices", async () => {
		const result = await buildPaymasterAndData(options(client({})))
		expect(result.type).toBe("simplex")
		expect(result.address).toBe(SIMPLEX)
	})

	it("prices the deposit gate against the Permit2 verification limit", async () => {
		const result = await buildPaymasterAndData(options(client({ simplexDeposit: SIMPLEX_REQUIRED - 1n })))
		expect(result.type).toBe("none")
		expect(result.reason).toBe(
			`simplex: EntryPoint deposit ${SIMPLEX_REQUIRED - 1n} < ${SIMPLEX_REQUIRED} required`,
		)
		expect(buildSimplexPaymasterData).not.toHaveBeenCalled()
	})

	it("returns none carrying the reason when the builder throws", async () => {
		vi.mocked(buildSimplexPaymasterData).mockRejectedValueOnce(
			new Error("SimplexPaymaster needs a one-time Permit2 approval"),
		)
		const logger = { warn: vi.fn() }
		const result = await buildPaymasterAndData(options(client({}), { logger }))
		expect(result.type).toBe("none")
		expect(result.reason).toContain("simplex: SimplexPaymaster needs a one-time Permit2 approval")
		expect(logger.warn).toHaveBeenCalledOnce()
	})

	it("returns none when Simplex has no eligible stablecoin", async () => {
		vi.mocked(buildSimplexPaymasterData).mockResolvedValueOnce({
			insufficient: [{ symbol: "USDC", balance: 0n, required: 1_000_000n }],
		})
		const result = await buildPaymasterAndData(options(client({})))
		expect(result.type).toBe("none")
		expect(result.reason).toContain("simplex: solver USDC balance 0 < 1000000")
	})

	it("skips the deposit check entirely when no prefund is given", async () => {
		const publicClient = client({ simplexDeposit: 0n })
		const result = await buildPaymasterAndData(options(publicClient, { prefund: undefined }))
		expect(result.type).toBe("simplex")
		expect(publicClient.readContract.mock.calls).toHaveLength(0)
	})

	it("fails open when the deposit read errors", async () => {
		const logger = { warn: vi.fn() }
		const result = await buildPaymasterAndData(options(client({ depositError: true }), { logger }))
		expect(result.type).toBe("simplex")
		expect(logger.warn).toHaveBeenCalledOnce()
	})

	it("accepts a deposit exactly at the headroom boundary", async () => {
		const result = await buildPaymasterAndData(options(client({ simplexDeposit: SIMPLEX_REQUIRED })))
		expect(result.type).toBe("simplex")
	})

	it("skips the deposit check when no EntryPoint is configured", async () => {
		const publicClient = client({ simplexDeposit: 0n })
		const result = await buildPaymasterAndData(options(publicClient, { configService: configService(null) }))
		expect(result.type).toBe("simplex")
		expect(publicClient.readContract.mock.calls).toHaveLength(0)
	})

	it("returns none without reading anything on a chain with no Simplex paymaster", async () => {
		const publicClient = client({})
		const result = await buildPaymasterAndData(
			options(publicClient, { configService: configService(ENTRY_POINT, { simplexConfigured: false }) }),
		)
		expect(result.type).toBe("none")
		expect(result.reason).toBe("no paymaster configured")
		expect(buildSimplexPaymasterData).not.toHaveBeenCalled()
		expect(publicClient.readContract.mock.calls).toHaveLength(0)
	})

	it("reports each fee token's balance when nothing is eligible", async () => {
		vi.mocked(buildSimplexPaymasterData).mockResolvedValueOnce({
			insufficient: [
				{ symbol: "USDC", balance: 999_999n, required: 1_000_000n },
				{ symbol: "USDT", balance: 0n, required: 1_000_000n },
			],
		})
		const result = await buildPaymasterAndData(options(client({})))
		expect(result.type).toBe("none")
		expect(result.reason).toBe("simplex: solver USDC balance 999999 < 1000000, USDT balance 0 < 1000000")
	})

	it("names the missing fee token config when the builder had nothing to read", async () => {
		vi.mocked(buildSimplexPaymasterData).mockResolvedValueOnce({ insufficient: [] })
		const result = await buildPaymasterAndData(options(client({})))
		expect(result.type).toBe("none")
		expect(result.reason).toBe("simplex: no fee token configured")
	})
})
