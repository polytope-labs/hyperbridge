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

const amountInput = (sheet: ParentNode) => sheet.querySelector<HTMLInputElement>('input[placeholder="0.00"]')!
const recipientInput = (sheet: ParentNode) =>
	sheet.querySelector<HTMLInputElement>('input[placeholder="Recipient address"]')!
const optionTexts = (sheet: ParentNode) =>
	[...sheet.querySelectorAll<HTMLElement>(".send-asset-option")].map((row) => row.textContent)

/** Confirms the open review, once its confirm button has come on. */
async function confirmReview(sheet: ParentNode, label: string) {
	await act(async () => vi.advanceTimersByTime(500))
	await click(button(sheet.querySelector<HTMLElement>(".send-review")!, label))
	await flush()
}

/** Opens the send dialog, fills the form and submits it through the review step. */
async function send() {
	const sheet = await openSend()
	await enter(amountInput(sheet), "25")
	await enter(recipientInput(sheet), RECIPIENT)
	await click(button(sheet, "Send"))
	await confirmReview(sheet, "Send 25 ETH")
}

const NATIVE_ONLY = [{ symbol: "ETH", address: "native" }]
/** The config the dialog reads its sendable tokens from. */
const config = (sendTokens: Record<string, { symbol: string; address: string }[]>) => ({
	toml: "",
	logLevel: "info",
	vaultConfigured: false,
	allowlistUsers: [],
	vaults: [],
	sendTokens,
	knownVaults: {},
})
const snapshot = (chains: unknown[]) =>
	({ updatedAt: 1, status: "fresh", issues: [], chains }) as unknown as BalanceSnapshot

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
	// The review's confirm button comes on after a delay, which the tests step over.
	vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"] })
	vi.spyOn(api, "get").mockResolvedValue(config({ "EVM-8453": NATIVE_ONLY }))
	vi.spyOn(api, "post").mockImplementation(async () => sendReply)
	element = document.createElement("div")
	document.body.append(element)
	root = createRoot(element)
})
afterEach(async () => {
	await act(async () => root.unmount())
	element.remove()
	vi.useRealTimers()
	vi.restoreAllMocks()
})

const render = async (balances?: BalanceSnapshot, chains = [8453]) => {
	await act(async () =>
		root.render(
			<WalletTools
				chains={chains}
				chainLabels={{ "8453": "Base", "56": "BNB Chain" }}
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

		await enter(amountInput(sheet), "1000")
		await enter(recipientInput(sheet), RECIPIENT)
		await click(button(sheet, "Send"))
		await confirmReview(sheet, "Send 1,000 USDC")
		expect(api.post).toHaveBeenCalledWith("/api/send", {
			chain: "EVM-8453",
			token: usdc,
			amount: "1000",
			to: RECIPIENT,
		})
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
		await confirmReview(sheet, "Send 25,000.5 ETH")
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

	it("does not confirm on a tap that lands as the review appears", async () => {
		await render()
		const sheet = await openSend()
		await enter(amountInput(sheet), "25")
		await enter(recipientInput(sheet), RECIPIENT)
		await click(button(sheet, "Send"))

		const confirm = button(sheet.querySelector<HTMLElement>(".send-review")!, "Send 25 ETH")
		expect(confirm.disabled).toBe(true)
		await click(confirm)
		await flush()
		expect(api.post).not.toHaveBeenCalled()

		await act(async () => vi.advanceTimersByTime(500))
		expect(confirm.disabled).toBe(false)
	})

	describe("with two networks", () => {
		const twoNetworks = snapshot([
			{ chainId: 8453, native: { symbol: "ETH", amount: 1 }, assets: [] },
			{ chainId: 56, native: { symbol: "BNB", amount: 3 }, assets: [] },
		])
		beforeEach(() => {
			vi.mocked(api.get).mockResolvedValue(
				config({ "EVM-8453": NATIVE_ONLY, "EVM-56": [{ symbol: "BNB", address: "native" }] }),
			)
		})

		it("sends the picked token on its own network", async () => {
			await render(twoNetworks, [8453, 56])
			const sheet = await openSend()
			await click(sheet.querySelector<HTMLElement>(".send-asset-trigger")!)
			expect(optionTexts(sheet)).toEqual(["BNBBNB Chain3", "ETHBase1"])
			await click(sheet.querySelector<HTMLElement>(".send-asset-option")!)

			await enter(amountInput(sheet), "2")
			await enter(recipientInput(sheet), RECIPIENT)
			await click(button(sheet, "Send"))
			await confirmReview(sheet, "Send 2 BNB")
			expect(api.post).toHaveBeenCalledWith("/api/send", {
				chain: "EVM-56",
				token: "native",
				amount: "2",
				to: RECIPIENT,
			})
		})

		it("submits what the review showed when the networks change under it", async () => {
			await render(twoNetworks, [8453, 56])
			const sheet = await openSend()
			await enter(amountInput(sheet), "1")
			await enter(recipientInput(sheet), RECIPIENT)
			await click(button(sheet, "Send"))
			expect(sheet.querySelector(".send-review-headline")?.textContent).toContain("on Base")

			// The first network is the default asset's, so a new order would move an unpicked token.
			await render(twoNetworks, [56, 8453])
			await confirmReview(sheet, "Send 1 ETH")
			expect(api.post).toHaveBeenCalledWith("/api/send", expect.objectContaining({ chain: "EVM-8453" }))
		})

		it("sends an unlisted token by contract address, on the network chosen for it", async () => {
			const contract = "0x55d398326f99059fF775485246999027B3197955"
			await render(twoNetworks, [8453, 56])
			const sheet = await openSend()
			await click(button(sheet, "Token not listed?"))
			const trigger = sheet.querySelector<HTMLElement>(".send-asset-trigger")!
			expect(trigger.textContent).toBe("Custom tokenBase")

			// With no balance to list tokens by, the list is the networks.
			await click(trigger)
			expect(optionTexts(sheet)).toEqual(["Custom tokenBase", "Custom tokenBNB Chain"])
			await click(sheet.querySelectorAll<HTMLElement>(".send-asset-option")[1])
			expect(trigger.textContent).toBe("Custom tokenBNB Chain")

			const token = sheet.querySelector<HTMLInputElement>('input[placeholder="0x…"]')!
			await enter(token, "not-an-address")
			await enter(amountInput(sheet), "1000")
			await enter(recipientInput(sheet), RECIPIENT)
			expect(sheet.textContent).toContain("Enter a valid contract address")
			expect(button(sheet, "Send").disabled).toBe(true)

			await enter(token, contract)
			await click(button(sheet, "Send"))
			expect(sheet.querySelector(".send-review")?.textContent).toContain(`Token contract${contract}`)
			await confirmReview(sheet, "Send 1,000 tokens")
			expect(api.post).toHaveBeenCalledWith("/api/send", {
				chain: "EVM-56",
				token: contract,
				amount: "1000",
				to: RECIPIENT,
			})
		})
	})

	it("lets the whole wallet be sent when a vault's floor is above it", async () => {
		const usdc = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
		vi.mocked(api.get).mockResolvedValue(
			config({ "EVM-8453": [...NATIVE_ONLY, { symbol: "USDC", address: usdc }] }),
		)
		// The dashboard's `available` is zero here: the wallet is under the 3,000 the vault asks it to keep.
		await render(
			snapshot([
				{
					chainId: 8453,
					native: { symbol: "ETH", amount: 1 },
					assets: [
						{ address: usdc, symbol: "USDC", wallet: 2500, walletReserve: 3000, available: 0, vaults: [] },
					],
				},
			]),
		)
		const sheet = await openSend()
		await click(sheet.querySelector<HTMLElement>(".send-asset-trigger")!)
		expect(optionTexts(sheet)).toEqual(["USDCBase2,500", "ETHBase1"])
		await click(sheet.querySelector<HTMLElement>(".send-asset-option")!)

		await click(button(sheet, "Max"))
		expect(amountInput(sheet).value).toBe("2,500")
		await enter(recipientInput(sheet), RECIPIENT)
		expect(button(sheet, "Send").disabled).toBe(false)
	})

	it("counts what the largest vault can return, and never fills more than that", async () => {
		const usdc = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
		vi.mocked(api.get).mockResolvedValue(config({ "EVM-8453": [{ symbol: "USDC", address: usdc }] }))
		// The two parts add up to a float a hair under 12463.331201.
		await render(
			snapshot([
				{
					chainId: 8453,
					assets: [
						{
							address: usdc,
							symbol: "USDC",
							wallet: 1990.91807,
							available: 0,
							vaults: [{ available: 10472.413131 }, { available: 500 }],
						},
					],
				},
			]),
		)
		const sheet = await openSend()
		await click(button(sheet, "Max"))
		expect(amountInput(sheet).value).toBe("12,463.3312")
		expect(sheet.querySelector(".send-amount-error")).toBeNull()

		await enter(amountInput(sheet), "12463.34")
		expect(sheet.querySelector(".send-amount-error")?.textContent).toBe("More than the 12,463.3312 USDC available")
		await enter(recipientInput(sheet), RECIPIENT)
		expect(button(sheet, "Send").disabled).toBe(true)
	})

	it("reads a comma decimal as one, where dropping it would send ten times as much", async () => {
		await render()
		const sheet = await openSend()
		await enter(amountInput(sheet), "1,5")
		expect(amountInput(sheet).value).toBe("1.5")
		await enter(recipientInput(sheet), RECIPIENT)
		await click(button(sheet, "Send"))
		await confirmReview(sheet, "Send 1.5 ETH")
		expect(api.post).toHaveBeenCalledWith("/api/send", expect.objectContaining({ amount: "1.5" }))
	})

	it("opens on an empty form after the dialog is closed", async () => {
		await render()
		const sheet = await openSend()
		await enter(amountInput(sheet), "1234")
		await enter(recipientInput(sheet), RECIPIENT)
		await click(button(sheet, "Cancel"))

		const reopened = await openSend()
		expect(amountInput(reopened).value).toBe("")
		expect(recipientInput(reopened).value).toBe("")
	})

	it("closes the token list on Escape, and leaves the dialog open", async () => {
		await render(snapshot([{ chainId: 8453, native: { symbol: "ETH", amount: 1 }, assets: [] }]))
		const sheet = await openSend()
		const trigger = sheet.querySelector<HTMLElement>(".send-asset-trigger")!
		expect(trigger.getAttribute("aria-label")).toBe("Token to send: ETH on Base, balance 1")
		await click(trigger)
		expect(sheet.querySelector(".send-asset-list")).not.toBeNull()

		await act(async () => {
			trigger.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }))
		})
		expect(sheet.querySelector(".send-asset-list")).toBeNull()
		expect(document.querySelector('[role="dialog"]')).not.toBeNull()
	})
})
