import { existsSync } from "node:fs"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import {
	app,
	BrowserWindow,
	dialog,
	Menu,
	nativeImage,
	Notification,
	powerSaveBlocker,
	protocol,
	session,
	shell,
	Tray,
} from "electron"
import electronUpdater from "electron-updater"
import { ensureDaemon, probeHealth, type DaemonLaunch } from "./daemon"
import { installSessionSecurity, installWebContentsSecurity, rendererWebPreferences } from "./desktop-security"
import { assertResources, resourcePaths, socketPathFor, userDataOverrideFromArgv } from "./desktop-paths"
import { latestLogPath, loginItemExecutable, LoginItemController } from "./login-item"
import { proxyToSimplex } from "./protocol"
import {
	holdsMachineAwake,
	sendSolverAction,
	shouldNotifySolverFailure,
	SolverSupervisor,
	solverHasVersionSkew,
	stopRequestAccepted,
	type SolverStatus,
} from "./solver-supervisor"
import {
	buildApplicationMenuTemplate,
	buildTrayMenuTemplate,
	solverStatusLabel,
	type DesktopMenuActions,
} from "./tray-menu"
import { TRAY_ICON_STATES, trayIconPath, trayIconRetinaPath } from "./tray-icon"
import { UpdateCoordinator, waitForSolverExit, type UpdateStatus } from "./update-coordinator"
import { FileUpdateStore, type UpdateChannel } from "./update-store"

const { autoUpdater } = electronUpdater

protocol.registerSchemesAsPrivileged([
	{ scheme: "simplex", privileges: { standard: true, secure: true, supportFetchAPI: true, stream: true } },
])

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
const STATUS_POLL_INTERVAL_MS = 3_000
const STOP_TIMEOUT_MS = 30_000

let mainWindow: BrowserWindow | undefined
let tray: Tray | undefined
let daemonLaunch: DaemonLaunch | undefined
let supervisor: SolverSupervisor | undefined
let loginItem: LoginItemController | undefined
let trayIconDirectory: string | undefined
let applicationIconPath: string | undefined
let dataDirectory: string | undefined
let startPromise: Promise<boolean> | undefined
let powerSaveBlockerId: number | undefined
let quitting = false
let intentionalStop = false
let updateCoordinator: UpdateCoordinator | undefined
let updateStatus: UpdateStatus = { state: "disabled", channel: "stable" }

const userDataSwitch = app.commandLine.getSwitchValue("user-data-dir")
const userDataOverride = userDataSwitch ? resolve(userDataSwitch) : userDataOverrideFromArgv()
if (userDataOverride) app.setPath("userData", userDataOverride)

async function simplexPackageRoot(): Promise<string> {
	const manifest = import.meta.resolve("@hyperbridge/simplex/package.json")
	return dirname(fileURLToPath(manifest))
}

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error)
}

function reportActionError(title: string, error: unknown): void {
	dialog.showErrorBox(title, errorMessage(error))
}

function currentPowerSaveState(): boolean {
	return powerSaveBlockerId !== undefined && powerSaveBlocker.isStarted(powerSaveBlockerId)
}

function syncPowerSaveBlocker(status: SolverStatus): void {
	const shouldBlock = holdsMachineAwake(status)
	if (shouldBlock && !currentPowerSaveState()) {
		powerSaveBlockerId = powerSaveBlocker.start("prevent-app-suspension")
	} else if (!shouldBlock && powerSaveBlockerId !== undefined) {
		if (powerSaveBlocker.isStarted(powerSaveBlockerId)) powerSaveBlocker.stop(powerSaveBlockerId)
		powerSaveBlockerId = undefined
	}
}

function trayImage(status: SolverStatus) {
	if (!trayIconDirectory) throw new Error("The Simplex tray icons are not initialized")
	const path = trayIconPath(trayIconDirectory, status)
	if (!existsSync(path)) throw new Error(`The Simplex tray icon is missing: ${path}`)
	const image = nativeImage.createFromPath(path)
	if (image.isEmpty()) throw new Error("Electron could not render the Simplex tray icon")
	if (process.platform === "darwin") {
		image.setTemplateImage(true)
		return image
	}
	return image.resize({ width: 24 })
}

const menuActions: DesktopMenuActions = {
	showWindow: () => safeShowWindow(),
	togglePause: () => togglePause(),
	stopSolver: async () => {
		await stopSolver()
	},
	restartSolver: () => restartSolver(),
	restartBundledSolver: () => restartBundledSolver(),
	toggleLoginItem: () => toggleLoginItem(),
	showAbout: () => app.showAboutPanel(),
	checkForUpdates: () => updateCoordinator?.checkNow(),
	setUpdateChannel: (channel: UpdateChannel) => updateCoordinator?.setChannel(channel),
	openDataDirectory: () => openDataDirectory(),
	openLog: () => openCurrentLog(),
	quitApp: () => quitApp(),
	stopAndQuit: () => stopAndQuit(),
}

function refreshNativeUi(): void {
	if (!supervisor || !loginItem) return
	const status = supervisor.status
	const model = {
		status,
		loginItemSupported: loginItem.supported,
		loginItemEnabled: loginItem.isEnabled(),
		logAvailable: Boolean(dataDirectory && latestLogPath(dataDirectory)),
		sleepPreventionActive: currentPowerSaveState(),
		updatesEnabled: app.isPackaged && Boolean(updateCoordinator),
		update: updateStatus,
		versionSkew: solverHasVersionSkew(status, app.getVersion()),
	}

	if (tray && !tray.isDestroyed()) {
		tray.setImage(trayImage(status))
		tray.setToolTip(`Simplex — ${solverStatusLabel(status).replace("Solver: ", "")}`)
		// Linux StatusNotifierItems require the context menu to be reset after changes.
		tray.setContextMenu(Menu.buildFromTemplate(buildTrayMenuTemplate(model, menuActions)))
	}
	Menu.setApplicationMenu(Menu.buildFromTemplate(buildApplicationMenuTemplate(model, menuActions)))
	if (mainWindow && !mainWindow.isDestroyed()) {
		mainWindow.setTitle(`Simplex — ${solverStatusLabel(status).replace("Solver: ", "")}`)
	}
}

function notifySolverFailure(status: SolverStatus): void {
	if (!Notification.isSupported()) return
	try {
		const notification = new Notification({
			title: "Simplex solver stopped",
			body:
				status.state === "unreachable"
					? `The solver cannot be reached: ${status.detail}. Restart it from the Simplex menu.`
					: "The solver is no longer running. Restart it from the Simplex menu.",
		})
		notification.on("click", () => void safeShowWindow())
		notification.show()
	} catch (error) {
		console.error(`Simplex could not display its solver notification: ${errorMessage(error)}`)
	}
}

function onSolverStatusChanged(next: SolverStatus, previous: SolverStatus): void {
	syncPowerSaveBlocker(next)
	refreshNativeUi()
	if ((next.state === "stopped" || next.state === "unreachable") && intentionalStop) {
		intentionalStop = false
		return
	}
	if (shouldNotifySolverFailure(previous, next, intentionalStop)) notifySolverFailure(next)
}

async function startOrAttachSolver(fatal: boolean): Promise<boolean> {
	if (startPromise) return startPromise
	startPromise = (async () => {
		if (!daemonLaunch || !supervisor) throw new Error("The Simplex desktop runtime is not initialized")
		supervisor.setStatus({ state: "starting" })
		try {
			await ensureDaemon({ launch: daemonLaunch })
			await supervisor.pollNow()
			return true
		} catch (error) {
			supervisor.setStatus({ state: "unreachable", detail: errorMessage(error) })
			if (fatal) throw error
			reportActionError("Simplex could not restart the solver", error)
			return false
		} finally {
			startPromise = undefined
		}
	})()
	return startPromise
}

async function waitForStopAccepted(): Promise<void> {
	if (!supervisor) return
	const deadline = Date.now() + STOP_TIMEOUT_MS
	while (Date.now() < deadline) {
		const status = await supervisor.pollNow()
		if (stopRequestAccepted(status)) return
		await new Promise((resolveDelay) => setTimeout(resolveDelay, 250))
	}
	throw new Error(`Simplex did not stop within ${STOP_TIMEOUT_MS / 1_000} seconds`)
}

async function togglePause(): Promise<void> {
	if (!daemonLaunch || !supervisor) return
	const action = supervisor.status.state === "paused" ? "resume" : "pause"
	try {
		await sendSolverAction(daemonLaunch.socketPath, action)
		await supervisor.pollNow()
	} catch (error) {
		reportActionError(`Simplex could not ${action} filling`, error)
	}
}

async function stopSolver(): Promise<boolean> {
	if (!daemonLaunch || !supervisor) return false
	intentionalStop = true
	try {
		await sendSolverAction(daemonLaunch.socketPath, "stop")
		await waitForStopAccepted()
		return true
	} catch (error) {
		intentionalStop = false
		reportActionError("Simplex could not stop the solver", error)
		return false
	}
}

async function restartSolver(): Promise<void> {
	if (supervisor?.status.state === "stopping") return
	await startOrAttachSolver(false)
}

async function restartBundledSolver(): Promise<void> {
	if (!daemonLaunch || !supervisor) return
	const status = await supervisor.pollNow()
	if (status.state === "running" || status.state === "paused" || status.state === "setup") {
		intentionalStop = true
		try {
			await sendSolverAction(daemonLaunch.socketPath, "stop")
			const exited = await waitForSolverExit({
				pid: status.pid,
				probe: () => probeHealth(daemonLaunch!.socketPath),
			})
			if (!exited) throw new Error("The old solver did not finish its graceful shutdown")
		} catch (error) {
			intentionalStop = false
			return reportActionError("Simplex could not restart with its bundled solver", error)
		}
	}
	await startOrAttachSolver(false)
}

function toggleLoginItem(): void {
	if (!loginItem) return
	try {
		loginItem.setEnabled(!loginItem.isEnabled())
		refreshNativeUi()
	} catch (error) {
		reportActionError("Simplex could not change launch-at-login", error)
	}
}

async function openPath(path: string, title: string): Promise<void> {
	const failure = await shell.openPath(path)
	if (failure) dialog.showErrorBox(title, failure)
}

async function openDataDirectory(): Promise<void> {
	if (dataDirectory) await openPath(dataDirectory, "Simplex could not open its data directory")
}

async function openCurrentLog(): Promise<void> {
	if (!dataDirectory) return
	const path = latestLogPath(dataDirectory)
	if (!path) {
		dialog.showMessageBox({ type: "info", message: "Simplex has not created a log file yet." })
		return
	}
	await openPath(path, "Simplex could not open its current log")
}

function quitApp(): void {
	quitting = true
	app.quit()
}

async function stopAndQuit(): Promise<void> {
	if (await stopSolver()) quitApp()
}

async function createWindow(): Promise<void> {
	if (mainWindow && !mainWindow.isDestroyed()) {
		if (mainWindow.isMinimized()) mainWindow.restore()
		mainWindow.show()
		mainWindow.focus()
		return
	}
	if (!supervisor) return
	if (supervisor.status.state === "stopped" || supervisor.status.state === "unreachable") {
		await dialog.showMessageBox({
			type: "warning",
			title: "Simplex solver is not running",
			message: "The Simplex solver is not running.",
			detail: "Restart it explicitly from the Simplex tray or application menu.",
		})
		return
	}

	const window = new BrowserWindow({
		width: 1280,
		height: 840,
		minWidth: 880,
		minHeight: 640,
		show: false,
		icon: applicationIconPath,
		webPreferences: rendererWebPreferences(app.isPackaged),
	})
	mainWindow = window
	window.on("close", (event) => {
		if (quitting) return
		event.preventDefault()
		window.hide()
	})
	window.once("closed", () => {
		if (mainWindow === window) mainWindow = undefined
	})
	window.once("ready-to-show", () => window.show())
	refreshNativeUi()
	await window.loadURL("simplex://local/")
}

async function showWindow(): Promise<void> {
	if (process.platform === "darwin") await app.dock?.show()
	await createWindow()
}

async function safeShowWindow(): Promise<void> {
	try {
		await showWindow()
	} catch (error) {
		reportActionError("Simplex could not open its window", error)
	}
}

function createTray(): void {
	if (!supervisor) return
	tray = new Tray(trayImage(supervisor.status))
	if (process.platform === "win32") tray.on("click", () => void safeShowWindow())
	refreshNativeUi()
}

async function prepareDesktop(): Promise<void> {
	dataDirectory = app.getPath("userData")
	const socketPath = socketPathFor(dataDirectory)
	const resources = resourcePaths({
		isPackaged: app.isPackaged,
		resourcesPath: process.resourcesPath,
		packageRoot,
		simplexPackageRoot: app.isPackaged ? packageRoot : await simplexPackageRoot(),
	})
	assertResources(resources)
	const iconPath = join(dirname(resources.ui), "icons", "mobile-logo.svg")
	if (!existsSync(iconPath)) throw new Error(`The Simplex PWA tray icon is missing: ${iconPath}`)
	trayIconDirectory = app.isPackaged
		? join(process.resourcesPath, "desktop", "tray")
		: join(packageRoot, "resources", "tray")
	applicationIconPath = join(trayIconDirectory, "app.png")
	if (!existsSync(applicationIconPath))
		throw new Error(`The Simplex application icon is missing: ${applicationIconPath}`)
	for (const state of TRAY_ICON_STATES) {
		const path = trayIconPath(trayIconDirectory, { state })
		if (!existsSync(path)) throw new Error(`The Simplex ${state} tray icon is missing: ${path}`)
		if (process.platform === "darwin") {
			const retinaPath = trayIconRetinaPath(trayIconDirectory, { state })
			if (!existsSync(retinaPath))
				throw new Error(`The Simplex ${state} Retina tray icon is missing: ${retinaPath}`)
		}
	}
	if (process.platform === "darwin") app.dock?.setIcon(applicationIconPath)
	daemonLaunch = { nodePath: resources.node, solverPath: resources.solver, socketPath, dataDir: dataDirectory }

	await protocol.handle("simplex", (request) => proxyToSimplex(request, socketPath, undefined, app.getVersion()))
	installSessionSecurity(session.defaultSession)
	loginItem = new LoginItemController({
		app,
		platform: process.platform,
		isPackaged: app.isPackaged,
		executable: loginItemExecutable(process.platform, process.execPath),
	})
	const openedAtLogin = loginItem.wasOpenedAtLogin()
	if (openedAtLogin && process.platform === "darwin") app.dock?.hide()
	supervisor = new SolverSupervisor({
		socketPath,
		intervalMs: STATUS_POLL_INTERVAL_MS,
		onChange: onSolverStatusChanged,
	})
	app.setAboutPanelOptions({ applicationName: "Simplex", applicationVersion: app.getVersion() })
	createTray()
	await startOrAttachSolver(true)
	supervisor.start()
	updateCoordinator = new UpdateCoordinator({
		updater: autoUpdater,
		packaged: app.isPackaged,
		appVersion: app.getVersion(),
		store: new FileUpdateStore(dataDirectory),
		probeSolver: () => supervisor!.pollNow(),
		requestSolverStop: async () => {
			intentionalStop = true
			await sendSolverAction(socketPath, "stop")
		},
		waitForExit: (pid) => waitForSolverExit({ pid, probe: () => probeHealth(socketPath) }),
		onChange: (next) => {
			updateStatus = next
			if (next.state === "installing") quitting = true
			refreshNativeUi()
		},
		notify: (title, body) => {
			if (!Notification.isSupported()) return
			const notification = new Notification({ title, body })
			notification.on("click", () => void safeShowWindow())
			notification.show()
		},
	})
	updateStatus = updateCoordinator.status
	updateCoordinator.start()
	refreshNativeUi()

	if (!openedAtLogin) {
		await createWindow()
	}
}

function reportStartupError(error: unknown): void {
	const message = errorMessage(error)
	console.error(`Simplex could not start: ${message}`)
	dialog.showErrorBox("Simplex could not start", message)
	quitting = true
	app.quit()
}

if (!app.requestSingleInstanceLock()) {
	quitting = true
	app.quit()
} else {
	app.on("web-contents-created", (_event, webContents) => {
		installWebContentsSecurity(
			webContents,
			(url) => shell.openExternal(url),
			() =>
				dialog.showErrorBox("Could not open link", "Simplex could not open this link in your default browser."),
		)
	})
	app.on("second-instance", () => void safeShowWindow())
	app.on("activate", () => void safeShowWindow())
	app.on("before-quit", () => {
		quitting = true
		supervisor?.stop()
		updateCoordinator?.dispose()
		if (powerSaveBlockerId !== undefined && powerSaveBlocker.isStarted(powerSaveBlockerId)) {
			powerSaveBlocker.stop(powerSaveBlockerId)
		}
		powerSaveBlockerId = undefined
	})
	// A closed window leaves the tray and detached solver running on every OS.
	app.on("window-all-closed", () => {})
	app.whenReady().then(prepareDesktop).catch(reportStartupError)
}
