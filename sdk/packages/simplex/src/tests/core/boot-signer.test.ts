import { afterAll, beforeAll, describe, expect, it, vi } from "vitest"
import { allChainsWatchOnly, bootFiller, type FillerRuntime } from "@/core/boot"
import type { FillerTomlConfig } from "@/config/filler-toml"
import type { ResolvedChainConfig } from "@/services/FillerConfigService"
import { ChainController } from "@/simplex"
import { LoggerContext } from "@/services/Logger"
import { MemoryDataStore } from "@/data/memory"
import { startMockRpc, type MockRpc } from "../helpers/mock-rpc"
import { stubHyperbridgeScanner, stubOrderScanner } from "../helpers/stub-scanner"

/**
 * The signer requirement used to live in `validateConfig` and was tested there;
 * this branch moved it to boot (a signer is an argument now, not config) and
 * added the per-chain watch-only exemption. These tests pin the relocated rule
 * at the layer that now owns it — and pin that a signerless start is an
 * observer that stays one at runtime.
 */

const CHAIN_ID = 31337
const SUBSTRATE_SEED = `0x${"11".repeat(32)}`

let rpc: MockRpc

beforeAll(async () => {
	rpc = await startMockRpc({ chainId: CHAIN_ID })
})

afterAll(() => {
	rpc.close()
})

function config(overrides: Partial<FillerTomlConfig["simplex"]> = {}): FillerTomlConfig {
	return {
		simplex: {
			substratePrivateKey: SUBSTRATE_SEED,
			hyperbridgeWsUrl: "ws://127.0.0.1:1",
			...overrides,
		},
		pairs: [
			{ token0: "USDC", token1: "USDC", maxOrderSize: "1000", askPriceCurve: [{ amount: "0", price: "0.999" }] },
		],
		chains: [{ rpcUrls: [rpc.url], bundlerUrl: "https://bundler.example" }],
	}
}

function bootOptions() {
	return {
		loggers: new LoggerContext({}),
		scanners: { orders: stubOrderScanner([CHAIN_ID]), hyperbridge: stubHyperbridgeScanner() },
		data: new MemoryDataStore(),
		ownsData: true,
	}
}

// The rejection runs the real path: validateConfig, chain resolution against
// the mock RPC, then the guard. (The signerless *acceptance* paths — global
// watch-only, --watch-only — predate this branch and boot real services against
// real chains; they stay covered by the CLI's observer mode in the field.)
describe("boot signer requirement", () => {
	it("rejects a signerless config with a filling chain", async () => {
		await expect(bootFiller(config(), bootOptions())).rejects.toThrow(/A signer is required/)
	})

	// The per-chain exemption matches on resolved chain ids — a key naming a
	// chain this config does not run must not satisfy it.
	it("rejects a signerless config whose watch-only map misses the resolved chain", async () => {
		await expect(bootFiller(config({ watchOnly: { "EVM-1": true } }), bootOptions())).rejects.toThrow(
			/A signer is required/,
		)
	})
})

describe("allChainsWatchOnly", () => {
	const chains = (...ids: number[]) => ids.map((chainId) => ({ chainId }) as ResolvedChainConfig)

	it("is false with no map at all", () => {
		expect(allChainsWatchOnly(undefined, chains(1))).toBe(false)
	})

	it("is true only when every resolved chain is covered with true", () => {
		expect(allChainsWatchOnly({ 1: true, 8453: true }, chains(1, 8453))).toBe(true)
		expect(allChainsWatchOnly({ 1: true }, chains(1, 8453))).toBe(false)
		expect(allChainsWatchOnly({ 1: true, 8453: false }, chains(1, 8453))).toBe(false)
	})

	// Truthiness is not enough: the map comes from parsed config, where a stray
	// string would otherwise read as watch-only.
	it("requires the literal true, and ignores keys for other chains", () => {
		expect(allChainsWatchOnly({ 1: "yes" as unknown as boolean }, chains(1))).toBe(false)
		expect(allChainsWatchOnly({ 999: true, 1: true }, chains(1))).toBe(true)
	})
})

// The C2 guards, pinned directly on the controller with a hand-built runtime:
// boot admits a signerless start only because every chain is watch-only, and
// these are the two calls that could undo that.
describe("signerless runtime is watch-only for good", () => {
	function signerlessRuntime(): FillerRuntime {
		return {
			signerless: true,
			globalWatchOnly: false,
			config: { simplex: { watchOnly: { [`EVM-${CHAIN_ID}`]: true } }, chains: [] },
			configService: { getConfiguredChainIds: () => [1] },
			intentFiller: { setWatchOnly: vi.fn(), getWatchOnly: () => ({ [CHAIN_ID]: true }) },
			resolvedChains: [],
		} as unknown as FillerRuntime
	}

	it("setWatchOnly(false) throws instead of putting the throwaway key to work", async () => {
		const chains = new ChainController(signerlessRuntime(), async () => {}, stubOrderScanner([CHAIN_ID]), false)
		await expect(chains.setWatchOnly(CHAIN_ID, false)).rejects.toThrow(/started without a signer/)
		// Re-asserting watch-only is fine.
		await expect(chains.setWatchOnly(CHAIN_ID, true)).resolves.toBeUndefined()
	})

	it("add() refuses an explicit watchOnly: false", async () => {
		const chains = new ChainController(signerlessRuntime(), async () => {}, stubOrderScanner([CHAIN_ID]), false)
		await expect(
			chains.add({
				rpcUrls: [rpc.url],
				bundlerUrl: "https://bundler.example",
				watchOnly: false,
				confirmationPolicy: {
					points: [
						{ amount: "1000", value: 1 },
						{ amount: "100000", value: 2 },
					],
				},
			}),
		).rejects.toThrow(/started without a signer/)
	})
})
