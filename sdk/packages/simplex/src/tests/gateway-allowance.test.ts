import { describe, expect, it, vi } from "vitest"
import { decodeFunctionData } from "viem"
import type { ERC7821Call, HexString } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { revokeStaleGatewayAllowances, type StaleGatewayAllowanceDeps } from "@/services/gateway-allowance"

/**
 * Fills used to approve the gateway for the destination fee token in the wrong decimals and never
 * reset it, so a solver that ran that code can hold a standing allowance on a chain no fill has
 * touched since. Boot clears it with one `approve(gateway, 0)` per token, and does nothing once
 * it has.
 */

const SOLVER = "0x5b2c3e25243634732eE94525Ae98aEc404c82506" as HexString
const GATEWAY = "0x1111111111111111111111111111111111111111" as HexString
const FEE_TOKEN = "0x2222222222222222222222222222222222222222" as HexString
const USDC = "0x3333333333333333333333333333333333333333" as HexString
const USDT = "0x4444444444444444444444444444444444444444" as HexString
const STALE = 10_900_000_000_000_000n

function build(
	opts: {
		/** Allowance by lowercased token address; anything absent reads zero. */
		allowances?: Record<string, bigint>
		chainIds?: number[]
		watchOnly?: Record<number, boolean>
		usdt?: HexString
		feeToken?: () => Promise<{ address: HexString; decimals: number }>
		submitCalls?: (chain: string, calls: ERC7821Call[]) => Promise<{ txHash: HexString; sponsored: boolean }>
		readContract?: (call: { address: HexString }) => Promise<bigint>
	} = {},
) {
	const allowances = opts.allowances ?? {}
	const readContract = vi.fn(
		opts.readContract ?? (async ({ address }: { address: HexString }) => allowances[address.toLowerCase()] ?? 0n),
	)
	const submitCalls = vi.fn(opts.submitCalls ?? (async () => ({ txHash: "0xrevoke" as HexString, sponsored: true })))
	const logger = { info: vi.fn(), warn: vi.fn() }
	const deps: StaleGatewayAllowanceDeps = {
		clientManager: { getPublicClient: () => ({ readContract }) } as never,
		configService: {
			getConfiguredChainIds: () => opts.chainIds ?? [8453],
			getIntentGatewayAddress: () => GATEWAY,
			getUsdcAsset: () => USDC,
			getUsdtAsset: () => opts.usdt ?? USDT,
		},
		contractService: {
			getFeeTokenWithDecimals: opts.feeToken ?? (async () => ({ address: FEE_TOKEN, decimals: 6 })),
		},
		tokenSender: { submitCalls },
		solver: SOLVER,
		watchOnly: opts.watchOnly,
		logger,
	}
	return { deps, readContract, submitCalls, logger }
}

describe("revokeStaleGatewayAllowances", () => {
	it("sends one approve(gateway, 0) for a token whose allowance is still standing", async () => {
		const { deps, submitCalls, logger } = build({ allowances: { [FEE_TOKEN.toLowerCase()]: STALE } })

		await revokeStaleGatewayAllowances(deps)

		expect(submitCalls).toHaveBeenCalledOnce()
		const [chain, calls] = submitCalls.mock.calls[0]
		expect(chain).toBe("EVM-8453")
		// One call per submission, so an undelegated wallet sends it as a plain transaction.
		expect(calls).toHaveLength(1)
		expect(calls[0].target).toBe(FEE_TOKEN)
		expect(calls[0].value).toBe(0n)
		expect(decodeFunctionData({ abi: ERC20_ABI, data: calls[0].data })).toEqual({
			functionName: "approve",
			args: [GATEWAY, 0n],
		})
		expect(logger.info).toHaveBeenCalledWith(
			expect.objectContaining({ chain: "EVM-8453", token: FEE_TOKEN, allowance: STALE.toString() }),
			"Revoked a stale IntentGateway allowance",
		)
	})

	it("reads the allowance the solver granted the gateway on the fee token, USDC and USDT", async () => {
		const { deps, readContract } = build()

		await revokeStaleGatewayAllowances(deps)

		expect(readContract.mock.calls.map(([call]) => call)).toEqual(
			[FEE_TOKEN, USDC, USDT].map((address) =>
				expect.objectContaining({ address, functionName: "allowance", args: [SOLVER, GATEWAY] }),
			),
		)
	})

	it("revokes each token separately, and only the ones that still stand", async () => {
		const { deps, submitCalls } = build({
			allowances: { [FEE_TOKEN.toLowerCase()]: STALE, [USDT.toLowerCase()]: 1n },
		})

		await revokeStaleGatewayAllowances(deps)

		expect(submitCalls.mock.calls.map(([, calls]) => calls.map((call) => call.target))).toEqual([[FEE_TOKEN], [USDT]])
	})

	it("does nothing once every allowance reads zero, as on the boot after a revoke", async () => {
		const { deps, submitCalls, logger } = build()

		await revokeStaleGatewayAllowances(deps)

		expect(submitCalls).not.toHaveBeenCalled()
		expect(logger.warn).not.toHaveBeenCalled()
	})

	it("reads a token once when the fee token is also USDC", async () => {
		const { deps, readContract, submitCalls } = build({
			allowances: { [USDC.toLowerCase()]: STALE },
			feeToken: async () => ({ address: USDC, decimals: 6 }),
		})

		await revokeStaleGatewayAllowances(deps)

		expect(readContract).toHaveBeenCalledTimes(2)
		expect(submitCalls).toHaveBeenCalledOnce()
	})

	it("skips a USDT that the chain does not configure", async () => {
		const { deps, readContract } = build({ usdt: "0x" })

		await revokeStaleGatewayAllowances(deps)

		expect(readContract.mock.calls.map(([call]) => call.address)).toEqual([FEE_TOKEN, USDC])
	})

	it("leaves a watch-only chain alone and still clears the others", async () => {
		const { deps, readContract, submitCalls } = build({
			chainIds: [8453, 56],
			watchOnly: { 8453: true },
			allowances: { [FEE_TOKEN.toLowerCase()]: STALE },
		})

		await revokeStaleGatewayAllowances(deps)

		expect(readContract).toHaveBeenCalledTimes(3)
		expect(submitCalls.mock.calls.map(([chain]) => chain)).toEqual(["EVM-56"])
	})

	it("does nothing when every chain is watch-only", async () => {
		const { deps, readContract, submitCalls } = build({
			chainIds: [8453, 56],
			watchOnly: { 8453: true, 56: true },
			allowances: { [FEE_TOKEN.toLowerCase()]: STALE },
		})

		await revokeStaleGatewayAllowances(deps)

		expect(readContract).not.toHaveBeenCalled()
		expect(submitCalls).not.toHaveBeenCalled()
	})

	it("warns and carries on to the next token when a revoke fails", async () => {
		const { deps, submitCalls, logger } = build({
			allowances: { [FEE_TOKEN.toLowerCase()]: STALE, [USDC.toLowerCase()]: STALE },
			submitCalls: vi
				.fn()
				.mockRejectedValueOnce(new Error("insufficient funds"))
				.mockResolvedValueOnce({ txHash: "0xok", sponsored: false }),
		})

		await expect(revokeStaleGatewayAllowances(deps)).resolves.toBeUndefined()

		expect(submitCalls).toHaveBeenCalledTimes(2)
		expect(logger.warn).toHaveBeenCalledWith(
			expect.objectContaining({ token: FEE_TOKEN }),
			"Could not revoke a stale IntentGateway allowance",
		)
		expect(logger.info).toHaveBeenCalledOnce()
	})

	it("warns and carries on to the next chain when a chain's allowances cannot be read", async () => {
		const { deps, submitCalls, logger } = build({
			chainIds: [8453, 56],
			readContract: async () => {
				throw new Error("rpc down")
			},
		})
		deps.configService.getIntentGatewayAddress = (chain: string) => {
			if (chain === "EVM-8453") throw new Error("no gateway")
			return GATEWAY
		}

		await expect(revokeStaleGatewayAllowances(deps)).resolves.toBeUndefined()

		expect(submitCalls).not.toHaveBeenCalled()
		expect(logger.warn).toHaveBeenCalledWith(
			expect.objectContaining({ chain: "EVM-8453" }),
			"Could not check the gateway allowance, leaving it as it is",
		)
		// EVM-56 was still reached: each token's failed read is its own warning.
		expect(logger.warn).toHaveBeenCalledWith(
			expect.objectContaining({ chain: "EVM-56", token: FEE_TOKEN }),
			"Could not revoke a stale IntentGateway allowance",
		)
	})

	it("still checks USDC and USDT when the fee token cannot be read", async () => {
		const { deps, readContract, logger } = build({
			feeToken: async () => {
				throw new Error("host unreachable")
			},
		})

		await revokeStaleGatewayAllowances(deps)

		expect(readContract.mock.calls.map(([call]) => call.address)).toEqual([USDC, USDT])
		expect(logger.warn).toHaveBeenCalledWith(
			expect.objectContaining({ chain: "EVM-8453" }),
			"Could not read the gateway's fee token, checking USDC and USDT only",
		)
	})
})
