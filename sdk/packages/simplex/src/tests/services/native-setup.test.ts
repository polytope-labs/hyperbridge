import { beforeEach, describe, expect, it, vi } from "vitest"
import { concat, formatEther, type PublicClient } from "viem"
import type { HexString } from "@hyperbridge/sdk"
import type { FillerConfigService } from "@/services/FillerConfigService"
import { nativeSetupNeed } from "@/services/native-setup"
import { APPROVE_TX_GAS, resolvePendingPermit2Approval } from "@/services/paymaster/provider/simplex"
import { BATCHED_APPROVE_GAS, DELEGATION_TX_GAS_FLOOR } from "@/services/DelegationService"

vi.mock("@/services/paymaster/provider/simplex", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/services/paymaster/provider/simplex")>()),
	resolvePendingPermit2Approval: vi.fn(),
}))

const WALLET = "0x7aae193f22bb500e38a6f970deafc46f58523c4c" as HexString
const PAYMASTER = "0xeD02f9f0df8F562B89cC5b25867Ad3C2d61252A9" as HexString
const SOLVER_ACCOUNT = "0xaAd062555800a97Af062795189e32a3CBd045612" as HexString
const PERMIT2 = "0x000000000022D473030F116dDEE9F6B43aC78BA3" as HexString
const USDC = "0x8ac76a51cc950d9822d68b83fe1ad97b32cd580d" as HexString
const USDT = "0x55d398326f99059ff775485246999027b3197955" as HexString
const GAS_PRICE = 100_000_000n // 0.1 gwei, as on BNB Chain
const DELEGATE_AND_APPROVE = (DELEGATION_TX_GAS_FLOOR + BATCHED_APPROVE_GAS) * GAS_PRICE
const APPROVE = APPROVE_TX_GAS * GAS_PRICE

const config = (paymaster: HexString | null = PAYMASTER) =>
	({
		getSimplexPaymasterAddress: () => paymaster ?? undefined,
		getSolverAccountContractAddress: () => SOLVER_ACCOUNT,
		getUsdcAsset: () => USDC,
	}) as unknown as FillerConfigService

function client(code: HexString | undefined) {
	return {
		getGasPrice: vi.fn(async () => GAS_PRICE),
		getCode: vi.fn(async () => code),
	} as unknown as PublicClient & { getCode: ReturnType<typeof vi.fn> }
}

const delegated = concat(["0xef0100", SOLVER_ACCOUNT])

describe("nativeSetupNeed", () => {
	beforeEach(() => {
		vi.mocked(resolvePendingPermit2Approval).mockReset()
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDC,
			spender: PERMIT2,
			permitCapable: false,
		})
	})

	it("asks for a delegation and the approve when the wallet has not delegated yet", async () => {
		const need = await nativeSetupNeed(client(undefined), config(), WALLET, "EVM-56", 0n)
		expect(need).toEqual({
			required: Number(formatEther(DELEGATE_AND_APPROVE * 2n)),
			feeToken: "USDC",
		})
	})

	it("asks only for the approve once the wallet is delegated", async () => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue({
			token: USDT,
			spender: PERMIT2,
			permitCapable: false,
		})
		const need = await nativeSetupNeed(client(delegated), config(), WALLET, "EVM-56", 0n)
		expect(need).toEqual({ required: Number(formatEther(APPROVE * 2n)), feeToken: "USDT" })
	})

	it("asks nothing once the wallet holds enough for its remaining setup", async () => {
		expect(await nativeSetupNeed(client(delegated), config(), WALLET, "EVM-56", APPROVE)).toBeNull()
	})

	it("asks nothing, and reads nothing else, when the wallet covers the costliest setup", async () => {
		const rpc = client(undefined)
		expect(await nativeSetupNeed(rpc, config(), WALLET, "EVM-56", DELEGATE_AND_APPROVE)).toBeNull()
		expect(resolvePendingPermit2Approval).not.toHaveBeenCalled()
		expect(rpc.getCode).not.toHaveBeenCalled()
	})

	it.each([
		["the fee token has a permit", { token: USDC, spender: PERMIT2, permitCapable: true }],
		["the approval is in place or no fee token is held", null],
	])("asks nothing when %s", async (_case, pending) => {
		vi.mocked(resolvePendingPermit2Approval).mockResolvedValue(pending as never)
		expect(await nativeSetupNeed(client(undefined), config(), WALLET, "EVM-56", 0n)).toBeNull()
	})

	it("judges the token it is given, for a wallet that holds none yet", async () => {
		await nativeSetupNeed(client(undefined), config(), WALLET, "EVM-56", 0n, USDT)
		expect(vi.mocked(resolvePendingPermit2Approval).mock.calls[0][5]).toBe(USDT)
	})

	it("asks nothing on a chain without a paymaster", async () => {
		expect(await nativeSetupNeed(client(undefined), config(null), WALLET, "EVM-56", 0n)).toBeNull()
		expect(resolvePendingPermit2Approval).not.toHaveBeenCalled()
	})
})
