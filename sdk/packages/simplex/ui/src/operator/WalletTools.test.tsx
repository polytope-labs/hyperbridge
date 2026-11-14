// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"
import { api } from "../api"
import type { BalanceSnapshot } from "../types"
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

/** Opens the send dialog. */
async function openSend() {
	await click(element.querySelector<HTMLElement>(".operator-tool-row")!)
	return document.querySelector<HTMLElement>('[role="dialog"]')!
}

/** Opens the send dialog, fills the form and submits it through the review dialog. */
async function send() {
	const sheet = await openSend()
	await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!, "25")
	await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="Recipient address"]')!, RECIPIENT)
	await click(button(sheet, "Send"))
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

const render = async (balances?: BalanceSnapshot) => {
	await act(async () =>
		root.render(
			<WalletTools
				chains={[8453]}
				chainLabels={{ "8453": "Base" }}
				balances={balances}
				onBalancesChanged={onBalancesChanged}
				onOpenChains={vi.fn()}
			/>,
		),
	)
	await flush()
}

describe("send funds", () => {
	it("fills the amount from a share of the balance, rounded down", async () => {
		await render({
			updatedAt: 1,
			status: "fresh",
			issues: [],
			chains: [{ chainId: 8453, native: { symbol: "ETH", amount: 1.0000009 }, assets: [] }],
		} as unknown as BalanceSnapshot)
		const sheet = await openSend()
		const amount = sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!
		expect(sheet.querySelector(".send-asset-balance")?.textContent).toBe("1")

		await click(button(sheet, "Max"))
		expect(amount.value).toBe("1")
		expect(button(sheet, "Max").getAttribute("aria-pressed")).toBe("true")
		await click(button(sheet, "50%"))
		expect(amount.value).toBe("0.5")
	})

	it("offers no share of a balance it has not read, and says when an address is not one", async () => {
		await render()
		const sheet = await openSend()
		expect(button(sheet, "Max").disabled).toBe(true)

		const recipient = sheet.querySelector<HTMLInputElement>('input[placeholder="Recipient address"]')!
		await enter(recipient, "0x1234")
		expect(sheet.querySelector(".send-address-note")?.textContent).toBe("Enter a valid address")
		expect(button(sheet, "Send").disabled).toBe(true)
		await enter(recipient, RECIPIENT)
		expect(sheet.querySelector(".send-address-note")?.textContent).toBe("Valid address")
	})

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
