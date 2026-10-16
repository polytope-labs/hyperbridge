import { afterEach, describe, expect, it, vi } from "vitest"
import { InsufficientFundsError, RpcRequestError, TransactionExecutionError, parseEther } from "viem"
import { privateKeyToAccount } from "viem/accounts"
import { type SkipContext, requireFunded, skipIfUnfunded } from "./sequential/funding"

const ACCOUNT = "0x00000000000000000000000000000000000000aa"
const AMOY_POL = { chain: "Polygon Amoy", symbol: "POL", decimals: 18 }

class Skipped extends Error {}

function fakeContext(): SkipContext & { skip: ReturnType<typeof vi.fn> } {
	return {
		task: { name: "cancels an order" },
		skip: vi.fn((note?: string) => {
			throw new Skipped(note)
		}),
	} as never
}

function insufficientFunds(): TransactionExecutionError {
	const rpc = new RpcRequestError({
		body: {},
		url: "http://localhost",
		error: {
			code: -32000,
			message:
				"insufficient funds for gas * price + value: balance 1615829043237491017, tx cost 1826425200459650342",
		},
	})
	return new TransactionExecutionError(new InsufficientFundsError({ cause: rpc }), {
		account: privateKeyToAccount(`0x${"11".repeat(32)}`),
		chain: undefined,
	})
}

describe("live test funding", () => {
	afterEach(() => {
		vi.restoreAllMocks()
	})

	it("skips with a warning annotation when a balance is below its minimum", async () => {
		const write = vi.spyOn(process.stdout, "write").mockImplementation(() => true)
		const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
		const ctx = fakeContext()

		await expect(
			requireFunded(ctx, ACCOUNT, [
				{ ...AMOY_POL, min: parseEther("5"), read: async () => parseEther("1.5") },
				{ chain: "BSC Chapel", symbol: "tBNB", decimals: 18, min: 1n, read: async () => 2n },
			]),
		).rejects.toBeInstanceOf(Skipped)

		const message = `cancels an order: ${ACCOUNT} holds 1.5 POL on Polygon Amoy, needs at least 5 POL. Fund it to run this test.`
		expect(write).toHaveBeenCalledExactlyOnceWith(`\n::warning title=Unfunded live test::${message}\n`)
		expect(warn).toHaveBeenCalledExactlyOnceWith(message)
		expect(ctx.skip).toHaveBeenCalledExactlyOnceWith(message)
	})

	it("runs when every balance meets its minimum", async () => {
		const ctx = fakeContext()

		await requireFunded(ctx, ACCOUNT, [{ ...AMOY_POL, min: parseEther("5"), read: async () => parseEther("5") }])

		expect(ctx.skip).not.toHaveBeenCalled()
	})

	it("skips when a step fails with InsufficientFundsError inside TransactionExecutionError", async () => {
		const write = vi.spyOn(process.stdout, "write").mockImplementation(() => true)
		vi.spyOn(console, "warn").mockImplementation(() => {})
		const ctx = fakeContext()

		await expect(
			skipIfUnfunded(ctx, ACCOUNT, AMOY_POL, async () => {
				throw insufficientFunds()
			}),
		).rejects.toBeInstanceOf(Skipped)

		expect(write).toHaveBeenCalledExactlyOnceWith(
			`\n::warning title=Unfunded live test::cancels an order: ${ACCOUNT} holds 1.615829043237491017 POL on Polygon Amoy, needs at least 1.826425200459650342 POL. Fund it to run this test.\n`,
		)
		expect(ctx.skip).toHaveBeenCalledOnce()
	})

	it("returns the step's result and rethrows any other error", async () => {
		const ctx = fakeContext()

		expect(await skipIfUnfunded(ctx, ACCOUNT, AMOY_POL, async () => 7)).toBe(7)
		const reverted = new Error("execution reverted")
		await expect(
			skipIfUnfunded(ctx, ACCOUNT, AMOY_POL, async () => {
				throw reverted
			}),
		).rejects.toBe(reverted)
		expect(ctx.skip).not.toHaveBeenCalled()
	})
})
