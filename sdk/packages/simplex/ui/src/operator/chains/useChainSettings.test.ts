// @vitest-environment jsdom
import { act, createElement } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { chainsForNetwork } from "@/cli/init/chains"
import type { BalanceSnapshot, ChainFunding, ChainRowDto, ChainsDto } from "../../types"
import { Chains } from "../Chains"
import { useChainSettings } from "./useChainSettings"

type Model = ReturnType<typeof useChainSettings>
let model: Model
let root: Root
let container: HTMLDivElement
let dto: ChainsDto
let savedChains: { chains: Array<Pick<ChainRowDto, "chainId" | "rpcUrls" | "watchOnly">> } | undefined
const requests = vi.fn()

function Harness() {
	model = useChainSettings()
	return null
}

async function mount() {
	await act(async () => root.render(createElement(Harness)))
}

/** Renders the Chains panel with Base's funding as the balances report it. */
async function renderChains(baseFunding?: ChainFunding, onBalancesChanged = () => {}) {
	const balances = baseFunding
		? ({ chains: [{ chainId: 8453, assets: [], funding: baseFunding }] } as unknown as BalanceSnapshot)
		: undefined
	await act(async () => root.render(createElement(Chains, { balances, onBalancesChanged })))
}

function card(label: string) {
	const found = Array.from(container.querySelectorAll(".chain-configuration")).find(
		(card) => card.querySelector("h2")?.textContent === label,
	)
	if (!found) throw new Error(`Missing ${label} card`)
	return found
}

function fillsSwitch(label: string) {
	const toggle = card(label).querySelector<HTMLInputElement>('.chain-enable-toggle input[type="checkbox"]')
	if (!toggle) throw new Error(`Missing ${label} switch`)
	return toggle
}

const usdc = (balance: number, overrides: Partial<ChainFunding> = {}): ChainFunding => ({
	feeToken: { symbol: "USDC", balance },
	nativeSetup: null,
	funded: balance > 0,
	delegated: false,
	...overrides,
})

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
				bundlerUrl: "https://bundler.polytope.technology/base",
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
							// The server reports the chain's Hyperbridge bundler; the panel never sends one.
							bundlerUrl: meta?.hyperbridgeBundlerUrl ?? "",
							stateMachineId: meta?.stateMachineId ?? previous?.stateMachineId ?? `EVM-${row.chainId}`,
							label: meta?.label ?? previous?.label ?? String(row.chainId),
							running: previous?.running ?? false,
						}
					}),
				}
				return new Response(JSON.stringify({ ok: true }))
			}
			if (path === "/api/chains/enable-fills") return new Response(JSON.stringify({ chains: [] }))
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
		await renderChains()
		await act(async () => fillsSwitch("Ethereum").click())
		// Enabling a chain leaves its card closed; the chevron opens it.
		expect(card("Ethereum").querySelector('input[type="text"]')).toBeNull()
		await act(async () => card("Ethereum").querySelector<HTMLButtonElement>(".chain-collapse-trigger")?.click())
		const fields = () => Array.from(card("Ethereum").querySelectorAll<HTMLInputElement>('input[type="text"]'))
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
					running: true,
				})
			} else {
				expect(draft).toMatchObject({ enabled: false, rpcUrls: meta.defaultRpcUrls })
			}
		}
		await act(async () => model.toggleChain(chain(1), true))
		expect(chain(1)).toMatchObject({ enabled: true, rpcUrls: dto.catalog[0].defaultRpcUrls })
	})

	it("shows the Hyperbridge bundler for every catalog chain", async () => {
		await mount()
		for (const meta of dto.catalog) {
			expect(meta.hyperbridgeBundlerUrl, meta.label).toMatch(/^https:\/\/bundler\.polytope\.technology\//)
			expect(chain(meta.chainId).bundlerUrl).toBe(meta.hyperbridgeBundlerUrl)
		}
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

	it("preserves configured chains outside the catalog, with no bundler", async () => {
		const custom = {
			...dto.chains[0],
			chainId: 12345,
			stateMachineId: "EVM-12345",
			label: "Custom",
			bundlerUrl: "",
		}
		dto.chains.push(custom)
		await mount()
		expect(chain(12345)).toMatchObject({ enabled: true, rpcUrls: custom.rpcUrls, bundlerUrl: "" })
	})

	it("will not enable a chain Hyperbridge runs no bundler for", async () => {
		dto.catalog = [{ ...dto.catalog[0], hyperbridgeBundlerUrl: undefined }]
		dto.chains = []
		await renderChains()
		const toggle = container.querySelector<HTMLInputElement>('.chain-enable-toggle input[type="checkbox"]')
		if (!toggle) throw new Error("Missing enable switch")
		expect(toggle.disabled).toBe(true)
		expect(container.querySelector(".chain-configuration")?.textContent).toContain("Watch only")
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

	it("saves a new chain and a configured one without a bundler, then reseeds from the saved config", async () => {
		await mount()
		const rpcUrls = [...(dto.catalog[0].defaultRpcUrls ?? [])]
		const savedBaseRpcUrls = [...chain(8453).rpcUrls]
		await act(async () => model.toggleChain(chain(1), true))
		await act(async () => model.save())
		expect(savedChains).toEqual({
			chains: [
				{ chainId: 1, rpcUrls, watchOnly: false },
				// Limit orders choose where to fill, so a chain with a bundler is never saved watch-only.
				{ chainId: 8453, rpcUrls: savedBaseRpcUrls, watchOnly: false },
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
	it("locks fills on a running chain until the wallet holds the fee token there", async () => {
		await renderChains(usdc(0))
		expect(fillsSwitch("Base").disabled).toBe(true)
		expect(fillsSwitch("Base").checked).toBe(false)
		expect(card("Base").textContent).toContain("Needs at least 5 USDC")
	})

	it("names the BNB a chain still needs for the paymaster's setup", async () => {
		await renderChains(
			usdc(250, {
				feeToken: { symbol: "USDT", balance: 250 },
				funded: false,
				nativeSetup: { required: 0.000142, feeToken: "USDT" },
			}),
		)
		expect(fillsSwitch("Base").disabled).toBe(true)
		expect(card("Base").textContent).toContain("Needs about 0.00014")
	})

	it("enables fills on a funded running chain at once, then reloads the balances", async () => {
		const reload = vi.fn()
		await renderChains(usdc(250), reload)
		expect(fillsSwitch("Base").disabled).toBe(false)
		await act(async () => fillsSwitch("Base").click())
		const call = requests.mock.calls.find(([path]) => path === "/api/chains/enable-fills")
		expect(call?.[1].method).toBe("POST")
		expect(JSON.parse(call?.[1].body as string)).toEqual({ chainId: 8453 })
		expect(reload).toHaveBeenCalled()
	})

	it("shows fills enabled once the wallet is delegated, with nothing left to switch", async () => {
		await renderChains(usdc(250, { delegated: true }))
		expect(fillsSwitch("Base").checked).toBe(true)
		expect(fillsSwitch("Base").disabled).toBe(true)
		expect(card("Base").textContent).toContain("Fills enabled")
	})
})
