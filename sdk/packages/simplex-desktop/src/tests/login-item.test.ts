import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"
import { afterEach, describe, expect, it, vi } from "vitest"
import {
	HIDDEN_LAUNCH_ARGUMENT,
	latestLogPath,
	linuxAutostartEntry,
	linuxAutostartPath,
	LoginItemController,
} from "../login-item"

const directories: string[] = []

afterEach(() => {
	for (const directory of directories.splice(0)) rmSync(directory, { recursive: true, force: true })
})

function temporaryDirectory(): string {
	const directory = mkdtempSync(join(tmpdir(), "simplex-login-item-"))
	directories.push(directory)
	return directory
}

function appMock(openAtLogin = false, wasOpenedAtLogin = false) {
	return {
		getLoginItemSettings: vi.fn(() => ({ openAtLogin, wasOpenedAtLogin })),
		setLoginItemSettings: vi.fn(),
	}
}

describe("launch at login", () => {
	it("uses Electron login items for installed macOS and Windows apps", () => {
		const macApp = appMock(true, true)
		const mac = new LoginItemController({
			app: macApp,
			platform: "darwin",
			isPackaged: true,
			executable: "/Applications/Simplex.app/Contents/MacOS/Simplex",
		})
		expect(mac.isEnabled()).toBe(true)
		expect(mac.wasOpenedAtLogin()).toBe(true)
		mac.setEnabled(false)
		expect(macApp.setLoginItemSettings).toHaveBeenCalledWith({ openAtLogin: false })

		const windowsApp = appMock()
		const windows = new LoginItemController({
			app: windowsApp,
			platform: "win32",
			isPackaged: true,
			executable: "C:\\Program Files\\Simplex\\Simplex.exe",
		})
		windows.setEnabled(true)
		expect(windowsApp.setLoginItemSettings).toHaveBeenCalledWith({
			openAtLogin: true,
			path: "C:\\Program Files\\Simplex\\Simplex.exe",
			args: [HIDDEN_LAUNCH_ARGUMENT],
		})
	})

	it("does not register a development Electron binary", () => {
		const controller = new LoginItemController({
			app: appMock(),
			platform: "darwin",
			isPackaged: false,
			executable: "/tmp/Electron",
		})
		expect(controller.supported).toBe(false)
		expect(() => controller.setEnabled(true)).toThrow(/installed Simplex app/)
	})

	it("recognizes explicit hidden launches on every platform", () => {
		const controller = new LoginItemController({
			app: appMock(),
			platform: "linux",
			isPackaged: false,
			executable: "/tmp/simplex",
			argv: ["simplex", HIDDEN_LAUNCH_ARGUMENT],
		})
		expect(controller.wasOpenedAtLogin()).toBe(true)
	})

	it("writes and removes only its own Linux XDG autostart entry", () => {
		const configHome = temporaryDirectory()
		const executable = '/opt/Simplex App/simplex"desktop'
		const controller = new LoginItemController({
			app: appMock(),
			platform: "linux",
			isPackaged: true,
			executable,
			linuxConfigHome: configHome,
		})
		controller.setEnabled(true)
		const path = linuxAutostartPath(configHome)
		expect(readFileSync(path, "utf8")).toBe(linuxAutostartEntry(executable))
		if (process.platform !== "win32") expect(statSync(path).mode & 0o777).toBe(0o600)
		expect(controller.isEnabled()).toBe(true)
		controller.setEnabled(false)
		expect(existsSync(path)).toBe(false)

		mkdirSync(dirname(path), { recursive: true })
		writeFileSync(path, "[Desktop Entry]\nName=Someone else\n")
		expect(() => controller.setEnabled(false)).toThrow(/unmanaged autostart file/)
		expect(() => controller.setEnabled(true)).toThrow(/unmanaged autostart file/)
	})

	it("finds the newest rotated log", () => {
		const dataDir = temporaryDirectory()
		const logsDir = join(dataDir, "logs")
		mkdirSync(logsDir)
		writeFileSync(join(logsDir, "simplex-2026-09-14T10-00-00-000.log"), "old")
		writeFileSync(join(logsDir, "simplex-2026-09-15T10-00-00-000.log"), "new")
		writeFileSync(join(logsDir, "other.log"), "ignored")
		expect(latestLogPath(dataDir)).toBe(join(logsDir, "simplex-2026-09-15T10-00-00-000.log"))
	})
})
