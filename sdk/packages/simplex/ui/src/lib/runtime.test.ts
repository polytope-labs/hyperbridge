import { describe, expect, it } from "vitest"
import { isNativeDesktopProtocol } from "./runtime"

describe("runtime surface detection", () => {
	it("identifies the native Electron renderer", () => {
		expect(isNativeDesktopProtocol("simplex:")).toBe(true)
		expect(isNativeDesktopProtocol("http:")).toBe(false)
		expect(isNativeDesktopProtocol("https:")).toBe(false)
	})
})
