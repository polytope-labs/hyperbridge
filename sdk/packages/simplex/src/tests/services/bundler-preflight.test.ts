import { afterEach, describe, expect, it, vi } from "vitest"
import { FillerConfigService } from "@/services/FillerConfigService"
import { LoggerContext } from "@/services/Logger"
import { assertBundlersServeEntryPoint, bundlerHost } from "@/services/bundler-preflight"

const logLines: string[] = []
const configService = new FillerConfigService(
	[],
	undefined,
	new LoggerContext({ sink: { write: (line: string) => logLines.push(line) } }),
)
const ENTRY_POINT = configService.getEntryPointAddress("EVM-8453")!
const V07 = "0x0000000071727De22E5E9d8BAf0edAc6f37da032"
const BUNDLER = "https://bundler.example/rpc"
const SECRET_BUNDLER = "https://api.pimlico.io/v2/8453/rpc?apikey=supersecretpimlicokey"

type Answer = { result: unknown } | { error: string } | { status: number } | { body: string } | Error | "hang"

/** Answers eth_supportedEntryPoints per bundler URL, and records every call. */
function stubBundlers(answers: Record<string, Answer>) {
	const calls: Array<{ url: string; method: string }> = []
	vi.stubGlobal("fetch", async (url: string | URL, init?: { body?: string; signal?: AbortSignal }) => {
		const { method } = JSON.parse(init?.body ?? "{}") as { method: string }
		calls.push({ url: String(url), method })
		const answer = answers[String(url)]
		if (answer === undefined) throw new Error(`unexpected fetch to ${url}`)
		if (answer === "hang") {
			return new Promise<Response>((_, reject) => {
				init?.signal?.addEventListener("abort", () => reject(init.signal!.reason))
			})
		}
		if (answer instanceof Error) throw answer
		if ("status" in answer) return new Response("nope", { status: answer.status })
		if ("body" in answer) return new Response(answer.body, { status: 200 })
		const body = "error" in answer ? { error: { message: answer.error } } : { result: answer.result }
		return new Response(JSON.stringify({ jsonrpc: "2.0", id: 1, ...body }), { status: 200 })
	})
	return calls
}

function warnings(): Array<{ msg: string; bundler: string; chain: string }> {
	return logLines.map((line) => JSON.parse(line)).filter((record) => record.level === 40)
}

afterEach(() => {
	vi.unstubAllGlobals()
	logLines.length = 0
})

describe("assertBundlersServeEntryPoint", () => {
	it("passes when the bundler lists the chain's EntryPoint, asking once per chain", async () => {
		const calls = stubBundlers({ [BUNDLER]: { result: [V07, ENTRY_POINT] } })

		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
		).resolves.toBeUndefined()
		expect(calls).toEqual([{ url: BUNDLER, method: "eth_supportedEntryPoints" }])
		expect(warnings()).toEqual([])
	})

	it("matches the EntryPoint in any letter case", async () => {
		stubBundlers({ [BUNDLER]: { result: [ENTRY_POINT.toLowerCase()] } })
		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
		).resolves.toBeUndefined()

		stubBundlers({ [BUNDLER]: { result: [`0x${ENTRY_POINT.slice(2).toUpperCase()}`] } })
		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
		).resolves.toBeUndefined()
	})

	it("refuses a bundler that does not list the EntryPoint, naming the chain, host, requirement and listing", async () => {
		stubBundlers({ [BUNDLER]: { result: [V07] } })

		const error = await assertBundlersServeEntryPoint(
			[{ chainId: 8453, bundlerUrl: BUNDLER }],
			configService,
		).catch((err: Error) => err)
		expect(error).toBeInstanceOf(Error)
		const message = (error as Error).message
		expect(message).toContain("Base (EVM-8453)")
		expect(message).toContain("bundler.example")
		expect(message).toContain(ENTRY_POINT)
		expect(message).toContain(`it lists ${V07}`)
	})

	it("refuses a bundler that lists nothing", async () => {
		stubBundlers({ [BUNDLER]: { result: [] } })
		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
		).rejects.toThrow(/it lists none/)
	})

	it("warns and passes on an RPC error", async () => {
		stubBundlers({ [BUNDLER]: { error: "method eth_supportedEntryPoints not found" } })
		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
		).resolves.toBeUndefined()
		const [warning] = warnings()
		expect(warning.msg).toMatch(
			/Bundler bundler\.example for Base \(EVM-8453\) did not answer eth_supportedEntryPoints: method eth_supportedEntryPoints not found/,
		)
		expect(warning.bundler).toBe("bundler.example")
		expect(warning.chain).toBe("EVM-8453")
	})

	it("warns and passes on an HTTP error and an unreachable bundler", async () => {
		stubBundlers({ [BUNDLER]: { status: 401 } })
		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
		).resolves.toBeUndefined()

		stubBundlers({
			[BUNDLER]: Object.assign(new TypeError("fetch failed"), {
				cause: new Error("getaddrinfo ENOTFOUND bundler.example"),
			}),
		})
		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
		).resolves.toBeUndefined()

		const messages = warnings().map((warning) => warning.msg)
		expect(messages[0]).toMatch(/did not answer eth_supportedEntryPoints: HTTP 401/)
		expect(messages[1]).toMatch(/fetch failed \(getaddrinfo ENOTFOUND bundler\.example\)/)
	})

	it("warns and passes on a bundler that does not answer in time", async () => {
		stubBundlers({ [BUNDLER]: "hang" })
		await expect(
			assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService, { timeoutMs: 20 }),
		).resolves.toBeUndefined()
		expect(warnings()[0].msg).toMatch(/did not answer eth_supportedEntryPoints: timed out after 20ms/)
	})

	it("warns and passes on an answer that is not a list of addresses", async () => {
		for (const answer of [{ body: "<html>gateway</html>" }, { result: null }, { result: "0x" }, { result: [1] }]) {
			stubBundlers({ [BUNDLER]: answer })
			await expect(
				assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: BUNDLER }], configService),
			).resolves.toBeUndefined()
		}
		expect(warnings()).toHaveLength(4)
	})

	it("names the bundler by host alone, never its key-bearing path or query", async () => {
		stubBundlers({ [SECRET_BUNDLER]: { result: [V07] } })
		const missing = await assertBundlersServeEntryPoint(
			[{ chainId: 8453, bundlerUrl: SECRET_BUNDLER }],
			configService,
		).catch((err: Error) => err.message)
		expect(missing).toContain("api.pimlico.io")
		expect(missing).not.toContain("supersecretpimlicokey")
		expect(missing).not.toContain("/v2/8453")

		stubBundlers({ [SECRET_BUNDLER]: new Error(`request to ${SECRET_BUNDLER} failed`) })
		await assertBundlersServeEntryPoint([{ chainId: 8453, bundlerUrl: SECRET_BUNDLER }], configService)
		expect(warnings()[0].msg).toContain("request to api.pimlico.io failed")
		expect(logLines.join("\n")).not.toContain("supersecretpimlicokey")
		expect(logLines.join("\n")).not.toContain("/v2/8453")
	})

	it("refuses every chain whose bundler lacks the EntryPoint, and only warns about one that cannot answer", async () => {
		stubBundlers({
			"https://base.example": { result: [V07] },
			"https://arb.example": { result: [] },
			"https://op.example": { status: 500 },
		})
		const message = await assertBundlersServeEntryPoint(
			[
				{ chainId: 8453, bundlerUrl: "https://base.example" },
				{ chainId: 42161, bundlerUrl: "https://arb.example" },
				{ chainId: 10, bundlerUrl: "https://op.example" },
			],
			configService,
		).catch((err: Error) => err.message)
		expect(message).toContain("Base (EVM-8453)")
		expect(message).toContain("Arbitrum (EVM-42161)")
		expect(message).not.toContain("EVM-10")
		expect(warnings().map((warning) => warning.chain)).toEqual(["EVM-10"])
	})

	it("skips watch-only chains, and asks for chains marked false", async () => {
		const calls = stubBundlers({
			"https://base.example": { result: [V07] },
			"https://arb.example": { result: [V07] },
		})
		const chains = [
			{ chainId: 8453, bundlerUrl: "https://base.example" },
			{ chainId: 42161, bundlerUrl: "https://arb.example" },
		]

		await expect(
			assertBundlersServeEntryPoint(chains, configService, { watchOnly: { 8453: true, 42161: true } }),
		).resolves.toBeUndefined()
		expect(calls).toEqual([])

		await expect(
			assertBundlersServeEntryPoint(chains, configService, { watchOnly: { 8453: true, 42161: false } }),
		).rejects.toThrow(/Arbitrum \(EVM-42161\) does not support EntryPoint/)
		expect(calls.map((call) => call.url)).toEqual(["https://arb.example"])
	})

	it("skips chains with no bundler or no known EntryPoint", async () => {
		const calls = stubBundlers({})
		await expect(
			assertBundlersServeEntryPoint(
				[
					{ chainId: 8453, bundlerUrl: undefined },
					{ chainId: 8453, bundlerUrl: "  " },
					{ chainId: 31337, bundlerUrl: BUNDLER },
				],
				configService,
			),
		).resolves.toBeUndefined()
		expect(calls).toEqual([])
	})
})

describe("bundlerHost", () => {
	it("keeps the host and port, dropping credentials, path and query", () => {
		expect(bundlerHost("https://user:pass@bundler.example:8443/v1/abcdef?apikey=x")).toBe("bundler.example:8443")
		expect(bundlerHost("not a url")).toBe("an unparseable URL")
	})
})
