import { describe, it, expect } from "vitest"
import { FillerConfigService } from "@/services/FillerConfigService"

/**
 * `bidValiditySeconds` bounds how long a signed bid stays executable. It is the only thing
 * bounding it: the order's own deadline is placer-chosen with no ceiling, and retracting a bid
 * on Hyperbridge does not reach the destination chain. So the default matters as much as the
 * override — a filler that never sets it must still get a bounded quote.
 */

const CHAINS = [{ chainId: 8453, rpcUrls: ["https://rpc.example"] }] as any

function service(fillerConfig?: Record<string, unknown>) {
	return new FillerConfigService(CHAINS, fillerConfig as any)
}

describe("bidValiditySeconds", () => {
	it("defaults to 5 minutes when the operator does not set it", () => {
		expect(service().getBidValiditySeconds()).toBe(300)
	})

	it("takes the configured value when set", () => {
		expect(service({ bidValiditySeconds: 900 }).getBidValiditySeconds()).toBe(900)
	})

	it("is honoured at zero rather than falling back to the default", () => {
		// `?? 300` and `|| 300` differ here, and only the first is right: 0 is a deliberate
		// "no bound", not an absent value.
		expect(service({ bidValiditySeconds: 0 }).getBidValiditySeconds()).toBe(0)
	})
})
