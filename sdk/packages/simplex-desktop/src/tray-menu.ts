import type { MenuItemConstructorOptions } from "electron"
import type { SolverStatus } from "./solver-supervisor"
import { updateInstalling, type UpdateStatus } from "./update-coordinator"
import type { UpdateChannel } from "./update-store"

export interface DesktopMenuActions {
	showWindow: () => void | Promise<void>
	togglePause: () => void | Promise<void>
	stopSolver: () => void | Promise<void>
	restartSolver: () => void | Promise<void>
	restartBundledSolver: () => void | Promise<void>
	toggleLoginItem: () => void | Promise<void>
	showAbout: () => void
	checkForUpdates: () => void | Promise<void>
	installUpdate: () => void | Promise<void>
	setUpdateChannel: (channel: UpdateChannel) => void | Promise<void>
	openDataDirectory: () => void | Promise<void>
	openLog: () => void | Promise<void>
	resetConfiguration: () => void | Promise<void>
	quitApp: () => void
	stopAndQuit: () => void | Promise<void>
}

export interface DesktopMenuModel {
	status: SolverStatus
	locked?: boolean
	loginItemSupported: boolean
	loginItemEnabled: boolean
	logAvailable: boolean
	sleepPreventionActive: boolean
	updatesEnabled: boolean
	update: UpdateStatus
	versionSkew: boolean
	/** A confirmed reset is stopping the solver before the app relaunches. */
	resetting?: boolean
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
		case "stopping":
			return "Solver: Stopping…"
		case "stopped":
			return "Solver: Stopped"
		case "unreachable":
			return "Solver: Unreachable"
	}
}

function operationItems(model: DesktopMenuModel, actions: DesktopMenuActions): MenuItemConstructorOptions[] {
	const canPause = !model.locked && (model.status.state === "running" || model.status.state === "paused")
	const canStop = !model.locked && (canPause || model.status.state === "setup")
	// An update that has stopped the solver is about to replace it: a restart would put one back under the installer.
	const canRestart =
		!model.locked &&
		!model.resetting &&
		!updateInstalling(model.update) &&
		(model.status.state === "stopped" || model.status.state === "unreachable")
	return [
		{
			id: "solver-status",
			label: model.locked ? "Simplex locked" : solverStatusLabel(model.status),
			enabled: false,
		},
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
		{
			id: "restart-bundled-solver",
			label: "Restart with bundled solver",
			visible: model.versionSkew,
			enabled: model.versionSkew && canStop && !updateInstalling(model.update),
			click: run(actions.restartBundledSolver),
		},
		{ id: "stop-solver", label: "Stop solver", enabled: canStop, click: run(actions.stopSolver) },
	]
}

/**
 * Whether a reset may start. It stops the solver and deletes the profile, so it needs an unlocked
 * session like stopping does, and it must not run alongside an update that is doing the same.
 */
export function resetAvailable(model: Pick<DesktopMenuModel, "locked" | "update" | "resetting">): boolean {
	return !model.locked && !model.resetting && !updateInstalling(model.update)
}

function resetItem(model: DesktopMenuModel, actions: DesktopMenuActions): MenuItemConstructorOptions {
	return {
		id: "reset-configuration",
		label: model.resetting ? "Resetting Configuration…" : "Reset Configuration…",
		enabled: resetAvailable(model),
		click: run(actions.resetConfiguration),
	}
}

function updateItems(model: DesktopMenuModel, actions: DesktopMenuActions): MenuItemConstructorOptions[] {
	const { state } = model.update
	const installing = updateInstalling(model.update)
	// A downloaded update turns the check into the install, which needs a solver it can stop.
	const ready = state === "ready"
	const canInstall = canStopSolver(model.status) && !model.resetting
	return [
		{
			id: "check-for-updates",
			label: updateMenuLabel(model.update),
			visible: model.updatesEnabled,
			enabled:
				!model.locked && !installing && state !== "checking" && state !== "downloading" && (!ready || canInstall),
			click: run(ready ? actions.installUpdate : actions.checkForUpdates),
		},
		{
			id: "update-channel",
			label: "Update Channel",
			visible: model.updatesEnabled,
			enabled: !model.locked && !installing,
			submenu: (["stable", "beta"] as const).map((channel) => ({
				id: `update-channel-${channel}`,
				label: channel === "stable" ? "Stable" : "Beta",
				type: "radio" as const,
				checked: model.update.channel === channel,
				click: run(() => actions.setUpdateChannel(channel)),
			})),
		},
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
		...updateItems(model, actions),
		{ id: "open-data-directory", label: "Open Data Directory", click: run(actions.openDataDirectory) },
		{
			id: "open-current-log",
			label: "Open Current Log",
			enabled: !model.locked && model.logAvailable,
			click: run(actions.openLog),
		},
		resetItem(model, actions),
		{ type: "separator" },
		{
			id: "quit-simplex",
			label: appOnlyQuitLabel(model.status),
			accelerator: "CmdOrCtrl+Q",
			click: actions.quitApp,
		},
		{
			id: "stop-and-quit",
			label: "Stop solver and quit",
			enabled: !model.locked && canStopSolver(model.status),
			click: run(actions.stopAndQuit),
		},
	]
}

function canStopSolver(status: SolverStatus): boolean {
	return status.state === "setup" || status.state === "running" || status.state === "paused"
}

function appOnlyQuitLabel(status: SolverStatus): string {
	return status.state === "running" || status.state === "paused"
		? "Quit Simplex (solver keeps filling)"
		: "Quit Simplex"
}

export function buildApplicationMenuTemplate(
	model: DesktopMenuModel,
	actions: DesktopMenuActions,
	platform: NodeJS.Platform = process.platform,
): MenuItemConstructorOptions[] {
	const macApplicationItems: MenuItemConstructorOptions[] =
		platform === "darwin"
			? [{ type: "separator" }, { role: "hide" }, { role: "hideOthers" }, { role: "unhide" }]
			: []
	const windowMenu: MenuItemConstructorOptions =
		platform === "darwin"
			? {
					role: "windowMenu",
					submenu: [
						{ role: "close" },
						{ role: "minimize" },
						{ role: "zoom" },
						{ type: "separator" },
						{ role: "front" },
					],
				}
			: { role: "windowMenu" }
	// Zoom only, not Electron's viewMenu role, which also adds reload and devtools.
	// Cmd+= is the key most people press for zoom in; the Plus accelerator stays
	// working through a hidden duplicate.
	const viewMenu: MenuItemConstructorOptions = {
		label: "View",
		submenu: [
			{ role: "resetZoom" },
			{ role: "zoomIn", accelerator: "CmdOrCtrl+=" },
			{ role: "zoomIn", accelerator: "CmdOrCtrl+Plus", visible: false, acceleratorWorksWhenHidden: true },
			{ role: "zoomOut" },
			{ type: "separator" },
			{ role: "togglefullscreen" },
		],
	}
	return [
		{
			label: "Simplex",
			submenu: [
				{ id: "about-simplex", label: "About Simplex", click: actions.showAbout },
				...updateItems(model, actions),
				...macApplicationItems,
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
				resetItem(model, actions),
				{ type: "separator" },
				{
					id: "quit-simplex",
					label: appOnlyQuitLabel(model.status),
					accelerator: "CmdOrCtrl+Q",
					click: actions.quitApp,
				},
				{
					id: "stop-and-quit",
					label: "Stop solver and quit",
					enabled: canStopSolver(model.status),
					click: run(actions.stopAndQuit),
				},
			],
		},
		{ role: "editMenu" },
		viewMenu,
		windowMenu,
	]
}

function updateMenuLabel(status: UpdateStatus): string {
	switch (status.state) {
		case "checking":
			return "Checking for Updates…"
		case "downloading":
			return status.progress === undefined
				? "Downloading Update…"
				: `Downloading Update… ${Math.round(status.progress)}%`
		case "ready":
			return `Install Update ${status.targetVersion ?? ""}`.trimEnd()
		case "stopping-solver":
			return "Installing Update — Stopping Solver"
		case "installing":
			return "Installing Update…"
		case "error":
			return "Update Error — Retry"
		default:
			return "Check for Updates…"
	}
}
