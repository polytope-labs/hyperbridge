import { paymasterReserveForToken, PAYMASTER_RESERVE_TOKENS } from "@/services/paymaster"
import type { FillerConfigService } from "@/services/FillerConfigService"
import type { HexString } from "@hyperbridge/sdk"
import { describe, it, expect } from "vitest"

// The reserve exists because the paymaster charges gas in USDC or USDT and
// pulls it from the solver's wallet during validatePaymasterUserOp, before the
// UserOp's callData runs. A partial fill sized to the whole balance is short by
// exactly that pull — the failure this guards against reverted a $14,808.699383
// fill on Base for 2.9 cents.

const CHAIN = "EVM-8453"
const USDC = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913" as HexString
const USDT = "0xfde4c96c8593536e31f229ea8f37b2ada2699bb2" as HexString
const EXOTIC = "0x2222222222222222222222222222222222222222" as HexString
const ZERO = "0x0000000000000000000000000000000000000000" as HexString

interface ChainAssets {
	usdc?: HexString
	usdcDecimals?: number
	usdt?: HexString
	usdtDecimals?: number
	paymaster?: boolean
}

function configFor(assets: ChainAssets): FillerConfigService {
	return {
		getUsdcAsset: () => assets.usdc ?? ZERO,
		getUsdtAsset: () => assets.usdt ?? ZERO,
		getUsdcDecimals: () => assets.usdcDecimals ?? 6,
		getUsdtDecimals: () => assets.usdtDecimals ?? 6,
		getCirclePaymasterAddress: () =>
			assets.paymaster === false ? undefined : ("0x0578cfb241215b77442a541325d6a4e6dfe700ec" as HexString),
		getSimplexPaymasterAddress: () => undefined,
	} as unknown as FillerConfigService
}

const reserve = (assets: ChainAssets, token: HexString): bigint =>
	paymasterReserveForToken(CHAIN, token.toLowerCase(), configFor(assets))

describe("paymasterReserveForToken", () => {
	it("reserves whole tokens of USDC at the chain's decimals", () => {
		expect(reserve({ usdc: USDC, usdcDecimals: 6 }, USDC)).toBe(PAYMASTER_RESERVE_TOKENS * 1_000_000n)
	})

	it("reserves USDT at its own decimals, not USDC's", () => {
		// USDT is 18 decimals on BSC and 6 on Base — a literal reserve would be
		// wrong by a factor of 10^12 on one of them.
		const assets = { usdc: USDC, usdcDecimals: 6, usdt: USDT, usdtDecimals: 18 }
		expect(reserve(assets, USDT)).toBe(PAYMASTER_RESERVE_TOKENS * 10n ** 18n)
		expect(reserve(assets, USDC)).toBe(PAYMASTER_RESERVE_TOKENS * 1_000_000n)
	})

	it("reserves nothing for a token the paymaster cannot charge in", () => {
		expect(reserve({ usdc: USDC, usdt: USDT }, EXOTIC)).toBe(0n)
	})

	it("reserves nothing when the chain has no paymaster", () => {
		expect(reserve({ usdc: USDC, paymaster: false }, USDC)).toBe(0n)
	})

	it("does not treat an unconfigured stablecoin as a match for the native token", () => {
		// Unconfigured assets read back as the zero address, and a native-token
		// leg carries the zero address too. Matching them would reserve a
		// stablecoin's worth of native balance against a paymaster that never
		// charges in it.
		expect(reserve({ usdc: USDC, usdt: undefined }, ZERO)).toBe(0n)
	})

	it("matches the configured asset case-insensitively", () => {
		const checksummed = `0x${USDC.slice(2).toUpperCase()}` as HexString
		expect(reserve({ usdc: checksummed }, USDC)).toBe(PAYMASTER_RESERVE_TOKENS * 1_000_000n)
	})
})
