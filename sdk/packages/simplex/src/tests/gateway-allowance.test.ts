import { describe, expect, it, vi } from "vitest"
import { decodeFunctionData, keccak256, toHex } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import { ERC20_ABI } from "@/config/abis/ERC20"
import { chainConfigs } from "@hyperbridge/sdk"
import {
	RETIRED_GATEWAY_ADDRESSES,
	revokeStaleGatewayAllowances,
	type RetiredGateways,
	type StaleGatewayAllowanceDeps,
} from "@/services/gateway-allowance"
import type { SubmittedUserOp } from "@/services/submission"
import type { TokenSender } from "@/services/TokenSender"

/**
 * Fills used to approve the gateway for the destination fee token in the wrong decimals and never
 * reset it, so a solver that ran that code can hold a standing allowance on a chain no fill has
 * touched since. Boot clears it with one `approve(gateway, 0)` per token, and does nothing once
 * it has.
 */

const SOLVER = "0x5b2c3e25243634732eE94525Ae98aEc404c82506" as HexString
const GATEWAY = "0x1111111111111111111111111111111111111111" as HexString
const RETIRED = "0x5555555555555555555555555555555555555555" as HexString
const RETIRED_TOO = "0x6666666666666666666666666666666666666666" as HexString
const FEE_TOKEN = "0x2222222222222222222222222222222222222222" as HexString
const USDC = "0x3333333333333333333333333333333333333333" as HexString
const USDT = "0x4444444444444444444444444444444444444444" as HexString
const STALE = 10_900_000_000_000_000n
/** The revoke's own EntryPoint nonce lane: a uint192 key, so it never shares key 0 with boot-time senders. */
const REVOKE_NONCE_KEY = BigInt(keccak256(toHex("simplex.gateway-allowance-revoke"))) >> 64n
/** What `submitCalls` reports of the op it sent; nothing here reads it. */
const SUBMISSION: SubmittedUserOp = {
	kind: "userOp",
	userOpHash: "0xrevokeop" as HexString,
	nonce: 0n,
	nonceKey: REVOKE_NONCE_KEY,
	expires: false,
	submittedAt: 0,
}

function build(
	opts: {
		/** Allowance by lowercased token address; anything absent reads zero. */
		allowances?: Record<string, bigint>
		/** Allowance by lowercased token address to a retired gateway, keyed by lowercased gateway. */
		retiredAllowances?: Record<string, Record<string, bigint>>
		retiredGateways?: RetiredGateways
		chainIds?: number[]
		watchOnly?: Record<number, boolean>
		usdt?: HexString
		feeToken?: () => Promise<{ address: HexString; decimals: number }>
		submitCalls?: TokenSender["submitCalls"]
		readContract?: (call: { address: HexString; args: [HexString, HexString] }) => Promise<bigint>
	} = {},
) {
	const allowances = opts.allowances ?? {}
	const readContract = vi.fn(
		opts.readContract ??
			(async ({ address, args }: { address: HexString; args: [HexString, HexString] }) => {
				const retired = opts.retiredAllowances?.[args[1].toLowerCase()]
				if (retired) return retired[address.toLowerCase()] ?? 0n
				return args[1] === GATEWAY ? (allowances[address.toLowerCase()] ?? 0n) : 0n
			}),
	)
	const submitCalls = vi.fn(
		opts.submitCalls ??
			(async () => ({ txHash: "0xrevoke" as HexString, sponsored: true, submission: SUBMISSION })),
	)
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
		// No chain here has a retired gateway unless a test says so; the real list is checked on its own.
		retiredGateways: opts.retiredGateways ?? {},
		logger,
	}
	return { deps, readContract, submitCalls, logger }
}

describe("revokeStaleGatewayAllowances", () => {
	it("sends one approve(gateway, 0) for a token whose allowance is still standing", async () => {
		const { deps, submitCalls, logger } = build({ allowances: { [FEE_TOKEN.toLowerCase()]: STALE } })

		await revokeStaleGatewayAllowances(deps)

		expect(submitCalls).toHaveBeenCalledOnce()
		const [chain, calls, opts] = submitCalls.mock.calls[0]
		expect(chain).toBe("EVM-8453")
		// One call per submission, so the native fallback goes straight to the token and works
		// from a wallet that is not delegated yet.
		expect(calls).toHaveLength(1)
		expect(opts).toEqual({ nonceKey: REVOKE_NONCE_KEY })
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

	it("sends every revoke on the one nonce key, which fits the EntryPoint's uint192", async () => {
		const { deps, submitCalls } = build({
			allowances: { [FEE_TOKEN.toLowerCase()]: STALE, [USDT.toLowerCase()]: 1n },
		})

		await revokeStaleGatewayAllowances(deps)

		expect(submitCalls.mock.calls.map(([, , opts]) => opts?.nonceKey)).toEqual([REVOKE_NONCE_KEY, REVOKE_NONCE_KEY])
		expect(REVOKE_NONCE_KEY).toBeGreaterThan(0n)
		expect(REVOKE_NONCE_KEY).toBeLessThan(1n << 192n)
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

describe("retired gateways", () => {
	const retiredOn8453 = { 8453: [RETIRED] }

	it("revokes a standing allowance to a retired gateway", async () => {
		const { deps, submitCalls, logger } = build({
			retiredGateways: retiredOn8453,
			retiredAllowances: { [RETIRED.toLowerCase()]: { [USDC.toLowerCase()]: STALE } },
		})

		await revokeStaleGatewayAllowances(deps)

		expect(submitCalls).toHaveBeenCalledOnce()
		const [chain, calls, opts] = submitCalls.mock.calls[0]
		expect(chain).toBe("EVM-8453")
		expect(opts).toEqual({ nonceKey: REVOKE_NONCE_KEY })
		expect(calls).toHaveLength(1)
		expect(calls[0].target).toBe(USDC)
		expect(decodeFunctionData({ abi: ERC20_ABI, data: calls[0].data })).toEqual({
			functionName: "approve",
			args: [RETIRED, 0n],
		})
		expect(logger.info).toHaveBeenCalledWith(
			expect.objectContaining({ chain: "EVM-8453", token: USDC, gateway: RETIRED, retired: true }),
			"Revoked a stale IntentGateway allowance",
		)
	})

	it("reads the same fee token, USDC and USDT for the retired gateway", async () => {
		const { deps, readContract } = build({ retiredGateways: retiredOn8453 })

		await revokeStaleGatewayAllowances(deps)

		expect(readContract.mock.calls.map(([call]) => [call.address, call.args[1]])).toEqual([
			[FEE_TOKEN, GATEWAY],
			[USDC, GATEWAY],
			[USDT, GATEWAY],
			[FEE_TOKEN, RETIRED],
			[USDC, RETIRED],
			[USDT, RETIRED],
		])
	})

	it("sends nothing when every retired allowance reads zero", async () => {
		const { deps, submitCalls, logger } = build({ retiredGateways: retiredOn8453 })

		await revokeStaleGatewayAllowances(deps)

		expect(submitCalls).not.toHaveBeenCalled()
		expect(logger.warn).not.toHaveBeenCalled()
	})

	it("revokes the current and the retired gateway one after another on the one nonce key", async () => {
		let inFlight = 0
		let overlap = false
		const { deps, submitCalls } = build({
			retiredGateways: { 8453: [RETIRED, RETIRED_TOO] },
			allowances: { [FEE_TOKEN.toLowerCase()]: STALE },
			retiredAllowances: {
				[RETIRED.toLowerCase()]: { [USDT.toLowerCase()]: 1n },
				[RETIRED_TOO.toLowerCase()]: { [USDC.toLowerCase()]: 1n },
			},
			submitCalls: async () => {
				overlap ||= ++inFlight > 1
				await new Promise((resolve) => setTimeout(resolve, 1))
				inFlight--
				return { txHash: "0xrevoke" as HexString, sponsored: true, submission: SUBMISSION }
			},
		})

		await revokeStaleGatewayAllowances(deps)

		expect(overlap).toBe(false)
		expect(
			submitCalls.mock.calls.map(([, calls, opts]) => [
				calls[0].target,
				decodeFunctionData({ abi: ERC20_ABI, data: calls[0].data }).args?.[0],
				opts?.nonceKey,
			]),
		).toEqual([
			[FEE_TOKEN, GATEWAY, REVOKE_NONCE_KEY],
			[USDT, RETIRED, REVOKE_NONCE_KEY],
			[USDC, RETIRED_TOO, REVOKE_NONCE_KEY],
		])
	})

	it("skips a retired address that is the current gateway, whatever its case", async () => {
		const { deps, readContract } = build({
			retiredGateways: { 8453: [GATEWAY.toUpperCase().replace("0X", "0x") as HexString, RETIRED, RETIRED] },
		})

		await revokeStaleGatewayAllowances(deps)

		const spenders = readContract.mock.calls.map(([call]) => call.args[1])
		expect(spenders.filter((spender) => spender === GATEWAY)).toHaveLength(3)
		expect(spenders.filter((spender) => spender === RETIRED)).toHaveLength(3)
		expect(spenders).toHaveLength(6)
	})

	it("reads nothing extra on a chain with no retired gateway", async () => {
		const { deps, readContract } = build({ chainIds: [56], retiredGateways: retiredOn8453 })

		await revokeStaleGatewayAllowances(deps)

		expect(readContract.mock.calls.map(([call]) => [call.address, call.args[1]])).toEqual(
			[FEE_TOKEN, USDC, USDT].map((token) => [token, GATEWAY]),
		)
	})

	it("leaves a watch-only chain's retired gateways alone too", async () => {
		const { deps, readContract } = build({
			watchOnly: { 8453: true },
			retiredGateways: retiredOn8453,
		})

		await revokeStaleGatewayAllowances(deps)

		expect(readContract).not.toHaveBeenCalled()
	})

	it("warns and carries on to the next retired gateway when one revoke fails", async () => {
		const { deps, submitCalls, logger } = build({
			retiredGateways: { 8453: [RETIRED, RETIRED_TOO] },
			retiredAllowances: {
				[RETIRED.toLowerCase()]: { [USDC.toLowerCase()]: 1n },
				[RETIRED_TOO.toLowerCase()]: { [USDC.toLowerCase()]: 1n },
			},
			submitCalls: vi
				.fn()
				.mockRejectedValueOnce(new Error("insufficient funds"))
				.mockResolvedValueOnce({ txHash: "0xok", sponsored: false }),
		})

		await expect(revokeStaleGatewayAllowances(deps)).resolves.toBeUndefined()

		expect(submitCalls).toHaveBeenCalledTimes(2)
		expect(logger.warn).toHaveBeenCalledWith(
			expect.objectContaining({ gateway: RETIRED, retired: true }),
			"Could not revoke a stale IntentGateway allowance",
		)
		expect(logger.info).toHaveBeenCalledWith(
			expect.objectContaining({ gateway: RETIRED_TOO, retired: true }),
			"Revoked a stale IntentGateway allowance",
		)
	})
})

describe("RETIRED_GATEWAY_ADDRESSES", () => {
	it("holds only addresses the chain no longer uses, each once", () => {
		for (const [chainId, retired] of Object.entries(RETIRED_GATEWAY_ADDRESSES)) {
			const current = chainConfigs[Number(chainId)]?.addresses.IntentGateway
			expect(current, `chain ${chainId} has a gateway configured`).toBeTruthy()
			const lowered = retired.map((address) => address.toLowerCase())
			expect(lowered, `chain ${chainId}`).not.toContain(String(current).toLowerCase())
			expect(new Set(lowered).size).toBe(lowered.length)
		}
	})

	it("covers the gateway mainnet left in #996", () => {
		const previous = "0x16F9E57f735bBfF9f6c4E5276330f9c437d0e9E0".toLowerCase()
		for (const chainId of [1, 10, 56, 100, 137, 8453, 42161, 420420419]) {
			expect(RETIRED_GATEWAY_ADDRESSES[chainId]?.map((address) => address.toLowerCase())).toContain(previous)
		}
	})
})
