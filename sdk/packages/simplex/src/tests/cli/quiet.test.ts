import { describe, expect, it } from "vitest"
// Importing this module patches console.warn and sets the polkadot env flag in
// THIS worker — which is the module's whole job, and vitest's per-file worker
// isolation keeps it from leaking into other suites.
import { filteringWarn, isPolkadotNoise } from "@/bin/quiet"

describe("bin console hygiene", () => {
	it("drops the duplicate-package detector, both variants", () => {
		// One console.warn call: message + advice + version table, one string.
		expect(
			isPolkadotNoise([
				"@polkadot/util has multiple versions, ensure that there is only one installed.\n" +
					"Either remove and explicitly install matching versions or dedupe using your package manager.\n" +
					"The following conflicting packages were found:\n\tesm 14.0.3\t/app\n\tesm 13.5.9\t/app",
			]),
		).toBe(true)
		expect(
			isPolkadotNoise(["@polkadot/api requires direct dependencies exactly matching version 16.5.6."]),
		).toBe(true)
	})

	it("drops the polkadot logger's known chatter, which arrives as separate args", () => {
		expect(
			isPolkadotNoise(["2026-08-19 07:37:15", "REGISTRY:", "Unknown signed extensions PrioritizeVeto found, treating them as no-effect"]),
		).toBe(true)
		expect(
			isPolkadotNoise(["2026-08-19 07:37:15", "API/INIT:", "RPC methods not decorated: archive_v1_body, chainHead_v1_follow"]),
		).toBe(true)
	})

	it("passes everything else through, including other polkadot output", () => {
		expect(isPolkadotNoise(["ordinary warning"])).toBe(false)
		// A real connection failure from the same logger must never be dropped.
		expect(isPolkadotNoise(["2026-08-19 07:37:15", "API/INIT:", "Unable to initialize the API: connection refused"])).toBe(false)
		expect(isPolkadotNoise(["RPC-CORE:", "getBlockHash: disconnected from wss://..."])).toBe(false)
		// Non-string args never match by accident.
		expect(isPolkadotNoise([{ message: "REGISTRY: Unknown signed extensions" }])).toBe(false)
	})

	it("filteringWarn drops noise and forwards the rest, argument list intact", () => {
		const seen: unknown[][] = []
		const warn = filteringWarn((...args) => seen.push(args))

		warn("@polkadot/util has multiple versions, ensure that there is only one installed.")
		warn("2026-08-19 07:37:15", "REGISTRY:", "Unknown signed extensions PrioritizeVeto found, treating them as no-effect")
		warn("a warning an operator should see", { detail: 42 })

		expect(seen).toEqual([["a warning an operator should see", { detail: 42 }]])

		// The module's other two effects, applied at import time.
		expect(process.env.POLKADOTJS_DISABLE_ESM_CJS_WARNING).toBe("1")
		expect((process as { noDeprecation?: boolean }).noDeprecation).toBe(true)
	})
})
