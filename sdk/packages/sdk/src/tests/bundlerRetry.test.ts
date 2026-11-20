import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import type { IntentGatewayContext } from "@/protocols/intents/types"
import type { BidPreVerificationGasParams, HexString } from "@/types"

// Retries wait through `sleep`; here it returns at once and records the waits.
vi.mock("@/utils", async (importOriginal) => ({
	...(await importOriginal<typeof import("@/utils")>()),
	sleep: vi.fn(async () => {}),
}))

const { sleep } = await import("@/utils")
const { CryptoUtils } = await import("@/protocols/intents/CryptoUtils")
const { GasEstimator } = await import("@/protocols/intents/GasEstimator")
const { BundlerRpcError, isTransientBundlerError, retryTransientBundlerErrors } = await import(
	"@/protocols/intents/bundlerRetry"
)

const BUNDLER = "https://bundler.example"

/** Stubs `fetch` with one bundler response: an HTTP status and a body, JSON unless a string. */
function bundlerResponds(status: number, body: unknown, statusText = "") {
	vi.stubGlobal(
		"fetch",
		vi.fn(async () => ({
			status,
			statusText,
			json: async () => (typeof body === "string" ? JSON.parse(body) : body),
		})),
	)
}

/** What `sendBundler` throws for the stubbed response. */
async function bundlerError(): Promise<InstanceType<typeof BundlerRpcError>> {
	const crypto = new CryptoUtils({ bundlerUrl: BUNDLER } as IntentGatewayContext)
	const error = await crypto.sendBundler("eth_estimateUserOperationGas", []).then(
		() => null,
		(e: unknown) => e,
	)
	expect(error).toBeInstanceOf(BundlerRpcError)
	return error as InstanceType<typeof BundlerRpcError>
}

const rpcError = (code: number, message: string) => ({ jsonrpc: "2.0", id: 1, error: { code, message } })

beforeEach(() => vi.mocked(sleep).mockClear())
afterEach(() => vi.unstubAllGlobals())

describe("CryptoUtils.sendBundler failures", () => {
	it("reads rundler's upstream failure as transient, keeping its message", async () => {
		bundlerResponds(200, rpcError(-32603, "internal error: rpc provider error"))

		const error = await bundlerError()

		expect(error.message).toBe("Bundler error: internal error: rpc provider error")
		expect(error.code).toBe(-32603)
		expect(isTransientBundlerError(error)).toBe(true)
	})

	it("reads a failed connection as transient", async () => {
		const reset = Object.assign(new Error("read ECONNRESET"), { code: "ECONNRESET" })
		vi.stubGlobal("fetch", vi.fn().mockRejectedValue(new TypeError("fetch failed", { cause: reset })))

		const error = await bundlerError()

		expect(error.message).toBe("Bundler error: fetch failed")
		expect(error.transport).toBe(true)
		expect(isTransientBundlerError(error)).toBe(true)
	})

	it("reads a proxy's 502 page, a 408 and a 429 as transient", async () => {
		bundlerResponds(502, "<html>Bad Gateway</html>", "Bad Gateway")
		const badGateway = await bundlerError()
		expect(badGateway.message).toBe("Bundler error: HTTP 502 Bad Gateway")
		expect(isTransientBundlerError(badGateway)).toBe(true)

		bundlerResponds(429, {})
		const rateLimited = await bundlerError()
		expect(rateLimited.message).toBe("Bundler error: HTTP 429")
		expect(isTransientBundlerError(rateLimited)).toBe(true)

		bundlerResponds(408, {})
		const requestTimeout = await bundlerError()
		expect(requestTimeout.message).toBe("Bundler error: HTTP 408")
		expect(isTransientBundlerError(requestTimeout)).toBe(true)
	})

	it("reads a request aborted by the timeout as transient", async () => {
		vi.stubGlobal(
			"fetch",
			vi.fn().mockRejectedValue(new DOMException("The operation was aborted due to timeout", "TimeoutError")),
		)

		const error = await bundlerError()

		expect(error.transport).toBe(true)
		expect(isTransientBundlerError(error)).toBe(true)
	})

	it.each([
		[-32500, "validation reverted: AA23 reverted"],
		[-32501, "paymaster validation rejected"],
		[-32502, "opcode violation"],
		[-32521, "execution reverted"],
		[-32602, "Unsupported entry point"],
		[-32603, "internal error: AA33 reverted"],
	])("reads JSON-RPC error %i (%s) as the bundler's answer", async (code, message) => {
		bundlerResponds(200, rpcError(code, message))

		expect(isTransientBundlerError(await bundlerError())).toBe(false)
	})

	it("reads a non-deterministic JSON-RPC error on an HTTP 500 as transient", async () => {
		bundlerResponds(500, rpcError(-32000, "server error"))

		const error = await bundlerError()

		expect(error.code).toBe(-32000)
		expect(isTransientBundlerError(error)).toBe(true)
	})

	it("reads an ERC-4337 rejection as the bundler's answer even on an HTTP 500", async () => {
		bundlerResponds(500, rpcError(-32500, "validation reverted: AA21 didn't pay prefund"))

		expect(isTransientBundlerError(await bundlerError())).toBe(false)
	})

	it("reads an HTTP 404 and a body that is not JSON as permanent", async () => {
		bundlerResponds(404, "Not Found", "Not Found")
		expect(isTransientBundlerError(await bundlerError())).toBe(false)

		bundlerResponds(200, "<html></html>")
		expect(isTransientBundlerError(await bundlerError())).toBe(false)
	})

	it("reads an error from anywhere else as permanent", () => {
		expect(isTransientBundlerError(new Error("Bundler error: internal error: rpc provider error"))).toBe(false)
	})
})

describe("retryTransientBundlerErrors", () => {
	const transient = () => new BundlerRpcError("Bundler error: internal error: rpc provider error", { code: -32603 })

	it("returns the result of an attempt that follows a transient failure", async () => {
		const call = vi.fn().mockRejectedValueOnce(transient()).mockResolvedValueOnce("estimate")
		const onRetry = vi.fn()

		await expect(retryTransientBundlerErrors(call, onRetry)).resolves.toBe("estimate")

		expect(call).toHaveBeenCalledTimes(2)
		expect(onRetry).toHaveBeenCalledOnce()
		expect(onRetry.mock.calls[0][0]).toMatchObject({ attempt: 1, maxAttempts: 3, delayMs: 250 })
		expect(vi.mocked(sleep).mock.calls).toEqual([[250]])
	})

	it("throws the last error after three transient failures", async () => {
		const errors = [transient(), transient(), transient()]
		const call = vi
			.fn()
			.mockRejectedValueOnce(errors[0])
			.mockRejectedValueOnce(errors[1])
			.mockRejectedValueOnce(errors[2])
		const onRetry = vi.fn()

		await expect(retryTransientBundlerErrors(call, onRetry)).rejects.toBe(errors[2])

		expect(call).toHaveBeenCalledTimes(3)
		expect(onRetry.mock.calls.map(([event]) => [event.attempt, event.error])).toEqual([
			[1, errors[0]],
			[2, errors[1]],
		])
		expect(vi.mocked(sleep).mock.calls).toEqual([[250], [750]])
	})

	it("throws a validation revert at once", async () => {
		const revert = new BundlerRpcError("Bundler error: validation reverted: AA23 reverted", { code: -32500 })
		const call = vi.fn().mockRejectedValue(revert)
		const onRetry = vi.fn()

		await expect(retryTransientBundlerErrors(call, onRetry)).rejects.toBe(revert)

		expect(call).toHaveBeenCalledOnce()
		expect(onRetry).not.toHaveBeenCalled()
		expect(sleep).not.toHaveBeenCalled()
	})

	it("warns on the console with the attempt and the error when given no callback", async () => {
		const warn = vi.spyOn(console, "warn").mockImplementation(() => {})
		const call = vi.fn().mockRejectedValueOnce(transient()).mockResolvedValueOnce("estimate")

		await retryTransientBundlerErrors(call)

		expect(warn).toHaveBeenCalledWith(
			"Bundler gas estimate attempt 1/3 failed, retrying in 250ms: Bundler error: internal error: rpc provider error",
		)
		warn.mockRestore()
	})
})

describe("GasEstimator.estimateBidPreVerificationGas on a transient bundler failure", () => {
	const BID: BidPreVerificationGasParams = {
		solverAccount: "0x21426d68a9e5df153fe75ce0fed20173ebcb80ef" as HexString,
		nonce: 0n,
		entryPointAddress: "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString,
		callGasLimit: 1_000_000n,
		verificationGasLimit: 100_000n,
		maxFeePerGas: 2n,
		maxPriorityFeePerGas: 1n,
		callData: "0x" as HexString,
	}

	it("asks again", async () => {
		const sendBundler = vi
			.fn()
			.mockRejectedValueOnce(
				new BundlerRpcError("Bundler error: internal error: rpc provider error", { code: -32603 }),
			)
			.mockResolvedValueOnce({ preVerificationGas: "0x186a0", verificationGasLimit: "0x1", callGasLimit: "0x1" })
		const ctx = {
			dest: { client: { getBlock: vi.fn().mockResolvedValue({ baseFeePerGas: 1n }) } },
		} as unknown as IntentGatewayContext
		const estimator = new GasEstimator(ctx, { sendBundler } as unknown as InstanceType<typeof CryptoUtils>)
		const onBundlerRetry = vi.fn()

		const preVerificationGas = await estimator.estimateBidPreVerificationGas(BID, onBundlerRetry)

		expect(preVerificationGas).toBeGreaterThan(0n)
		expect(sendBundler).toHaveBeenCalledTimes(2)
		expect(onBundlerRetry).toHaveBeenCalledOnce()
	})
})
