import { describe, it, expect } from "vitest"
import { parse } from "toml"
import { emitFillerToml } from "@/cli/init/emit-toml"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"

const minimalStable: FillerTomlConfig = {
	simplex: {
		signer: {
			type: SignerType.PrivateKey,
			key: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
		},
		maxConcurrentOrders: 5,
		queue: { maxRechecks: 10, recheckDelayMs: 30000 },
		substratePrivateKey: "bottom drive obey lake curtain smoke basket hold race lonely fit walk",
		hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
	},
	strategies: [
		{
			type: "stable",
			bpsCurve: [
				{ amount: "100", value: 100 },
				{ amount: "1000", value: 50 },
				{ amount: "100000", value: 10 },
			],
		},
	],
	chains: [
		{
			rpcUrls: ["https://eth-mainnet.g.alchemy.com/v2/someKey"],
			bundlerUrl: "https://eth-mainnet.g.alchemy.com/v2/someKey",
		},
		{
			rpcUrls: ["https://base-mainnet.g.alchemy.com/v2/someKey"],
			bundlerUrl: "https://api.pimlico.io/v2/8453/rpc?apikey=pim_key",
		},
	],
}

const hyperfxWithCurves: FillerTomlConfig = {
	simplex: {
		signer: {
			type: SignerType.Turnkey,
			organizationId: "org-1",
			apiPublicKey: "pub",
			apiPrivateKey: "priv",
			signWith: "0x2222222222222222222222222222222222222222",
		},
		maxConcurrentOrders: 3,
		queue: { maxRechecks: 5, recheckDelayMs: 15000 },
		substratePrivateKey: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
		hyperbridgeWsUrl: "wss://gargantua.rpc.polytope.technology",
		logging: "info",
	},
	strategies: [
		{
			type: "hyperfx",
			maxOrderUsd: 5000,
			spreadBps: 50,
			bidPriceCurve: [
				{ amount: "100", price: "1580" },
				{ amount: "5000", price: "1570" },
			],
			askPriceCurve: [
				{ amount: "100", price: "1560" },
				{ amount: "5000", price: "1550" },
			],
			token1: {
				"EVM-56": "0x1111111111111111111111111111111111111111",
				"EVM-137": "0x3333333333333333333333333333333333333333",
			},
			confirmationPolicies: {
				"56": {
					points: [
						{ amount: "1", value: 3 },
						{ amount: "5000", value: 15 },
					],
				},
			},
		},
	],
	chains: [
		{ rpcUrls: ["https://bsc.example/rpc"], bundlerUrl: "https://api.pimlico.io/v2/56/rpc?apikey=k" },
		{ rpcUrls: ["https://polygon.example/rpc"], bundlerUrl: "https://api.pimlico.io/v2/137/rpc?apikey=k" },
	],
}

const kitchenSink: FillerTomlConfig = {
	simplex: {
		signer: {
			type: SignerType.MpcVault,
			apiToken: "token",
			vaultUuid: "uuid-1",
			accountAddress: "0x4444444444444444444444444444444444444444",
			callbackClientSignerPublicKey: "ssh-ed25519 AAAA",
			grpcTarget: "api.mpcvault.com:443",
		},
		maxConcurrentOrders: 8,
		queue: { maxRechecks: 10, recheckDelayMs: 30000 },
		substratePrivateKey: "seed",
		hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
		logging: "debug",
		watchOnly: false,
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
			confirmationPolicies: {
				"1": {
					points: [
						{ amount: "5", value: 3 },
						{ amount: "5000", value: 12 },
					],
				},
			},
		},
		{
			type: "hyperfx",
			maxOrderUsd: 10000,
			token1: { "EVM-8453": "0x5555555555555555555555555555555555555555" },
			vault: {
				uniswapV4: {
					side: "ask",
					positions: [
						{ chain: "EVM-8453", tokenId: "123456789", referencePrice: "1575", maxDeviationBps: 200 },
					],
				},
			},
		},
	],
	chains: [
		{
			rpcUrls: ["https://eth.example/rpc", "https://eth-two.example/rpc"],
			bundlerUrl: "https://api.pimlico.io/v2/1/rpc?apikey=k",
		},
	],
	rebalancing: {
		triggerPercentage: 0.5,
		baseBalances: {
			USDC: { "1": "10000", "8453": "10000" },
			USDT: { "1": "5000" },
		},
	},
	vault: {
		sweepIntervalMs: 300000,
		vaults: [
			{
				chain: "EVM-8453",
				vault: "0xC768c589647798a6EE01A91FdE98EF2ed046DBD6",
				threshold: "5000",
				minBalance: "3000",
				redeemOnShutdown: true,
			},
		],
	},
	allowlist: {
		users: ["0x1111111111111111111111111111111111111111"],
		bySource: { "EVM-1": ["0x2222222222222222222222222222222222222222"] },
	},
	binance: { apiKey: "bk", apiSecret: "bs", timeout: 5000 },
	keeper: { chains: ["EVM-8453"], intervalMinutes: 30, minSwapUsd: 25 },
}

describe("emitFillerToml", () => {
	const fixtures: Array<[string, FillerTomlConfig]> = [
		["minimal stable", minimalStable],
		["hyperfx with curves", hyperfxWithCurves],
		["kitchen sink", kitchenSink],
	]

	for (const [name, fixture] of fixtures) {
		it(`round-trips the ${name} config through the run parser`, () => {
			const emitted = emitFillerToml(fixture)
			const parsed = parse(emitted) as FillerTomlConfig
			expect(parsed).toEqual(fixture)
			expect(() => validateConfig(parsed)).not.toThrow()
		})
	}

	it("preserves special characters in URLs, mnemonics and quoted keys", () => {
		const emitted = emitFillerToml(kitchenSink)
		const parsed = parse(emitted) as FillerTomlConfig
		expect(parsed.chains[0].bundlerUrl).toBe("https://api.pimlico.io/v2/1/rpc?apikey=k")
		expect(parsed.rebalancing?.baseBalances.USDC?.["8453"]).toBe("10000")
		const fx = parsed.strategies[1]
		if (fx.type !== "hyperfx") throw new Error("expected hyperfx strategy")
		expect(fx.token1["EVM-8453"]).toBe("0x5555555555555555555555555555555555555555")

		const mnemonic = emitFillerToml(minimalStable)
		expect((parse(mnemonic) as FillerTomlConfig).simplex.substratePrivateKey).toBe(
			"bottom drive obey lake curtain smoke basket hold race lonely fit walk",
		)
	})

	it("renders chain comments above each [[chains]] entry", () => {
		const emitted = emitFillerToml(minimalStable, { chainComments: ["Ethereum (1)", "Base (8453)"] })
		expect(emitted).toContain("# Ethereum (1)\n[[chains]]")
		expect(emitted).toContain("# Base (8453)\n[[chains]]")
	})

	it("shows commented defaults for omitted optional sections", () => {
		const emitted = emitFillerToml(minimalStable)
		expect(emitted).toContain("# maxPriorityFeePerGasBumpPercent = 8")
		expect(emitted).toContain("# maxOverfillBps = 500")
	})
})
