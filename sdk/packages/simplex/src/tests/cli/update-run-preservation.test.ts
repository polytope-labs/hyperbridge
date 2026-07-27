import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { assembleConfig } from "@/cli/init/steps/write"
import { emitFillerToml } from "@/cli/init/emit-toml"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"
import { newWizardState, DEFAULT_SAME_ASSET_ASK_CURVE } from "@/cli/init/state"
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
		pairs: [
			{
				token0: "USDC",
				token1: "USDC",
				maxOrderSize: "100000",
				askPriceCurve: [
					{ amount: "100", price: "0.99" },
					{ amount: "100000", price: "0.999" },
				],
			},
		],
		chains: [{ rpcUrls: ["https://eth.example/rpc"], bundlerUrl: "https://bundler.example" }],
		binance: { apiKey: "bk", apiSecret: "bs", timeout: 9000 },
		keeper: { intervalMinutes: 45, minSwapUsd: 10 },
		allowlist: { users: ["0x1111111111111111111111111111111111111111"] },
	}

	const wizardPairs = [
		{ token0: "USDC", token1: "USDC", maxOrderSize: "100000", askPriceCurve: DEFAULT_SAME_ASSET_ASK_CURVE },
	]
	const wizardAssets = { BRZ: { "EVM-8453": "0x5555555555555555555555555555555555555555" as const } }
	const wizardConfirmationPolicies = {
		"EVM-1": {
			points: [
				{ amount: "100", value: 2 },
				{ amount: "10000", value: 6 },
			],
		},
	}

	function simulateUpdateRun(): FillerTomlConfig {
		// Mirrors runInit: prefillConfig stored, then each step overwrites its
		// managed fields (here with the same values, as if the user pressed Enter
		// through every prompt), then carryPrefillExtras seeds the finetune state.
		const state = newWizardState()
		// A pre-pairs config may still carry a legacy [[strategies]] array —
		// assembleConfig must strip it, the pair engine rejects it at startup.
		state.prefillConfig = JSON.parse(JSON.stringify({ ...existing, strategies: [{ type: "stable" }] }))
		state.chains = [{ meta: INIT_CHAINS.find((c) => c.chainId === 1)!, rpcUrls: ["https://eth.example/rpc"], bundlerUrl: "https://bundler.example" }]
		state.signer = existing.simplex.signer
		state.substratePrivateKey = existing.simplex.substratePrivateKey
		state.hyperbridgeWsUrl = existing.simplex.hyperbridgeWsUrl
		state.pairs = wizardPairs
		state.assets = wizardAssets
		state.confirmationPolicies = wizardConfirmationPolicies
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

	it("writes the wizard-managed pairs, assets and confirmation policies", () => {
		const assembled = simulateUpdateRun()

		expect(assembled.pairs).toEqual(wizardPairs)
		expect(assembled.assets).toEqual(wizardAssets)
		expect(assembled.confirmationPolicies).toEqual(wizardConfirmationPolicies)
	})

	it("merges wizard assets over the prefilled [assets] entries", () => {
		const state = newWizardState()
		state.prefillConfig = JSON.parse(
			JSON.stringify({
				...existing,
				assets: {
					XYZ: { "EVM-1": "0x7777777777777777777777777777777777777777" },
					BRZ: { "EVM-8453": "0x6666666666666666666666666666666666666666" },
				},
			}),
		)
		state.chains = [
			{
				meta: INIT_CHAINS.find((c) => c.chainId === 1)!,
				rpcUrls: ["https://eth.example/rpc"],
				bundlerUrl: "https://bundler.example",
			},
		]
		state.pairs = wizardPairs
		state.assets = wizardAssets
		const assembled = assembleConfig(state)
		// Prefilled entries survive; the wizard's entry wins on a symbol clash.
		expect(assembled.assets).toEqual({
			XYZ: { "EVM-1": "0x7777777777777777777777777777777777777777" },
			BRZ: wizardAssets.BRZ,
		})
	})

	it("drops a legacy [[strategies]] array from the prefill", () => {
		const assembled = simulateUpdateRun()
		expect("strategies" in assembled).toBe(false)
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
