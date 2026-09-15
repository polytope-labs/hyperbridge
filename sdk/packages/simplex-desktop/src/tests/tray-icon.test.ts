import { join } from "node:path"
import { describe, expect, it } from "vitest"
import { trayIconPath } from "../tray-icon"

describe("tray icon", () => {
	it("selects a state-specific PWA logo variant", () => {
		expect(trayIconPath("/desktop/tray", { state: "running" }, "linux")).toBe(join("/desktop/tray", "running.png"))
		expect(trayIconPath("/desktop/tray", { state: "stopped" }, "win32")).toBe(join("/desktop/tray", "stopped.png"))
	})

	it("selects transparent macOS template variants", () => {
		expect(trayIconPath("/desktop/tray", { state: "paused" }, "darwin")).toBe(
			join("/desktop/tray", "pausedTemplate.png"),
		)
	})

	it("covers setup and unreachable states", () => {
		expect(trayIconPath("/desktop/tray", { state: "setup" }, "linux")).toContain("setup.png")
		expect(trayIconPath("/desktop/tray", { state: "unreachable" }, "darwin")).toContain("unreachableTemplate.png")
	})
})
