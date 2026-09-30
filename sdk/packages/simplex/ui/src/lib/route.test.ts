import { describe, expect, it } from "vitest"
import { TAB_PATHS, tabFromPath } from "./route"

describe("operator tab paths", () => {
	it("maps every page to its own path and back", () => {
		for (const [tab, path] of Object.entries(TAB_PATHS)) expect(tabFromPath(path)).toBe(tab)
	})

	it("opens History from the path it had as Orders, which delivered notifications still use", () => {
		expect(tabFromPath("/orders")).toBe("history")
		expect(tabFromPath("/orders/")).toBe("history")
	})

	it("lands a limit-orders link on the overview, where the limit orders now live", () => {
		expect(tabFromPath("/limit-orders")).toBe("overview")
	})
})
