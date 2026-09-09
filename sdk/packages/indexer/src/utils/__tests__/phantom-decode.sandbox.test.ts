import fs from "node:fs"
import os from "node:os"
import path from "node:path"

// The aggregation runs inside SubQuery's vm2 sandbox, and that sandbox is not Node: `TextDecoder`
// is not a global there, and a `Uint8Array` created inside it reaches host code as a proxy that
// `ArrayBuffer.isView` does not recognise. The bid declaration decoder used to reach `TextDecoder`
// through @polkadot/util for every chain id it read, so a bid that named a source chain threw
// ("Failed to process bid for price snapshot") and was dropped, while bids naming nothing, or only
// positions, decoded fine — which is how two of three solvers vanished from the Base snapshots the
// moment their simplex began declaring sources.
//
// The shipped `intents-helpers` bundle is rolled into one file with esbuild (as `subql build`
// rolls the indexer into one file with webpack — vm2 cannot load the pnpm module graph piecemeal)
// and run inside a NodeVM configured the way @subql/node-core's Sandbox is: commonjs wrapper,
// requires evaluated in the sandbox context. The host's TextEncoder/TextDecoder are injected,
// which is the most charitable environment — viem cannot even load without TextEncoder — and the
// pre-fix decoder still threw there, because the failure is the realm of the bytes, not the
// presence of the API.
const nodeCoreDir = fs.realpathSync(path.dirname(require.resolve("@subql/node-core/package.json")))
const cliDir = fs.realpathSync(path.dirname(require.resolve("@subql/cli/package.json")))
// eslint-disable-next-line @typescript-eslint/no-var-requires
const { NodeVM } = require(require.resolve("vm2", { paths: [nodeCoreDir] }))
// eslint-disable-next-line @typescript-eslint/no-var-requires
const esbuild = require(require.resolve("esbuild", { paths: [cliDir] }))

/** The filler's phantom bid as it was on the wire: a v1 declaration of ["EVM-8453"]. */
const LIVE_V1_DECLARATION = "0x01010845564d2d38343533"

describe("phantom bid declaration inside the SubQuery vm2 sandbox", () => {
	let bundle: string
	let workDir: string

	beforeAll(async () => {
		workDir = fs.mkdtempSync(path.join(os.tmpdir(), "phantom-sandbox-"))
		bundle = path.join(workDir, "intents-helpers.bundle.cjs")
		await esbuild.build({
			entryPoints: [require.resolve("@hyperbridge/sdk/intents-helpers")],
			bundle: true,
			platform: "node",
			format: "cjs",
			logLevel: "silent",
			outfile: bundle,
		})
	}, 120_000)

	afterAll(() => {
		fs.rmSync(workDir, { recursive: true, force: true })
	})

	function runInSandbox<T>(body: string): T {
		const vm = new NodeVM({
			console: "inherit",
			wrapper: "commonjs",
			sandbox: { atob, TextEncoder, TextDecoder },
			require: { builtin: ["*"], external: true, context: "sandbox" },
		})
		return vm.run(
			`const h = require(${JSON.stringify(bundle)}); module.exports = (() => { ${body} })()`,
			path.join(workDir, "sandbox-entry.js"),
		)
	}

	it("decodes a v1 source-chain declaration without throwing", () => {
		const result = runInSandbox<{ sources: string[] | null; positions: string[] }>(`
			const d = h.decodePhantomBidDeclaration(${JSON.stringify(LIVE_V1_DECLARATION)})
			return { sources: d.acceptedSources, positions: d.uniswapV4Positions.map(String) }
		`)
		expect(result).toEqual({ sources: ["EVM-8453"], positions: [] })
	})

	it("decodes the declaration appended to a Permit2-sponsored bid", () => {
		const result = runInSandbox<{ mode: string; sources: string[] | null; paymaster: string }>(`
			const sponsorship = "0x" + "15".repeat(20) + "00".repeat(32) + "02" + "833589fcd6edb6e08f4c7c32d4f71b54bda02913" + "00".repeat(32 * 3) + "1b" + "aa".repeat(32) + "bb".repeat(32)
			const encoded = h.encodePhantomBidPaymasterAndData({ sponsorship, acceptedSourceChains: ["EVM-1", "EVM-8453"] })
			const d = h.decodePhantomBidPaymasterAndData(encoded)
			return { mode: d.mode, sources: d.declaration.acceptedSources, paymaster: d.sponsorship.paymaster }
		`)
		expect(result).toEqual({ mode: "permit2", sources: ["EVM-1", "EVM-8453"], paymaster: `0x${"15".repeat(20)}` })
	})

	it("round-trips a declaration encoded inside the sandbox", () => {
		const result = runInSandbox<string[] | null>(`
			return h.decodeAcceptedSourceChains(h.encodeAcceptedSourceChains(["EVM-1", "EVM-56", "EVM-137", "EVM-8453", "EVM-42161"]))
		`)
		expect(result).toEqual(["EVM-1", "EVM-56", "EVM-137", "EVM-8453", "EVM-42161"])
	})
})
