// @vitest-environment jsdom
import { act, createElement } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { chainsForNetwork } from "@/cli/init/chains"
import type { ChainRowDto, ChainsDto } from "../../types"
import { Chains } from "../Chains"
import { useChainSettings } from "./useChainSettings"

type Model = ReturnType<typeof useChainSettings>
let model: Model
let root: Root
let container: HTMLDivElement
let dto: ChainsDto
let alchemyResponse: {
	valid: boolean
	chains: Array<{ chainId: number; rpcUrl: string | null; bundlerUrl: string | null }>
}
let savedChains: { chains: Array<Pick<ChainRowDto, "chainId" | "rpcUrls" | "bundlerUrl" | "watchOnly">> } | undefined
const requests = vi.fn()

function Harness() {
	model = useChainSettings()
	return null
}

async function mount() {
	await act(async () => root.render(createElement(Harness)))
}

function chain(chainId: number) {
	const row = model.chains.find((row) => row.meta.chainId === chainId)
	if (!row) throw new Error(`Missing chain ${chainId}`)
	return row
}

async function enter(input: HTMLInputElement, value: string) {
	await act(async () => {
		Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value)
		input.dispatchEvent(new Event("input", { bubbles: true }))
	})
}

beforeEach(() => {
	dto = {
		network: "mainnet",
		globalWatchOnly: false,
		catalog: chainsForNetwork("mainnet"),
		chains: [
			{
				chainId: 8453,
				stateMachineId: "EVM-8453",
				label: "Base",
				rpcUrls: ["https://saved.example", "https://backup.example"],
				bundlerUrl: "https://saved-bundler.example",
				watchOnly: true,
				running: true,
			},
		],
	}
	alchemyResponse = {
		valid: true,
		chains: [
			{ chainId: 1, rpcUrl: "https://eth.g.alchemy.com/v2/test", bundlerUrl: "https://eth-bundler.example" },
			{ chainId: 8453, rpcUrl: "https://base.g.alchemy.com/v2/test", bundlerUrl: "https://base-bundler.example" },
		],
	}
	savedChains = undefined
	requests.mockReset()
	vi.stubGlobal("IS_REACT_ACT_ENVIRONMENT", true)
	vi.stubGlobal(
		"fetch",
		requests.mockImplementation(async (path: string, init?: RequestInit) => {
			if (path === "/api/chains" && init?.method === "PUT") {
				const submitted = JSON.parse(init.body as string) as NonNullable<typeof savedChains>
				savedChains = submitted
				dto = {
					...dto,
					chains: submitted.chains.map((row) => {
						const previous = dto.chains.find((chain) => chain.chainId === row.chainId)
						const meta = dto.catalog.find((chain) => chain.chainId === row.chainId)
						return {
							...row,
							stateMachineId: meta?.stateMachineId ?? previous?.stateMachineId ?? `EVM-${row.chainId}`,
							label: meta?.label ?? previous?.label ?? String(row.chainId),
							running: previous?.running ?? false,
						}
					}),
				}
				return new Response(JSON.stringify({ ok: true }))
			}
			if (path === "/api/chains") return new Response(JSON.stringify(dto))
			if (path === "/api/setup/validate-alchemy-key") return new Response(JSON.stringify(alchemyResponse))
			if (path === "/api/setup/validate-rpc") return new Response(JSON.stringify({ ok: true, results: [] }))
			if (path === "/api/setup/validate-bundler") return new Response(JSON.stringify({ ok: true }))
			throw new Error(`Unexpected request: ${path}`)
		}),
	)
	container = document.createElement("div")
	document.body.append(container)
	root = createRoot(container)
})

afterEach(async () => {
	await act(async () => root.unmount())
	container.remove()
	vi.unstubAllGlobals()
})

describe("operator chain settings", () => {
	it("shows default fields and saves edits through the rendered Chains panel", async () => {
		await act(async () => root.render(createElement(Chains)))
		const card = Array.from(container.querySelectorAll(".chain-configuration")).find(
			(card) => card.querySelector("h2")?.textContent === "Ethereum",
		)
		if (!card) throw new Error("Missing Ethereum card")
		const toggle = card.querySelector<HTMLInputElement>('input[type="checkbox"]')
		if (!toggle) throw new Error("Missing enable switch")
		await act(async () => toggle.click())
		const rpcs = () => Array.from(card.querySelectorAll<HTMLInputElement>('.row input[type="text"]'))
		expect(rpcs().map((input) => input.value)).toEqual(dto.catalog[0].defaultRpcUrls)
		const key = container.querySelector<HTMLInputElement>('input[aria-label="Alchemy API key"]')
		if (!key) throw new Error("Missing Alchemy key field")
		await enter(key, "test-key")
		const prefill = Array.from(container.querySelectorAll("button")).find(
			(button) => button.textContent === "Validate & prefill",
		)
		if (!prefill) throw new Error("Missing prefill button")
		expect(prefill.disabled).toBe(false)
		await act(async () => prefill.click())
		expect(rpcs().map((input) => input.value)).toEqual(dto.catalog[0].defaultRpcUrls)
		expect(card.textContent).toContain("Bundler via Alchemy")
		await enter(rpcs()[0], "https://edited.example")
		expect(card.textContent).toContain("Bundler via Alchemy")
		const bundler = card.querySelector<HTMLInputElement>('input[placeholder*="pimlico"]')
		if (!bundler) throw new Error("Missing bundler field")
		expect(bundler.value).toBe("https://eth-bundler.example")
		await enter(bundler, "https://custom-bundler.example")
		expect(card.textContent).not.toContain("Bundler via Alchemy")
		const save = Array.from(container.querySelectorAll("button")).find(
			(button) => button.textContent === "Save chain settings",
		)
		if (!save) throw new Error("Missing save button")
		await act(async () => save.click())
		expect(savedChains).toMatchObject({
			chains: expect.arrayContaining([
				{
					chainId: 1,
					rpcUrls: ["https://edited.example", ...(dto.catalog[0].defaultRpcUrls?.slice(1) ?? [])],
					bundlerUrl: "https://custom-bundler.example",
					watchOnly: false,
				},
			]),
		})
		expect(rpcs().map((input) => input.value)).toEqual([
			"https://edited.example",
			...(dto.catalog[0].defaultRpcUrls?.slice(1) ?? []),
		])
		expect(bundler.value).toBe("https://custom-bundler.example")
	})

	it("loads bundled RPCs for every new mainnet chain and preserves saved endpoints", async () => {
		await mount()
		for (const meta of dto.catalog) {
			const draft = chain(meta.chainId)
			if (meta.chainId === 8453) {
				expect(draft).toMatchObject({
					enabled: true,
					rpcUrls: dto.chains[0].rpcUrls,
					watchOnly: true,
					running: true,
				})
			} else {
				expect(draft).toMatchObject({ enabled: false, rpcUrls: meta.defaultRpcUrls, bundlerUrl: "" })
			}
		}
		await act(async () => model.toggleChain(chain(1), true))
		expect(chain(1)).toMatchObject({ enabled: true, rpcUrls: dto.catalog[0].defaultRpcUrls })
	})

	it("keeps one empty RPC field when bundled defaults are missing or empty", async () => {
		dto.catalog = dto.catalog.slice(0, 2).map((meta, index) => ({
			...meta,
			defaultRpcUrls: index === 0 ? undefined : [],
		}))
		dto.chains = []
		await mount()
		for (const draft of model.chains) expect(draft.rpcUrls).toEqual([""])
	})

	it("preserves configured chains outside the catalog", async () => {
		const custom = { ...dto.chains[0], chainId: 12345, stateMachineId: "EVM-12345", label: "Custom" }
		dto.chains.push(custom)
		await mount()
		expect(chain(12345)).toMatchObject({ enabled: true, rpcUrls: custom.rpcUrls, bundlerUrl: custom.bundlerUrl })
	})

	it("fills Alchemy bundlers while preserving public and saved RPC lists", async () => {
		await mount()
		await act(async () => model.patch(1, { verificationState: "success", verificationMessage: "Verified" }))
		await act(async () => model.updateAlchemyKey(" test-key "))
		await act(async () => model.applyAlchemyKey())
		expect(chain(1)).toMatchObject({
			rpcUrls: dto.catalog[0].defaultRpcUrls,
			bundlerUrl: "https://eth-bundler.example",
			viaAlchemy: true,
			verificationState: undefined,
			verificationMessage: undefined,
		})
		expect(chain(8453)).toMatchObject({
			rpcUrls: dto.chains[0].rpcUrls,
			bundlerUrl: "https://base-bundler.example",
		})
		expect(chain(42161)).toMatchObject({
			rpcUrls: dto.catalog.find((meta) => meta.chainId === 42161)?.defaultRpcUrls,
			viaAlchemy: false,
		})
		const request = requests.mock.calls.find(([path]) => path === "/api/setup/validate-alchemy-key")
		expect(JSON.parse(request?.[1].body as string)).toEqual({ apiKey: "test-key", network: "mainnet" })
	})

	it("uses the Alchemy RPC URL as a bundler fallback without replacing an empty RPC field", async () => {
		dto.catalog = [{ ...dto.catalog[0], defaultRpcUrls: undefined }]
		dto.chains = []
		alchemyResponse.chains = [{ chainId: 1, rpcUrl: "https://eth.g.alchemy.com/v2/test", bundlerUrl: null }]
		await mount()
		await act(async () => model.updateAlchemyKey("test-key"))
		await act(async () => model.applyAlchemyKey())
		expect(chain(1)).toMatchObject({
			rpcUrls: [""],
			bundlerUrl: "https://eth.g.alchemy.com/v2/test",
			viaAlchemy: true,
		})
	})

	it("leaves endpoints unchanged when the Alchemy key is invalid", async () => {
		alchemyResponse.valid = false
		await mount()
		const before = model.chains
		await act(async () => model.updateAlchemyKey("bad-key"))
		await act(async () => model.applyAlchemyKey())
		expect(model.alchemy.status).toBe("err")
		expect(model.chains).toEqual(before)
	})

	it("verifies and saves new defaults with the bundler, then reseeds from the saved config", async () => {
		await mount()
		const rpcUrls = [...(dto.catalog[0].defaultRpcUrls ?? [])]
		const savedBaseRpcUrls = [...chain(8453).rpcUrls]
		await act(async () => model.toggleChain(chain(1), true))
		await act(async () => model.updateAlchemyKey("test-key"))
		await act(async () => model.applyAlchemyKey())
		await act(async () => model.verifyChain(chain(1)))
		expect(chain(1).verificationState).toBe("success")
		const probe = requests.mock.calls.find(([path]) => path === "/api/setup/validate-rpc")
		expect(JSON.parse(probe?.[1].body as string)).toEqual({ urls: rpcUrls, expectedChainId: 1 })
		await act(async () => model.save())
		expect(savedChains).toEqual({
			chains: [
				{ chainId: 1, rpcUrls, bundlerUrl: "https://eth-bundler.example", watchOnly: false },
				{
					chainId: 8453,
					rpcUrls: savedBaseRpcUrls,
					bundlerUrl: "https://base-bundler.example",
					watchOnly: true,
				},
			],
		})
		expect(model.saved).toBe(true)
		expect(chain(1)).toMatchObject({
			enabled: true,
			rpcUrls,
			bundlerUrl: "https://eth-bundler.example",
			running: false,
		})
		expect(chain(8453)).toMatchObject({
			rpcUrls: savedBaseRpcUrls,
			bundlerUrl: "https://base-bundler.example",
			running: true,
		})
		expect(chain(42161).rpcUrls).toEqual(dto.catalog.find((meta) => meta.chainId === 42161)?.defaultRpcUrls)
	})
})
