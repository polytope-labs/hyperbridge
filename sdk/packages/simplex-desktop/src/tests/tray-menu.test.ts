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
		installUpdate: vi.fn(),
		setUpdateChannel: vi.fn(),
		openDataDirectory: vi.fn(),
		openLog: vi.fn(),
		resetConfiguration: vi.fn(),
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

	it("zooms from the View menu without exposing reload or devtools", () => {
		const template = buildApplicationMenuTemplate(runningModel, actions(), "darwin")
		const view = template.find((entry) => entry.label === "View")
		const roles = (view?.submenu as MenuItemConstructorOptions[]).map((entry) => entry.role).filter(Boolean)
		expect(roles).toEqual(expect.arrayContaining(["resetZoom", "zoomIn", "zoomOut"]))
		expect(roles).not.toContain("reload")
		expect(roles).not.toContain("toggleDevTools")
		const zoomIn = (view?.submenu as MenuItemConstructorOptions[]).filter((entry) => entry.role === "zoomIn")
		expect(zoomIn.map((entry) => entry.accelerator)).toEqual(["CmdOrCtrl+=", "CmdOrCtrl+Plus"])
	})

	it("turns the update check into the install once an update is downloaded", () => {
		const callbacks = actions()
		const ready = { state: "ready", channel: "beta", targetVersion: "0.17.0-beta.1" } as const
		const template = buildTrayMenuTemplate({ ...runningModel, versionSkew: true, update: ready }, callbacks)
		expect(item(template, "check-for-updates").label).toBe("Install Update 0.17.0-beta.1")
		expect(item(template, "check-for-updates").enabled).toBe(true)
		;(item(template, "check-for-updates").click as () => void)()
		expect(callbacks.installUpdate).toHaveBeenCalledOnce()
		expect(callbacks.checkForUpdates).not.toHaveBeenCalled()
		expect(item(template, "restart-bundled-solver").visible).toBe(true)
		expect(item(template, "restart-bundled-solver").enabled).toBe(true)

		// The install stops the solver first, so it is offered only while there is one to stop.
		const stopped = buildTrayMenuTemplate(
			{ ...runningModel, status: { state: "stopped" }, update: ready },
			callbacks,
		)
		expect(item(stopped, "check-for-updates").enabled).toBe(false)

		const resetting = buildTrayMenuTemplate({ ...runningModel, update: ready, resetting: true }, callbacks)
		expect(item(resetting, "check-for-updates").enabled).toBe(false)

		const idle = buildTrayMenuTemplate(runningModel, callbacks)
		expect(item(idle, "check-for-updates").label).toBe("Check for Updates…")
		;(item(idle, "check-for-updates").click as () => void)()
		expect(callbacks.checkForUpdates).toHaveBeenCalledOnce()

		const stopping = buildTrayMenuTemplate(
			{ ...runningModel, update: { state: "stopping-solver", channel: "stable", targetVersion: "0.17.0" } },
			actions(),
		)
		expect(item(stopping, "update-channel").enabled).toBe(false)
		expect(item(stopping, "check-for-updates").enabled).toBe(false)
	})

	it("keeps the solver from being restarted under an installing update", () => {
		// On macOS the app stays open, with the solver stopped, while the installer fetches the update.
		for (const state of ["stopping-solver", "installing"] as const) {
			const model = {
				...runningModel,
				status: { state: "stopped" } as const,
				update: { state, channel: "stable", targetVersion: "0.17.0" } as const,
			}
			const template = buildTrayMenuTemplate(model, actions())
			expect(item(template, "restart-solver").enabled).toBe(false)
			expect(item(template, "update-channel").enabled).toBe(false)
			expect(item(template, "check-for-updates").enabled).toBe(false)
		}
		const failed = {
			...runningModel,
			status: { state: "stopped" } as const,
			update: { state: "error", channel: "stable", targetVersion: "0.17.0" } as const,
		}
		expect(item(buildTrayMenuTemplate(failed, actions()), "restart-solver").enabled).toBe(true)
	})

	it("disables privileged menu actions until this desktop session is unlocked", () => {
		const template = buildTrayMenuTemplate({ ...runningModel, locked: true, versionSkew: true }, actions())
		for (const id of [
			"toggle-pause",
			"stop-solver",
			"restart-solver",
			"restart-bundled-solver",
			"stop-and-quit",
			"open-current-log",
			"reset-configuration",
			"check-for-updates",
			"update-channel",
		]) {
			expect(item(template, id).enabled).toBe(false)
		}
		expect(item(template, "solver-status").label).toBe("Simplex locked")
		expect(item(template, "show-simplex").enabled).not.toBe(false)
	})

	it("offers the reset from the tray and the application menu", () => {
		const callbacks = actions()
		const tray = item(buildTrayMenuTemplate(runningModel, callbacks), "reset-configuration")
		expect(tray.label).toBe("Reset Configuration…")
		expect(tray.enabled).toBe(true)
		;(tray.click as () => void)()
		expect(callbacks.resetConfiguration).toHaveBeenCalledOnce()

		const application = buildApplicationMenuTemplate(runningModel, callbacks, "darwin")[0]
			.submenu as MenuItemConstructorOptions[]
		expect(item(application, "reset-configuration").enabled).toBe(true)
	})

	it("withholds the reset while locked, and while an update stops the solver or installs", () => {
		const callbacks = actions()
		const blocked = [
			{ ...runningModel, locked: true },
			{ ...runningModel, update: { state: "stopping-solver", channel: "stable", targetVersion: "0.19.0" } as const },
			{ ...runningModel, update: { state: "installing", channel: "stable", targetVersion: "0.19.0" } as const },
		]
		for (const model of blocked) {
			const application = buildApplicationMenuTemplate(model, callbacks, "darwin")[0]
				.submenu as MenuItemConstructorOptions[]
			expect(item(buildTrayMenuTemplate(model, callbacks), "reset-configuration").enabled).toBe(false)
			expect(item(application, "reset-configuration").enabled).toBe(false)
		}
		// A downloaded update that nobody has asked to install has not started stopping the solver.
		const ready = {
			...runningModel,
			update: { state: "ready", channel: "stable", targetVersion: "0.19.0" } as const,
		}
		expect(item(buildTrayMenuTemplate(ready, callbacks), "reset-configuration").enabled).toBe(true)
	})

	it("shows a reset in progress, and keeps the solver from being restarted under it", () => {
		const callbacks = actions()
		const model = { ...runningModel, status: { state: "stopped" } as const, resetting: true }
		const tray = buildTrayMenuTemplate(model, callbacks)
		expect(item(tray, "reset-configuration").label).toBe("Resetting Configuration…")
		expect(item(tray, "reset-configuration").enabled).toBe(false)
		expect(item(tray, "restart-solver").enabled).toBe(false)
		expect(item(buildTrayMenuTemplate({ ...model, resetting: false }, callbacks), "restart-solver").enabled).toBe(true)
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
