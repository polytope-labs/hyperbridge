// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { api } from "../api"
import { DesktopAccess } from "./DesktopAccess"

vi.mock("../lib/runtime", () => ({ isNativeDesktopProtocol: () => true }))

let root: Root
let element: HTMLDivElement
const locked = {
	mode: "unlock",
	passkeyAvailable: false,
	passkeyEnabled: true,
	passwordEnabled: false,
	recoveryEnabled: true,
	secureDeviceStorageAvailable: false,
	needsRestart: false,
}

beforeEach(() => {
	vi.useFakeTimers()
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	element = document.createElement("div")
	root = createRoot(element)
})
afterEach(async () => {
	await act(async () => root.unmount())
	vi.restoreAllMocks()
	vi.useRealTimers()
})

it("offers a passkey again when a slow OS storage check completes, then stops polling", async () => {
	const get = vi
		.spyOn(api, "get")
		.mockResolvedValueOnce(locked)
		.mockResolvedValue({
			...locked,
			secureDeviceStorageAvailable: true,
			passkeyAvailable: true,
		})
	await act(async () => root.render(<DesktopAccess>Dashboard</DesktopAccess>))
	expect(element.textContent).toContain("Recover access")
	await act(async () => {
		await vi.advanceTimersByTimeAsync(2_000)
	})
	expect(element.textContent).toContain("Unlock with passkey")
	await act(async () => {
		await vi.advanceTimersByTimeAsync(10_000)
	})
	expect(get).toHaveBeenCalledTimes(2)
})

it("keeps storage refreshes sequential and cancels an in-flight refresh on unmount", async () => {
	let complete!: (value: typeof locked) => void
	const get = vi
		.spyOn(api, "get")
		.mockResolvedValueOnce(locked)
		.mockImplementation(
			() =>
				new Promise((resolve) => {
					complete = resolve
				}),
		)
	await act(async () => root.render(<DesktopAccess>Dashboard</DesktopAccess>))
	await act(async () => {
		await vi.advanceTimersByTimeAsync(10_000)
	})
	expect(get).toHaveBeenCalledTimes(2)
	const signal = get.mock.calls[1][1]?.signal
	expect(signal?.aborted).toBe(false)
	await act(async () => root.unmount())
	expect(signal?.aborted).toBe(true)
	await act(async () => {
		complete(locked)
	})
	expect(vi.getTimerCount()).toBe(0)
})

it("preserves the password form when storage becomes available during account creation", async () => {
	vi.spyOn(api, "get")
		.mockResolvedValueOnce({ ...locked, mode: "create" })
		.mockResolvedValue({
			...locked,
			mode: "create",
			secureDeviceStorageAvailable: true,
			passkeyAvailable: true,
		})
	await act(async () => root.render(<DesktopAccess>Dashboard</DesktopAccess>))
	const password = element.querySelector("#desktop-password")
	expect(password).not.toBeNull()
	await act(async () => {
		await vi.advanceTimersByTimeAsync(2_000)
	})
	expect(element.querySelector("#desktop-password")).toBe(password)
	expect(element.textContent).toContain("Use a passkey instead")
})

it("clears the scheduled storage refresh when the login screen unmounts", async () => {
	const get = vi.spyOn(api, "get").mockResolvedValue(locked)
	await act(async () => root.render(<DesktopAccess>Dashboard</DesktopAccess>))
	await act(async () => root.unmount())
	await vi.advanceTimersByTimeAsync(10_000)
	expect(get).toHaveBeenCalledOnce()
	expect(vi.getTimerCount()).toBe(0)
})
