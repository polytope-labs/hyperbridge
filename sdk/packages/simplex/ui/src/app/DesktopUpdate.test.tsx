// @vitest-environment jsdom
import { act } from "react"
import { createRoot, type Root } from "react-dom/client"
import { toast } from "sonner"
import { afterEach, beforeEach, expect, it, vi } from "vitest"
import { api } from "../api"
import { DesktopUpdateProvider, UpdateButton, UpdatingScreen, useDesktopUpdate } from "./DesktopUpdate"

const runtime = vi.hoisted(() => ({ native: true }))
vi.mock("../lib/runtime", () => ({ isNativeDesktopProtocol: () => runtime.native }))

let root: Root
let element: HTMLDivElement
const ready = { state: "ready", channel: "stable", targetVersion: "0.21.0" }

/** What the app renders: the button in the sidebar, or the updating screen in place of the dashboard. */
function Screen() {
	return useDesktopUpdate().installing ? <UpdatingScreen /> : <UpdateButton />
}

async function render() {
	await act(async () =>
		root.render(
			<DesktopUpdateProvider>
				<Screen />
			</DesktopUpdateProvider>,
		),
	)
}

beforeEach(() => {
	vi.useFakeTimers()
	;(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true
	runtime.native = true
	element = document.createElement("div")
	root = createRoot(element)
})
afterEach(async () => {
	await act(async () => root.unmount())
	vi.restoreAllMocks()
	vi.useRealTimers()
})

it("shows the button only once an update has been downloaded", async () => {
	const get = vi
		.spyOn(api, "get")
		.mockResolvedValueOnce({ state: "downloading", channel: "stable", targetVersion: "0.21.0", progress: 40 })
		.mockResolvedValue(ready)
	await render()
	expect(get).toHaveBeenCalledWith("/api/desktop/update")
	expect(element.querySelector("button")).toBeNull()
	await act(async () => {
		await vi.advanceTimersByTimeAsync(15_000)
	})
	expect(element.querySelector("button")?.textContent).toContain("Update to 0.21.0")
})

it("installs the update when the button is clicked, and replaces the dashboard while it does", async () => {
	const get = vi.spyOn(api, "get").mockResolvedValue(ready)
	let installed!: () => void
	const post = vi.spyOn(api, "post").mockReturnValue(
		new Promise<void>((resolve) => {
			installed = resolve
		}),
	)
	await render()
	await act(async () => element.querySelector("button")?.click())
	expect(post).toHaveBeenCalledWith("/api/desktop/update")
	expect(element.querySelector("button")).toBeNull()
	expect(element.textContent).toContain("Updating to 0.21.0")
	// The shell is asked again every two seconds while the install runs.
	const polls = get.mock.calls.length
	await act(async () => {
		await vi.advanceTimersByTimeAsync(2_000)
	})
	expect(get).toHaveBeenCalledTimes(polls + 1)

	// The shell reports the install until the app quits, so the dashboard stays replaced.
	get.mockResolvedValue({ state: "installing", channel: "stable", targetVersion: "0.21.0" })
	await act(async () => installed())
	expect(element.textContent).toContain("Updating to 0.21.0")
})

it("brings the button back and says why when the install is refused", async () => {
	vi.spyOn(api, "get").mockResolvedValue(ready)
	vi.spyOn(api, "post").mockRejectedValue(new Error("The solver is stopping. Try again in a moment."))
	const error = vi.spyOn(toast, "error").mockReturnValue("")
	await render()
	await act(async () => element.querySelector("button")?.click())
	expect(error).toHaveBeenCalledWith("The update was not installed", {
		description: "The solver is stopping. Try again in a moment.",
	})
	expect(element.querySelector("button")?.textContent).toContain("Update to 0.21.0")
})

it("follows an install started from the menu, and says why when it fails", async () => {
	const get = vi.spyOn(api, "get").mockResolvedValue(ready)
	const post = vi.spyOn(api, "post")
	const error = vi.spyOn(toast, "error").mockReturnValue("")
	await render()
	get.mockResolvedValue({ state: "stopping-solver", channel: "stable", targetVersion: "0.21.0" })
	await act(async () => {
		await vi.advanceTimersByTimeAsync(15_000)
	})
	expect(element.textContent).toContain("Updating to 0.21.0")
	get.mockResolvedValue({ state: "installing", channel: "stable", targetVersion: "0.21.0" })
	await act(async () => {
		await vi.advanceTimersByTimeAsync(2_000)
	})
	expect(element.textContent).toContain("Updating to 0.21.0")

	const detail = "Update 0.21.0 could not be installed: code signature did not validate"
	get.mockResolvedValue({ state: "error", channel: "stable", targetVersion: "0.21.0", detail })
	await act(async () => {
		await vi.advanceTimersByTimeAsync(4_000)
	})
	expect(element.textContent).not.toContain("Updating")
	expect(error).toHaveBeenCalledExactlyOnceWith("The update was not installed", { description: detail })
	expect(post).not.toHaveBeenCalled()
})

it("says why when the installer fails after its own request was answered", async () => {
	const get = vi.spyOn(api, "get").mockResolvedValue(ready)
	vi.spyOn(api, "post").mockResolvedValue({ ok: true })
	const error = vi.spyOn(toast, "error").mockReturnValue("")
	await render()
	// The failure lands before the first read that follows the answer.
	const detail = "Update 0.21.0 could not be installed: code signature did not validate"
	get.mockResolvedValue({ state: "error", channel: "stable", targetVersion: "0.21.0", detail })
	await act(async () => element.querySelector("button")?.click())
	expect(error).toHaveBeenCalledExactlyOnceWith("The update was not installed", { description: detail })
	expect(element.textContent).not.toContain("Updating")
	await act(async () => {
		await vi.advanceTimersByTimeAsync(30_000)
	})
	expect(error).toHaveBeenCalledOnce()
})

it("reports a failed install once when its own request carries the failure", async () => {
	const get = vi.spyOn(api, "get").mockResolvedValue(ready)
	let refuse!: (cause: Error) => void
	vi.spyOn(api, "post").mockReturnValue(
		new Promise<void>((_resolve, reject) => {
			refuse = reject
		}),
	)
	const error = vi.spyOn(toast, "error").mockReturnValue("")
	await render()
	await act(async () => element.querySelector("button")?.click())
	const detail = "Update 0.21.0 could not be installed: installer unavailable"
	get.mockResolvedValue({ state: "installing", channel: "stable", targetVersion: "0.21.0" })
	await act(async () => {
		await vi.advanceTimersByTimeAsync(2_000)
	})
	get.mockResolvedValue({ state: "error", channel: "stable", targetVersion: "0.21.0", detail })
	await act(async () => {
		await vi.advanceTimersByTimeAsync(2_000)
	})
	// The solver is still being restarted, and the request has not come back yet.
	expect(error).not.toHaveBeenCalled()
	expect(element.textContent).toContain("Updating")
	await act(async () => refuse(new Error(detail)))
	expect(error).toHaveBeenCalledExactlyOnceWith("The update was not installed", { description: detail })
	expect(element.textContent).not.toContain("Updating")
})

it("asks for nothing outside the desktop app", async () => {
	runtime.native = false
	const get = vi.spyOn(api, "get").mockResolvedValue(ready)
	await render()
	await act(async () => {
		await vi.advanceTimersByTimeAsync(120_000)
	})
	expect(get).not.toHaveBeenCalled()
	expect(element.querySelector("button")).toBeNull()
})
