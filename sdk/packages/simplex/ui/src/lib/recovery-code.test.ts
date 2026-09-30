import { describe, expect, it } from "vitest"
import { formatRecoveryCode } from "./recovery-code"

const code = "0123ABCD-4567EF01-89ABCDEF-01234567-89ABCDEF-FEDCBA98-76543210-0F1E2D3C"

describe("recovery code input", () => {
	it("groups typed or lowercase digits into blocks of eight", () => {
		expect(formatRecoveryCode("0123abcd4567")).toBe("0123ABCD-4567")
		expect(formatRecoveryCode(code.toLowerCase().replaceAll("-", " "))).toBe(code)
	})

	it("ignores hex letters in a label pasted with the code", () => {
		expect(formatRecoveryCode(`Recovery code: ${code}`)).toBe(code)
		expect(formatRecoveryCode(`Simplex recovery code (saved Dec 2026)\n${code}\n`)).toBe(code)
	})

	it("caps input at a full code", () => {
		expect(formatRecoveryCode(`${code}-FFFF`).replaceAll("-", "")).toHaveLength(64)
	})
})
