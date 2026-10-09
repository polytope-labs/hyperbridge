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
			if (path === "/api/setup/validate-rpc") return new Response(JSON.stringify({ ok: true, results: [] }))
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

const HYPERBRIDGE_BUNDLER = {
	1: "https://bundler.polytope.technology/ethereum",
	8453: "https://bundler.polytope.technology/base",
}

describe("operator chain settings", () => {
	it("shows RPC fields alone and saves edits through the rendered Chains panel", async () => {
		await act(async () => root.render(createElement(Chains)))
		const card = Array.from(container.querySelectorAll(".chain-configuration")).find(
			(card) => card.querySelector("h2")?.textContent === "Ethereum",
		)
		if (!card) throw new Error("Missing Ethereum card")
		const toggle = card.querySelector<HTMLInputElement>('input[type="checkbox"]')
		if (!toggle) throw new Error("Missing enable switch")
		await act(async () => toggle.click())
		const fields = () => Array.from(card.querySelectorAll<HTMLInputElement>('input[type="text"]'))
		// Every text field on the card is an RPC endpoint: there is no bundler to enter.
		expect(fields().map((input) => input.value)).toEqual(dto.catalog[0].defaultRpcUrls)
		expect(container.textContent).not.toMatch(/bundler|alchemy/i)
		expect(container.querySelector('input[type="password"]')).toBeNull()
		await enter(fields()[0], "https://edited.example")
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
					bundlerUrl: HYPERBRIDGE_BUNDLER[1],
					watchOnly: false,
				},
			]),
		})
		expect(fields().map((input) => input.value)).toEqual([
			"https://edited.example",
			...(dto.catalog[0].defaultRpcUrls?.slice(1) ?? []),
		])
	})

	it("loads bundled RPCs for every new mainnet chain and preserves saved RPC endpoints", async () => {
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
				expect(draft).toMatchObject({ enabled: false, rpcUrls: meta.defaultRpcUrls })
			}
		}
		await act(async () => model.toggleChain(chain(1), true))
		expect(chain(1)).toMatchObject({ enabled: true, rpcUrls: dto.catalog[0].defaultRpcUrls })
	})

	it("uses the Hyperbridge bundler for every catalog chain, whatever the config held", async () => {
		await mount()
		for (const meta of dto.catalog) {
			expect(meta.hyperbridgeBundlerUrl, meta.label).toMatch(/^https:\/\/bundler\.polytope\.technology\//)
			expect(chain(meta.chainId).bundlerUrl).toBe(meta.hyperbridgeBundlerUrl)
		}
		// Base was saved with another bundler.
		expect(dto.chains[0].bundlerUrl).toBe("https://saved-bundler.example")
		expect(chain(8453).bundlerUrl).toBe(HYPERBRIDGE_BUNDLER[8453])
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

	it("preserves configured chains outside the catalog, with the bundler the config names", async () => {
		const custom = { ...dto.chains[0], chainId: 12345, stateMachineId: "EVM-12345", label: "Custom" }
		dto.chains.push(custom)
		await mount()
		expect(chain(12345)).toMatchObject({ enabled: true, rpcUrls: custom.rpcUrls, bundlerUrl: custom.bundlerUrl })
	})

	it("will not enable a chain that has neither a Hyperbridge bundler nor one in the config", async () => {
		dto.catalog = [{ ...dto.catalog[0], hyperbridgeBundlerUrl: undefined }]
		dto.chains = []
		await act(async () => root.render(createElement(Chains)))
		const toggle = container.querySelector<HTMLInputElement>('.chain-enable-toggle input[type="checkbox"]')
		if (!toggle) throw new Error("Missing enable switch")
		expect(toggle.disabled).toBe(true)
		expect(container.querySelector(".chain-configuration")?.textContent).toContain("Add in the config file")
	})

	it("verifies the RPC endpoints alone", async () => {
		await mount()
		await act(async () => model.toggleChain(chain(1), true))
		await act(async () => model.verifyChain(chain(1)))
		expect(chain(1)).toMatchObject({
			verificationState: "success",
			verificationMessage: "RPC connection is ready.",
		})
		const probes = requests.mock.calls.map(([path]) => path).filter((path) => path.startsWith("/api/setup/"))
		expect(probes).toEqual(["/api/setup/validate-rpc"])
		const probe = requests.mock.calls.find(([path]) => path === "/api/setup/validate-rpc")
		expect(JSON.parse(probe?.[1].body as string)).toEqual({
			urls: dto.catalog[0].defaultRpcUrls,
			expectedChainId: 1,
		})
	})

	it("saves a new chain and a configured one on the Hyperbridge bundler, then reseeds from the saved config", async () => {
		await mount()
		const rpcUrls = [...(dto.catalog[0].defaultRpcUrls ?? [])]
		const savedBaseRpcUrls = [...chain(8453).rpcUrls]
		await act(async () => model.toggleChain(chain(1), true))
		await act(async () => model.save())
		expect(savedChains).toEqual({
			chains: [
				{ chainId: 1, rpcUrls, bundlerUrl: HYPERBRIDGE_BUNDLER[1], watchOnly: false },
				{ chainId: 8453, rpcUrls: savedBaseRpcUrls, bundlerUrl: HYPERBRIDGE_BUNDLER[8453], watchOnly: true },
			],
		})
		expect(model.saved).toBe(true)
		expect(chain(1)).toMatchObject({ enabled: true, rpcUrls, bundlerUrl: HYPERBRIDGE_BUNDLER[1], running: false })
		expect(chain(8453)).toMatchObject({
			rpcUrls: savedBaseRpcUrls,
			bundlerUrl: HYPERBRIDGE_BUNDLER[8453],
			running: true,
		})
		expect(chain(42161).rpcUrls).toEqual(dto.catalog.find((meta) => meta.chainId === 42161)?.defaultRpcUrls)
	})
})
