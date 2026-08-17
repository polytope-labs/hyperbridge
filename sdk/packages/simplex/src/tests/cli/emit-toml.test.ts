import { describe, it, expect } from "vitest"
import { mkdtempSync, readdirSync, readFileSync, statSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { parse } from "toml"
import { emitFillerToml, writeConfigFileAtomic } from "@/cli/init/emit-toml"
import { validateConfig, type FillerTomlConfig } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"

const minimalSameAsset: FillerTomlConfig = {
	simplex: {
		signer: {
			type: SignerType.PrivateKey,
			key: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
		},
		maxConcurrentOrders: 5,
		substratePrivateKey: "bottom drive obey lake curtain smoke basket hold race lonely fit walk",
		hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
	},
	pairs: [
		{
			token0: "USDC",
			token1: "USDC",
			maxOrderSize: "100000",
			askPriceCurve: [
				{ amount: "100", price: "0.99" },
				{ amount: "1000", price: "0.995" },
				{ amount: "100000", price: "0.999" },
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

const crossAssetWithCurves: FillerTomlConfig = {
	simplex: {
		signer: {
			type: SignerType.Turnkey,
			organizationId: "org-1",
			apiPublicKey: "pub",
			apiPrivateKey: "priv",
			signWith: "0x2222222222222222222222222222222222222222",
		},
		maxConcurrentOrders: 3,
		substratePrivateKey: "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789",
		hyperbridgeWsUrl: "wss://gargantua.rpc.polytope.technology",
		logging: "info",
	},
	pairs: [
		{
			token0: "USDC",
			token1: "CNGN",
			maxOrderSize: "5000",
			bidPriceCurve: [
				{ amount: "100", price: "1580" },
				{ amount: "5000", price: "1570" },
			],
			askPriceCurve: [
				{ amount: "100", price: "1560" },
				{ amount: "5000", price: "1550" },
			],
		},
		// Anchors ZARP through CNGN without opening a ZARP/CNGN market.
		{
			token0: "ZARP",
			token1: "CNGN",
			referenceOnly: true,
			askPriceCurve: [{ amount: "0", price: "85" }],
		},
	],
	confirmationPolicies: {
		"EVM-56": {
			points: [
				{ amount: "1", value: 3 },
				{ amount: "5000", value: 15 },
			],
		},
	},
	chains: [
		{ rpcUrls: ["https://bsc.example/rpc"], bundlerUrl: "https://api.pimlico.io/v2/56/rpc?apikey=k" },
		{ rpcUrls: ["https://polygon.example/rpc"], bundlerUrl: "https://api.pimlico.io/v2/137/rpc?apikey=k" },
	],
}

// `side` requires pool pricing with no static curves, so this pair is curve-less
// and priced by the Uniswap V4 venue.
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
		substratePrivateKey: "seed",
		hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
		logging: "debug",
		watchOnly: false,
		targetGasUnits: 2500000,
		gasFeeBump: { maxPriorityFeePerGasBumpPercent: 12, maxFeePerGasBumpPercent: 15 },
		overfillProtection: { maxOverfillBps: 300, maxConsecutiveClamps: 2 },
	},
	assets: {
		BRZ: { "EVM-8453": "0x5555555555555555555555555555555555555555" },
	},
	pairs: [
		{
			token0: "USDC",
			token1: "CNGN",
			maxOrderSize: "10000",
		},
	],
	confirmationPolicies: {
		"EVM-1": {
			points: [
				{ amount: "5", value: 3 },
				{ amount: "5000", value: 12 },
			],
		},
	},
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
		uniswapV4: {
			side: "ask",
			spreadBps: 75,
			positions: [
				{ chain: "EVM-8453", tokenId: "123456789", referencePrice: "1575", maxDeviationBps: 200 },
			],
		},
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
		["minimal same-asset", minimalSameAsset],
		["cross-asset with curves", crossAssetWithCurves],
		["kitchen sink", kitchenSink],
	]

	for (const [name, fixture] of fixtures) {
		it(`round-trips the ${name} config through the run parser`, () => {
			const emitted = emitFillerToml(fixture)
			const parsed = parse(emitted) as FillerTomlConfig
			expect(JSON.parse(JSON.stringify(parsed))).toEqual(JSON.parse(JSON.stringify(fixture)))
			expect(() => validateConfig(parsed)).not.toThrow()
		})
	}

	it("round-trips a signer-less watch-only config without a [simplex.signer] header", () => {
		const signerless: FillerTomlConfig = JSON.parse(JSON.stringify(minimalSameAsset))
		delete signerless.simplex.signer
		signerless.simplex.watchOnly = true

		const emitted = emitFillerToml(signerless)
		expect(emitted).not.toContain("[simplex.signer]")
		const parsed = parse(emitted) as FillerTomlConfig
		expect(JSON.parse(JSON.stringify(parsed))).toEqual(JSON.parse(JSON.stringify(signerless)))
		expect(() => validateConfig(parsed)).not.toThrow()
	})

	it("renders chain comments above each [[chains]] entry", () => {
		const emitted = emitFillerToml(minimalSameAsset, { chainComments: ["Ethereum (1)", "Base (8453)"] })
		expect(emitted).toContain("# Ethereum (1)\n[[chains]]")
		expect(emitted).toContain("# Base (8453)\n[[chains]]")
	})
})

describe("writeConfigFileAtomic", () => {
	it("writes content with mode 600 and leaves no temp file behind", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-write-"))
		const path = join(dir, "filler-config.toml")

		writeConfigFileAtomic(path, "a = 1\n")
		expect(readFileSync(path, "utf-8")).toBe("a = 1\n")
		expect(statSync(path).mode & 0o777).toBe(0o600)
		expect(readdirSync(dir)).toEqual(["filler-config.toml"])

		// overwrites atomically, keeping the mode
		writeConfigFileAtomic(path, "a = 2\n")
		expect(readFileSync(path, "utf-8")).toBe("a = 2\n")
		expect(statSync(path).mode & 0o777).toBe(0o600)
		expect(readdirSync(dir)).toEqual(["filler-config.toml"])
	})

	it("cleans up the temp file when the write fails", () => {
		const dir = mkdtempSync(join(tmpdir(), "simplex-write-"))
		const path = join(dir, "missing-subdir", "filler-config.toml")
		expect(() => writeConfigFileAtomic(path, "x")).toThrow()
		expect(readdirSync(dir)).toEqual([])
	})
})
