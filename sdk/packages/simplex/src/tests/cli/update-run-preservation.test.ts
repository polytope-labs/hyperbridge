import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { assembleConfig } from "@/cli/init/steps/write"
import { emitFillerToml } from "@/cli/init/emit-toml"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"
import { newWizardState, DEFAULT_STABLE_BPS_CURVE } from "@/cli/init/state"
import { INIT_CHAINS } from "@/cli/init/chains"

/**
 * An update run (`simplex init` over an existing config) must preserve every
 * section the wizard doesn't prompt for. Regression for the review finding
 * that binance/targetGasUnits/entryPointAddress/watchOnly/keeper were dropped.
 */
describe("CLI wizard update run", () => {
	const existing: FillerTomlConfig = {
		simplex: {
			signer: { type: SignerType.PrivateKey, key: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d" },
			maxConcurrentOrders: 7,
			queue: { maxRechecks: 4, recheckDelayMs: 12000 },
			logging: "warn",
			watchOnly: { "56": true },
			substratePrivateKey: "seed",
			hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
			entryPointAddress: "0x0000000071727De22E5E9d8BAf0edAc6f37da032",
			solverAccountContractAddress: "0x9999999999999999999999999999999999999999",
			targetGasUnits: 2500000,
			gasFeeBump: { maxPriorityFeePerGasBumpPercent: 12, maxFeePerGasBumpPercent: 15 },
			overfillProtection: { maxOverfillBps: 300, maxConsecutiveClamps: 2 },
		},
		strategies: [
			{
				type: "stable",
				bpsCurve: [
					{ amount: "100", value: 100 },
					{ amount: "100000", value: 10 },
				],
			},
		],
		chains: [{ rpcUrls: ["https://eth.example/rpc"], bundlerUrl: "https://bundler.example" }],
		binance: { apiKey: "bk", apiSecret: "bs", timeout: 9000 },
		keeper: { intervalMinutes: 45, minSwapUsd: 10 },
		allowlist: { users: ["0x1111111111111111111111111111111111111111"] },
	}

	function simulateUpdateRun(): FillerTomlConfig {
		// Mirrors runInit: prefillConfig stored, then each step overwrites its
		// managed fields (here with the same values, as if the user pressed Enter
		// through every prompt), then carryPrefillExtras seeds the finetune state.
		const state = newWizardState()
		state.prefillConfig = JSON.parse(JSON.stringify(existing))
		state.chains = [{ meta: INIT_CHAINS.find((c) => c.chainId === 1)!, rpcUrls: ["https://eth.example/rpc"], bundlerUrl: "https://bundler.example" }]
		state.signer = existing.simplex.signer
		state.substratePrivateKey = existing.simplex.substratePrivateKey
		state.hyperbridgeWsUrl = existing.simplex.hyperbridgeWsUrl
		state.strategies = [{ type: "stable", bpsCurve: DEFAULT_STABLE_BPS_CURVE }]
		// carryPrefillExtras equivalents
		state.maxConcurrentOrders = existing.simplex.maxConcurrentOrders
		state.queue = existing.simplex.queue
		state.logging = existing.simplex.logging
		state.gasFeeBump = existing.simplex.gasFeeBump
		state.overfillProtection = existing.simplex.overfillProtection
		state.allowlist = existing.allowlist
		return assembleConfig(state)
	}

	it("preserves sections the wizard never prompts for", () => {
		const assembled = simulateUpdateRun()

		expect(assembled.binance).toEqual(existing.binance)
		expect(assembled.keeper).toEqual(existing.keeper)
		expect(assembled.simplex.targetGasUnits).toBe(2500000)
		expect(assembled.simplex.entryPointAddress).toBe(existing.simplex.entryPointAddress)
		expect(assembled.simplex.solverAccountContractAddress).toBe(existing.simplex.solverAccountContractAddress)
		expect(assembled.simplex.watchOnly).toEqual({ "56": true })
		expect(assembled.simplex.logging).toBe("warn")
		expect(assembled.simplex.gasFeeBump).toEqual(existing.simplex.gasFeeBump)
	})

	it("survives the emit round-trip with unmanaged sections intact", () => {
		const assembled = simulateUpdateRun()
		const parsed = parse(emitFillerToml(assembled)) as FillerTomlConfig
		expect(() => validateConfig(parsed)).not.toThrow()
		expect(JSON.parse(JSON.stringify(parsed))).toEqual(JSON.parse(JSON.stringify(assembled)))
		expect(parsed.keeper).toEqual(existing.keeper)
		expect(parsed.binance).toEqual(existing.binance)
	})
})
