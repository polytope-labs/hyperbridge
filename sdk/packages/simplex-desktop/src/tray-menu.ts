import type { MenuItemConstructorOptions } from "electron"
import type { SolverStatus } from "./solver-supervisor"

export interface DesktopMenuActions {
	showWindow: () => void | Promise<void>
	togglePause: () => void | Promise<void>
	stopSolver: () => void | Promise<void>
	restartSolver: () => void | Promise<void>
	toggleLoginItem: () => void | Promise<void>
	showAbout: () => void
	checkForUpdates: () => void | Promise<void>
	openDataDirectory: () => void | Promise<void>
	openLog: () => void | Promise<void>
	quitApp: () => void
	stopAndQuit: () => void | Promise<void>
}

export interface DesktopMenuModel {
	status: SolverStatus
	loginItemSupported: boolean
	loginItemEnabled: boolean
	logAvailable: boolean
	sleepPreventionActive: boolean
}

function run(action: () => void | Promise<void>): () => void {
	return () => {
		try {
			void Promise.resolve(action()).catch((error) => console.error("Simplex menu action failed", error))
		} catch (error) {
			console.error("Simplex menu action failed", error)
		}
	}
}

export function solverStatusLabel(status: SolverStatus): string {
	switch (status.state) {
		case "starting":
			return "Solver: Starting…"
		case "setup":
			return "Solver: Setup required"
		case "running":
			return "Solver: Running"
		case "paused":
			return "Solver: Paused"
		case "stopped":
			return "Solver: Stopped"
		case "unreachable":
			return "Solver: Unreachable"
	}
}

function operationItems(model: DesktopMenuModel, actions: DesktopMenuActions): MenuItemConstructorOptions[] {
	const canPause = model.status.state === "running" || model.status.state === "paused"
	const canStop = canPause
	const canRestart = model.status.state === "stopped" || model.status.state === "unreachable"
	return [
		{ id: "solver-status", label: solverStatusLabel(model.status), enabled: false },
		{
			id: "sleep-prevention",
			label: `Sleep prevention: ${model.sleepPreventionActive ? "On" : "Off"}`,
			enabled: false,
		},
		{ type: "separator" },
		{ id: "show-simplex", label: "Show Simplex", click: run(actions.showWindow) },
		{
			id: "toggle-pause",
			label: model.status.state === "paused" ? "Resume filling" : "Pause filling",
			enabled: canPause,
			click: run(actions.togglePause),
		},
		{ id: "restart-solver", label: "Restart solver", enabled: canRestart, click: run(actions.restartSolver) },
		{ id: "stop-solver", label: "Stop solver", enabled: canStop, click: run(actions.stopSolver) },
	]
}

export function buildTrayMenuTemplate(
	model: DesktopMenuModel,
	actions: DesktopMenuActions,
): MenuItemConstructorOptions[] {
	return [
		...operationItems(model, actions),
		{ type: "separator" },
		{
			id: "launch-at-login",
			label: model.loginItemSupported ? "Launch Simplex at login" : "Launch at login (installed app only)",
			type: "checkbox",
			checked: model.loginItemEnabled,
			enabled: model.loginItemSupported,
			click: run(actions.toggleLoginItem),
		},
		{ type: "separator" },
		{ id: "about-simplex", label: "About Simplex", click: actions.showAbout },
		{ id: "check-for-updates", label: "Check for Updates…", click: run(actions.checkForUpdates) },
		{ id: "open-data-directory", label: "Open Data Directory", click: run(actions.openDataDirectory) },
		{ id: "open-current-log", label: "Open Current Log", enabled: model.logAvailable, click: run(actions.openLog) },
		{ type: "separator" },
		{ id: "quit-simplex", label: "Quit Simplex (solver keeps filling)", click: actions.quitApp },
		{
			id: "stop-and-quit",
			label: "Stop solver and quit",
			enabled: canStopSolver(model.status),
			click: run(actions.stopAndQuit),
		},
	]
}

function canStopSolver(status: SolverStatus): boolean {
	return status.state === "running" || status.state === "paused"
}

export function buildApplicationMenuTemplate(
	model: DesktopMenuModel,
	actions: DesktopMenuActions,
): MenuItemConstructorOptions[] {
	return [
		{
			label: "Simplex",
			submenu: [
				{ id: "about-simplex", label: "About Simplex", click: actions.showAbout },
				{ id: "check-for-updates", label: "Check for Updates…", click: run(actions.checkForUpdates) },
				{ type: "separator" },
				...operationItems(model, actions),
				{ type: "separator" },
				{ id: "open-data-directory", label: "Open Data Directory", click: run(actions.openDataDirectory) },
				{
					id: "open-current-log",
					label: "Open Current Log",
					enabled: model.logAvailable,
					click: run(actions.openLog),
				},
				{ type: "separator" },
				{ id: "quit-simplex", label: "Quit Simplex (solver keeps filling)", click: actions.quitApp },
				{
					id: "stop-and-quit",
					label: "Stop solver and quit",
					enabled: canStopSolver(model.status),
					click: run(actions.stopAndQuit),
				},
			],
		},
	]
}
