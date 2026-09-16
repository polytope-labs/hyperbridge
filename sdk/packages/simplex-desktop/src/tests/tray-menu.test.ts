import type { MenuItemConstructorOptions } from "electron"
import { describe, expect, it, vi } from "vitest"
import { buildApplicationMenuTemplate, buildTrayMenuTemplate, solverStatusLabel } from "../tray-menu"

function actions() {
	return {
		showWindow: vi.fn(),
		togglePause: vi.fn(),
		stopSolver: vi.fn(),
		restartSolver: vi.fn(),
		restartBundledSolver: vi.fn(),
		toggleLoginItem: vi.fn(),
		showAbout: vi.fn(),
		checkForUpdates: vi.fn(),
		setUpdateChannel: vi.fn(),
		openDataDirectory: vi.fn(),
		openLog: vi.fn(),
		quitApp: vi.fn(),
		stopAndQuit: vi.fn(),
	}
}

function item(template: MenuItemConstructorOptions[], id: string): MenuItemConstructorOptions {
	const found = template.find((candidate) => candidate.id === id)
	if (!found) throw new Error(`Missing menu item ${id}`)
	return found
}

const runningModel = {
	status: { state: "running" } as const,
	loginItemSupported: true,
	loginItemEnabled: true,
	logAvailable: true,
	sleepPreventionActive: true,
	updatesEnabled: true,
	update: { state: "idle", channel: "stable" } as const,
	versionSkew: false,
}

describe("native desktop menus", () => {
	it("puts every Linux-safe action in the tray context menu", () => {
		const template = buildTrayMenuTemplate(runningModel, actions())
		for (const id of [
			"show-simplex",
			"toggle-pause",
			"stop-solver",
			"restart-solver",
			"launch-at-login",
			"about-simplex",
			"check-for-updates",
			"open-data-directory",
			"open-current-log",
			"quit-simplex",
			"stop-and-quit",
		]) {
			expect(item(template, id)).toBeDefined()
		}
	})

	it("clearly distinguishes app-only quit from stop-and-quit", () => {
		const template = buildTrayMenuTemplate(runningModel, actions())
		expect(item(template, "quit-simplex").label).toBe("Quit Simplex (solver keeps filling)")
		expect(item(template, "stop-and-quit").label).toBe("Stop solver and quit")
		expect(item(template, "quit-simplex").accelerator).toBe("CmdOrCtrl+Q")

		const setup = buildTrayMenuTemplate({ ...runningModel, status: { state: "setup" } }, actions())
		expect(item(setup, "quit-simplex").label).toBe("Quit Simplex")
		expect(item(setup, "stop-solver").enabled).toBe(true)
		expect(item(setup, "stop-and-quit").enabled).toBe(true)
	})

	it("reflects running, paused, and stopped control states", () => {
		const callbacks = actions()
		const running = buildTrayMenuTemplate(runningModel, callbacks)
		expect(item(running, "solver-status").label).toBe("Solver: Running")
		expect(item(running, "sleep-prevention").label).toBe("Sleep prevention: On")
		expect(item(running, "toggle-pause").label).toBe("Pause filling")
		expect(item(running, "stop-solver").enabled).toBe(true)

		const paused = buildTrayMenuTemplate(
			{ ...runningModel, status: { state: "paused" }, sleepPreventionActive: false },
			callbacks,
		)
		expect(item(paused, "toggle-pause").label).toBe("Resume filling")
		expect(item(paused, "sleep-prevention").label).toBe("Sleep prevention: Off")

		const stopped = buildTrayMenuTemplate({ ...runningModel, status: { state: "stopped" } }, callbacks)
		expect(item(stopped, "restart-solver").enabled).toBe(true)
		expect(item(stopped, "stop-solver").enabled).toBe(false)

		const stopping = buildTrayMenuTemplate({ ...runningModel, status: { state: "stopping" } }, callbacks)
		expect(item(stopping, "restart-solver").enabled).toBe(false)
		expect(item(stopping, "stop-solver").enabled).toBe(false)
	})

	it("exposes the required commands in the application menu", () => {
		const template = buildApplicationMenuTemplate(runningModel, actions(), "darwin")
		const root = template[0]
		const submenu = root.submenu as MenuItemConstructorOptions[]
		for (const id of [
			"about-simplex",
			"check-for-updates",
			"open-data-directory",
			"open-current-log",
			"quit-simplex",
		]) {
			expect(item(submenu, id)).toBeDefined()
		}
		expect(template.some((entry) => entry.role === "editMenu")).toBe(true)
		expect(template.some((entry) => entry.role === "windowMenu")).toBe(true)
		expect(item(submenu, "check-for-updates").visible).toBe(true)
		expect(item(submenu, "update-channel").visible).toBe(true)
		expect(submenu.some((entry) => entry.role === "hide")).toBe(true)
		const window = template.find((entry) => entry.role === "windowMenu")
		expect((window?.submenu as MenuItemConstructorOptions[]).some((entry) => entry.role === "close")).toBe(true)
	})

	it("surfaces staged updates and a version-skew restart", () => {
		const template = buildTrayMenuTemplate(
			{
				...runningModel,
				versionSkew: true,
				update: { state: "waiting-for-idle", channel: "beta", targetVersion: "0.17.0-beta.1" },
			},
			actions(),
		)
		expect(item(template, "check-for-updates").label).toBe("Update Ready — Waiting for Idle")
		expect(item(template, "restart-bundled-solver").visible).toBe(true)
		expect(item(template, "restart-bundled-solver").enabled).toBe(true)

		const stopping = buildTrayMenuTemplate(
			{ ...runningModel, update: { state: "stopping-solver", channel: "stable", targetVersion: "0.17.0" } },
			actions(),
		)
		expect(item(stopping, "update-channel").enabled).toBe(false)
	})

	it("labels every solver status", () => {
		for (const [state, label] of [
			["starting", "Solver: Starting…"],
			["setup", "Solver: Setup required"],
			["running", "Solver: Running"],
			["paused", "Solver: Paused"],
			["stopping", "Solver: Stopping…"],
			["stopped", "Solver: Stopped"],
			["unreachable", "Solver: Unreachable"],
		] as const) {
			const status = state === "unreachable" ? { state, detail: "test" } : { state }
			expect(solverStatusLabel(status)).toBe(label)
		}
	})
})
