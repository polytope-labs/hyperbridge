import { describe, it, expect } from "vitest"
import { bytes20ToBytes32 } from "@hyperbridge/sdk"
import { FillerConfigService, type AllowlistConfig } from "@/services/FillerConfigService"

const USER_A = "0x1111111111111111111111111111111111111111"
const USER_B = "0x2222222222222222222222222222222222222222"
const USER_C = "0x3333333333333333333333333333333333333333"

// Orders surface `user` as the bytes32 (left-padded) form; isUserAllowed is called with that.
const asOrderUser = (addr: string) => bytes20ToBytes32(addr)

function service(allowlist?: AllowlistConfig): FillerConfigService {
	return new FillerConfigService([], { maxConcurrentOrders: 5, allowlist })
}

describe("FillerConfigService allowlist", () => {
	it("accepts every user when no allowlist is configured", () => {
		const svc = service()
		expect(svc.isUserAllowed(asOrderUser(USER_A), "EVM-1")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-42161")).toBe(true)
	})

	it("matches a bytes32 order user against a 20-byte config address", () => {
		const svc = service({ users: [USER_A] })
		// The runtime bytes32 form must match the 20-byte configured address.
		expect(svc.isUserAllowed(asOrderUser(USER_A), "EVM-1")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-1")).toBe(false)
	})

	it("matches global users case-insensitively", () => {
		const svc = service({ users: [USER_A.toUpperCase()] })
		expect(svc.isUserAllowed(asOrderUser(USER_A).toLowerCase(), "EVM-1")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_A).toUpperCase(), "EVM-1")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-1")).toBe(false)
	})

	it("merges per-source overrides with the global list", () => {
		const svc = service({ users: [USER_A], bySource: { "EVM-1": [USER_B] } })
		// Global user is allowed everywhere.
		expect(svc.isUserAllowed(asOrderUser(USER_A), "EVM-1")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_A), "EVM-42161")).toBe(true)
		// Per-source user is allowed only on its chain.
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-1")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-42161")).toBe(false)
	})

	it("isolates per-source overrides between chains", () => {
		const svc = service({ bySource: { "EVM-1": [USER_B], "EVM-42161": [USER_C] } })
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-1")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-42161")).toBe(false)
		expect(svc.isUserAllowed(asOrderUser(USER_C), "EVM-42161")).toBe(true)
		expect(svc.isUserAllowed(asOrderUser(USER_C), "EVM-1")).toBe(false)
	})

	it("enforces per-source overrides when no global users list is configured", () => {
		const svc = service({ bySource: { "EVM-1": [USER_B] } })
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-1")).toBe(true)
		// No global list and no override for this chain rejects all.
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-42161")).toBe(false)
		expect(svc.isUserAllowed(asOrderUser(USER_A), "EVM-1")).toBe(false)
	})

	it("rejects every user when the allowlist is present but empty for a chain", () => {
		const svc = service({ users: [], bySource: { "EVM-1": [USER_B] } })
		// Chain with no override and an empty global list rejects all.
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-42161")).toBe(false)
		expect(svc.isUserAllowed(asOrderUser(USER_A), "EVM-42161")).toBe(false)
		// Chain with an override still admits its listed user.
		expect(svc.isUserAllowed(asOrderUser(USER_B), "EVM-1")).toBe(true)
	})

	it("exposes the raw allowlist via getAllowlist", () => {
		const allowlist: AllowlistConfig = { users: [USER_A] }
		expect(service(allowlist).getAllowlist()).toBe(allowlist)
		expect(service().getAllowlist()).toBeUndefined()
	})
})
