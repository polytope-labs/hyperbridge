import { describe, expect, it } from "vitest"
import { desktopArguments, directElectronArguments } from "../../scripts/e2e/electron-launch"

describe("Electron E2E launch arguments", () => {
	it("builds the shared desktop application arguments", () => {
		expect(desktopArguments("/app/simplex-desktop", "/tmp/simplex-user", { hidden: true })).toEqual([
			"/app/simplex-desktop",
			"--user-data-dir=/tmp/simplex-user",
			"--hidden",
		])
	})

	it("disables the Chromium sandbox only for a direct Linux test launch", () => {
		expect(directElectronArguments("/app/simplex-desktop", "/tmp/simplex-user", "linux")).toEqual([
			"--no-sandbox",
			"/app/simplex-desktop",
			"--user-data-dir=/tmp/simplex-user",
		])
		expect(directElectronArguments("/app/simplex-desktop", "/tmp/simplex-user", "darwin")).toEqual([
			"/app/simplex-desktop",
			"--user-data-dir=/tmp/simplex-user",
		])
		expect(directElectronArguments("/app/simplex-desktop", "C:\\Simplex", "win32")).toEqual([
			"/app/simplex-desktop",
			"--user-data-dir=C:\\Simplex",
		])
	})
})
