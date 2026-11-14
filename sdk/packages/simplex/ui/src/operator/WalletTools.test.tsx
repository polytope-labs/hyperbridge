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

/** Opens the send dialog, fills the form and submits it through the review step. */
async function send() {
	const sheet = await openSend()
	await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!, "25")
	await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="Recipient address"]')!, RECIPIENT)
	await click(button(sheet, "Send"))
	await click(button(sheet.querySelector<HTMLElement>(".send-review")!, "Send 25 ETH"))
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

	it("lists the tokens the wallet holds with the largest balance first, and sends the one picked", async () => {
		const usdc = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
		const usdt = "0xfde4C96c8593536E31F229EA8f37b2ADa2699bb2"
		const dai = "0x50c5725949A6F0c72E6C4a641F24049A917DB0Cb"
		vi.mocked(api.get).mockResolvedValue({
			toml: "",
			logLevel: "info",
			vaultConfigured: false,
			allowlistUsers: [],
			vaults: [],
			sendTokens: {
				"EVM-8453": [
					{ symbol: "ETH", address: "native" },
					{ symbol: "USDT", address: usdt },
					{ symbol: "DAI", address: dai },
					{ symbol: "USDC", address: usdc },
				],
			},
			knownVaults: {},
		})
		// USDT is dust that shows as zero, and DAI's balance was not read: neither is listed.
		await render({
			updatedAt: 1,
			status: "fresh",
			issues: [],
			chains: [
				{
					chainId: 8453,
					native: { symbol: "ETH", amount: 1 },
					assets: [
						{ address: usdt, symbol: "USDT", wallet: 0.00001, available: 0.00001, vaults: [] },
						{ address: usdc, symbol: "USDC", wallet: 2500.5, available: 2500.5, vaults: [] },
					],
				},
			],
		} as unknown as BalanceSnapshot)
		const sheet = await openSend()
		const trigger = sheet.querySelector<HTMLElement>(".send-asset-trigger")!
		expect(sheet.querySelector(".send-asset-list")).toBeNull()

		await click(trigger)
		const rows = [...sheet.querySelectorAll<HTMLElement>(".send-asset-option")]
		expect(rows.map((row) => row.textContent)).toEqual(["USDCBase2,500.5", "ETHBase1"])
		expect(rows[1].getAttribute("aria-current")).toBe("true")

		await click(rows[0])
		expect(sheet.querySelector(".send-asset-list")).toBeNull()
		expect(trigger.textContent).toBe("USDCBase2,500.5")

		await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!, "1000")
		await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="Recipient address"]')!, RECIPIENT)
		await click(button(sheet, "Send"))
		await click(button(sheet.querySelector<HTMLElement>(".send-review")!, "Send 1,000 USDC"))
		await flush()
		expect(api.post).toHaveBeenCalledWith("/api/send", expect.objectContaining({ token: usdc, amount: "1000" }))
	})

	it("groups thousands in the amount while sending the plain value", async () => {
		await render()
		const sheet = await openSend()
		const amount = sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!
		await enter(amount, "2000")
		expect(amount.value).toBe("2,000")
		// What the browser reports after a digit is typed into the grouped text.
		await enter(amount, "2,5000.5")
		expect(amount.value).toBe("25,000.5")

		await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="Recipient address"]')!, RECIPIENT)
		await click(button(sheet, "Send"))
		await click(button(sheet.querySelector<HTMLElement>(".send-review")!, "Send 25,000.5 ETH"))
		await flush()
		expect(api.post).toHaveBeenCalledWith("/api/send", expect.objectContaining({ amount: "25000.5" }))
		expect(document.querySelector(".send-success")?.textContent).toContain("25,000.5 ETH sent on Base")
	})

	it("reviews the transfer inside the send dialog, and goes back to the form as it was", async () => {
		await render()
		const sheet = await openSend()
		const amount = sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!
		await enter(amount, "1000")
		await enter(sheet.querySelector<HTMLInputElement>('input[placeholder="Recipient address"]')!, RECIPIENT)
		await click(button(sheet, "Send"))

		// The review takes the form's place: no second dialog opens over the first.
		expect(document.querySelectorAll('[role="dialog"]')).toHaveLength(1)
		expect(sheet.textContent).toContain("Review transfer")
		expect(sheet.querySelector(".send-review-headline strong")?.textContent).toBe("1,000 ETH (native)")
		expect(sheet.querySelector<HTMLElement>(".send-form")?.hidden).toBe(true)

		await click(button(sheet.querySelector<HTMLElement>(".send-review")!, "Back"))
		expect(sheet.querySelector(".send-review")).toBeNull()
		expect(sheet.querySelector<HTMLElement>(".send-form")?.hidden).toBe(false)
		expect(amount.value).toBe("1,000")
		expect(api.post).not.toHaveBeenCalled()
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

		// A confirmed send replaces the form with the success screen.
		const sent = document.querySelector<HTMLElement>(".send-success")
		expect(sent?.textContent).toContain("Funds sent")
		expect(sent?.textContent).toContain("25 ETH sent on Base")
		expect(sent?.querySelector(".send-success-mark svg")).not.toBeNull()
		expect(sent?.querySelector("a")?.getAttribute("href")).toContain(`/tx/${TX_HASH}`)
		expect(document.querySelector<HTMLElement>(".send-form")?.hidden).toBe(true)

		// Done closes the dialog, and it opens on the form again.
		await click(button(sent!, "Done"))
		const reopened = await openSend()
		expect(reopened.querySelector(".send-success")).toBeNull()
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
		expect(document.querySelector(".send-success")).toBeNull()
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
