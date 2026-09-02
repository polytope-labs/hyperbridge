import { beforeEach, describe, expect, it, vi } from "vitest"
import type { PublicClient, WalletClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { buildPaymasterAndData } from "@/services/paymaster"
import {
	DEPOSIT_HEADROOM_PERCENT,
	POST_OP_GAS_LIMIT_CIRCLE,
	POST_OP_GAS_LIMIT_SIMPLEX,
	VERIFICATION_GAS_LIMIT_CIRCLE,
	VERIFICATION_GAS_LIMIT_PERMIT,
} from "@/services/paymaster/types"
import { buildCirclePaymasterData } from "@/services/paymaster/provider/circle"
import { buildSimplexPaymasterData } from "@/services/paymaster/provider/simplex"

vi.mock("@/services/paymaster/provider/circle", () => ({
	buildCirclePaymasterData: vi.fn(async () => ({
		paymaster: "0x00000000000000000000000000000000000c17c1" as HexString,
		paymasterData: "0x" as HexString,
		paymasterVerificationGasLimit: 200_000n,
		paymasterPostOpGasLimit: 100_000n,
	})),
}))

vi.mock("@/services/paymaster/provider/simplex", () => ({
	buildSimplexPaymasterData: vi.fn(async () => ({
		paymaster: "0x000000000000000000000000000000000051391e" as HexString,
		paymasterData: "0x" as HexString,
		paymasterVerificationGasLimit: 250_000n,
		paymasterPostOpGasLimit: 40_000n,
		token: "0x000000000000000000000000000000000000a5d0" as HexString,
	})),
}))

const CIRCLE = "0x00000000000000000000000000000000000c17c1" as HexString
const SIMPLEX = "0x000000000000000000000000000000000051391e" as HexString
const ENTRY_POINT = "0x0000000000000000000000000000000000004337" as HexString
const USDC = "0x000000000000000000000000000000000000a5d0" as HexString
const SOLVER = "0x0000000000000000000000000000000000501e13" as HexString

const BASE_GAS = 1_000_000n
const MAX_FEE = 1_000_000n
const PREFUND = { baseGas: BASE_GAS, maxFeePerGas: MAX_FEE }

const requiredFor = (pmGas: bigint) => ((BASE_GAS + pmGas) * MAX_FEE * DEPOSIT_HEADROOM_PERCENT) / 100n
const CIRCLE_REQUIRED = requiredFor(VERIFICATION_GAS_LIMIT_CIRCLE + POST_OP_GAS_LIMIT_CIRCLE)
const SIMPLEX_REQUIRED = requiredFor(VERIFICATION_GAS_LIMIT_PERMIT + POST_OP_GAS_LIMIT_SIMPLEX)

const AMPLE = CIRCLE_REQUIRED + SIMPLEX_REQUIRED

function configService(
	entryPoint: HexString | null = ENTRY_POINT,
	opts: { simplexConfigured?: boolean } = {},
): FillerConfigService {
	return {
		getCirclePaymasterAddress: () => CIRCLE,
		getSimplexPaymasterAddress: () => (opts.simplexConfigured === false ? undefined : SIMPLEX),
		getUsdcAsset: () => USDC,
		getUsdcDecimals: () => 6,
		getEntryPointAddress: () => entryPoint ?? undefined,
	} as unknown as FillerConfigService
}

function client(opts: { solverUsdc?: bigint; circleDeposit?: bigint; simplexDeposit?: bigint; depositError?: boolean }) {
	const readContract = vi.fn(
		async ({ address, functionName, args }: { address: HexString; functionName: string; args: unknown[] }) => {
			if (address === USDC && functionName === "balanceOf") return opts.solverUsdc ?? 2_000_000n
			if (address === ENTRY_POINT && functionName === "balanceOf") {
				if (opts.depositError) throw new Error("rpc down")
				if (args[0] === CIRCLE) return opts.circleDeposit ?? AMPLE
				if (args[0] === SIMPLEX) return opts.simplexDeposit ?? AMPLE
			}
			throw new Error(`unexpected read: ${address}.${functionName}`)
		},
	)
	return { readContract } as unknown as PublicClient & { readContract: ReturnType<typeof vi.fn> }
}

function options(
	publicClient: PublicClient,
	overrides: { prefund?: typeof PREFUND; configService?: FillerConfigService; logger?: { warn: ReturnType<typeof vi.fn> } } = {},
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
		expect(buildCirclePaymasterData).not.toHaveBeenCalled()
	})

	it("falls through to Circle when the Simplex deposit is one wei short", async () => {
		const logger = { warn: vi.fn() }
		const result = await buildPaymasterAndData(
			options(client({ simplexDeposit: SIMPLEX_REQUIRED - 1n }), { logger }),
		)
		expect(result.type).toBe("circle")
		expect(result.address).toBe(CIRCLE)
		expect(buildSimplexPaymasterData).not.toHaveBeenCalled()
		expect(logger.warn).toHaveBeenCalledOnce()
	})

	it("falls through to Circle when Simplex has no eligible stablecoin", async () => {
		vi.mocked(buildSimplexPaymasterData).mockResolvedValueOnce(null)
		const result = await buildPaymasterAndData(options(client({})))
		expect(result.type).toBe("circle")
		expect(result.address).toBe(CIRCLE)
		expect(buildSimplexPaymasterData).toHaveBeenCalledOnce()
	})

	it("returns none, naming both shortfalls, without invoking either builder", async () => {
		const result = await buildPaymasterAndData(
			options(client({ circleDeposit: CIRCLE_REQUIRED - 1n, simplexDeposit: SIMPLEX_REQUIRED - 1n })),
		)
		expect(result.type).toBe("none")
		expect(result.reason).toContain(`simplex: EntryPoint deposit ${SIMPLEX_REQUIRED - 1n} < ${SIMPLEX_REQUIRED}`)
		expect(result.reason).toContain(`circle: EntryPoint deposit ${CIRCLE_REQUIRED - 1n} < ${CIRCLE_REQUIRED}`)
		expect(buildCirclePaymasterData).not.toHaveBeenCalled()
		expect(buildSimplexPaymasterData).not.toHaveBeenCalled()
	})

	it("skips the deposit check entirely when no prefund is given", async () => {
		const publicClient = client({ circleDeposit: 0n, simplexDeposit: 0n })
		const result = await buildPaymasterAndData(options(publicClient, { prefund: undefined }))
		expect(result.type).toBe("simplex")
		const entryPointReads = publicClient.readContract.mock.calls.filter(
			(call) => (call[0] as { address: HexString }).address === ENTRY_POINT,
		)
		expect(entryPointReads).toHaveLength(0)
	})

	it("still gates Circle on the solver USDC balance before its deposit when Simplex is skipped", async () => {
		const publicClient = client({ simplexDeposit: SIMPLEX_REQUIRED - 1n, solverUsdc: 999_999n, circleDeposit: 0n })
		const result = await buildPaymasterAndData(options(publicClient))
		expect(result.type).toBe("none")
		expect(result.reason).toContain("circle: solver USDC balance 999999 < 1000000")
		expect(buildCirclePaymasterData).not.toHaveBeenCalled()
		// Circle's deposit is never read: the balance gate rejects first
		const circleDepositReads = publicClient.readContract.mock.calls.filter(
			(call) =>
				(call[0] as { address: HexString; args: unknown[] }).address === ENTRY_POINT &&
				(call[0] as { args: unknown[] }).args[0] === CIRCLE,
		)
		expect(circleDepositReads).toHaveLength(0)
	})

	it("fails open when the deposit read errors", async () => {
		const logger = { warn: vi.fn() }
		const result = await buildPaymasterAndData(options(client({ depositError: true }), { logger }))
		expect(result.type).toBe("simplex")
		expect(logger.warn).toHaveBeenCalledOnce()
	})

	it("accepts a deposit exactly at the headroom boundary", async () => {
		const result = await buildPaymasterAndData(options(client({ simplexDeposit: SIMPLEX_REQUIRED, circleDeposit: 0n })))
		expect(result.type).toBe("simplex")
	})

	it("skips the deposit check when no EntryPoint is configured", async () => {
		const publicClient = client({ simplexDeposit: 0n })
		const result = await buildPaymasterAndData(
			options(publicClient, { configService: configService(null) }),
		)
		expect(result.type).toBe("simplex")
		const entryPointReads = publicClient.readContract.mock.calls.filter(
			(call) => (call[0] as { address: HexString }).address === ENTRY_POINT,
		)
		expect(entryPointReads).toHaveLength(0)
	})

	it("picks Circle on a chain with no Simplex paymaster configured", async () => {
		const result = await buildPaymasterAndData(
			options(client({}), { configService: configService(ENTRY_POINT, { simplexConfigured: false }) }),
		)
		expect(result.type).toBe("circle")
		expect(result.address).toBe(CIRCLE)
		expect(buildSimplexPaymasterData).not.toHaveBeenCalled()
	})

	it("reports both balance and deposit skip reasons when nothing is eligible", async () => {
		vi.mocked(buildSimplexPaymasterData).mockResolvedValueOnce(null)
		const result = await buildPaymasterAndData(options(client({ solverUsdc: 0n })))
		expect(result.type).toBe("none")
		expect(result.reason).toContain("simplex: insufficient stablecoin balance")
		expect(result.reason).toContain("circle: solver USDC balance 0 < 1000000")
	})
})
