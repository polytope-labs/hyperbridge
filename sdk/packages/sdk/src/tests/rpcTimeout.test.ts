import { describe, expect, it } from "vitest"
import { RpcTimeoutError, withRpcTimeout } from "@/protocols/intents/RpcError"

describe("intent RPC deadlines", () => {
	it("aborts a request that does not settle before its deadline", async () => {
		let signal: AbortSignal | undefined
		const pending = withRpcTimeout(
			"test RPC",
			(requestSignal) => {
				signal = requestSignal
				return new Promise<never>(() => undefined)
			},
			5,
		)

		await expect(pending).rejects.toBeInstanceOf(RpcTimeoutError)
		expect(signal?.aborted).toBe(true)
	})
})
