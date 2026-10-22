// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../api"
import { WalletTools } from "./WalletTools"

const USER_OP_HASH = `0x${"ab".repeat(32)}`
const TX_HASH = `0x${"cd".repeat(32)}`
const RECIPIENT = "0x5b2c3e25243634732eE94525Ae98aEc404c82506"

let root: Root
let element: HTMLDivElement
/** What the next `/api/send` post answers with. */
let sendReply: unknown
let onBalancesChanged: ReturnType<typeof vi.fn>
const flush = () => act(async () => {})

const button = (scope: ParentNode, label: string) => {
	const found = [...scope.querySelectorAll("button")].find((entry) => entry.textContent?.trim().startsWith(label))
	if (!found) throw new Error(`No "${label}" button`)
	return found
}
const click = (target: HTMLElement) => act(async () => target.click())

async function enter(input: HTMLInputElement, value: string) {
	await act(async () => {
		Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")?.set?.call(input, value)
		input.dispatchEvent(new Event("input", { bubbles: true }))
	})
}

/** Opens the send sheet, fills the form and submits it through the review dialog. */
async function send() {
	await click(element.querySelector<HTMLElement>(".operator-tool-row")!)
	const sheet = document.querySelector<HTMLElement>('[role="dialog"]')!
	await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!, "25")
	await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="0x…"]')!, RECIPIENT)
	await click(button(sheet, "Review transfer"))
	const review = [...document.querySelectorAll<HTMLElement>('[role="dialog"]')].at(-1)!
	await click(button(review, "Send 25 ETH"))
	await flush()
}

beforeEach(async () => {
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	// The sheet asks whether it is on a phone; jsdom has no media queries to answer.
	window.matchMedia ??= ((query: string) => ({
		matches: false,
		media: query,
		onchange: null,
		addEventListener: () => {},
		removeEventListener: () => {},
		addListener: () => {},
		removeListener: () => {},
		dispatchEvent: () => false,
	})) as typeof window.matchMedia
	sendReply = { txHash: TX_HASH, sponsored: true, redeemed: false }
	onBalancesChanged = vi.fn()
	vi.spyOn(api, "get").mockResolvedValue({
		toml: "",
		logLevel: "info",
		vaultConfigured: false,
		allowlistUsers: [],
		vaults: [],
		sendTokens: { "EVM-8453": [{ symbol: "ETH", address: "native" }] },
		knownVaults: {},
	})
	vi.spyOn(api, "post").mockImplementation(async () => sendReply)
	element = document.createElement("div")
	document.body.append(element)
	root = createRoot(element)
})
afterEach(async () => {
	await act(async () => root.unmount())
	element.remove()
	vi.restoreAllMocks()
})

const render = async () => {
	await act(async () =>
		root.render(
			<WalletTools
				chains={[8453]}
				chainLabels={{ "8453": "Base" }}
				onBalancesChanged={onBalancesChanged}
				onOpenChains={vi.fn()}
			/>,
		),
	)
	await flush()
}

describe("send funds", () => {
	it("shows the explorer-linked tx hash on a confirmed send", async () => {
		await render()
		await send()

		const text = document.querySelector<HTMLElement>('[role="dialog"]')?.textContent ?? ""
		expect(text).toContain("✓ Sent")
		const link = document.querySelector<HTMLAnchorElement>(".operator-send-tx-link")
		expect(link?.getAttribute("href")).toContain(`/tx/${TX_HASH}`)
	})

	it("shows a pending warning and clears the amount when the send is unconfirmed", async () => {
		sendReply = {
			pending: true,
			sponsored: true,
			userOpHash: USER_OP_HASH,
			message: "The send was submitted but is not confirmed yet.",
		}
		await render()
		await send()

		const warning = document.querySelector<HTMLElement>("p.warning")
		expect(warning?.textContent).toContain("not confirmed yet")
		expect(warning?.textContent).toContain(`${USER_OP_HASH.slice(0, 18)}…`)
		// A UserOp has no tx hash: no success line, no explorer link.
		expect(document.querySelector<HTMLElement>('[role="dialog"]')?.textContent ?? "").not.toContain("✓ Sent")
		expect(warning?.querySelector("a")).toBeNull()
		// The amount is cleared so the next click cannot resend it.
		expect(document.querySelector<HTMLInputElement>('input[placeholder="0.00"]')?.value).toBe("")
		// The balance refresh still runs, so the operator sees what actually left the wallet.
		expect(onBalancesChanged).toHaveBeenCalledOnce()
	})

	it("links a pending native send's tx hash to the explorer", async () => {
		sendReply = {
			pending: true,
			sponsored: false,
			txHash: TX_HASH,
			message: "The send was submitted but is not confirmed yet.",
		}
		await render()
		await send()

		const warning = document.querySelector<HTMLElement>("p.warning")
		const link = warning?.querySelector<HTMLAnchorElement>(".operator-send-tx-link")
		expect(link?.getAttribute("href")).toContain(`/tx/${TX_HASH}`)
	})
})
