import { dirname, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import { app, BrowserWindow, dialog, protocol } from "electron"
import { ensureDaemon } from "./daemon"
import { assertResources, resourcePaths, socketPathFor } from "./desktop-paths"
import { proxyToSimplex } from "./protocol"

protocol.registerSchemesAsPrivileged([
	{ scheme: "simplex", privileges: { standard: true, secure: true, supportFetchAPI: true, stream: true } },
])

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..")
let mainWindow: BrowserWindow | undefined
let daemonReady: Promise<void> | undefined

async function simplexPackageRoot(): Promise<string> {
	const manifest = import.meta.resolve("@hyperbridge/simplex/package.json")
	return dirname(fileURLToPath(manifest))
}

async function prepareDaemon(): Promise<void> {
	const userData = app.getPath("userData")
	const socketPath = socketPathFor(userData)
	const resources = resourcePaths({
		isPackaged: app.isPackaged,
		resourcesPath: process.resourcesPath,
		packageRoot,
		simplexPackageRoot: app.isPackaged ? packageRoot : await simplexPackageRoot(),
	})
	assertResources(resources)
	await ensureDaemon({
		launch: {
			nodePath: resources.node,
			solverPath: resources.solver,
			socketPath,
			dataDir: userData,
		},
	})
	await protocol.handle("simplex", (request) => proxyToSimplex(request, socketPath))
}

async function createWindow(): Promise<void> {
	if (mainWindow && !mainWindow.isDestroyed()) {
		if (mainWindow.isMinimized()) mainWindow.restore()
		mainWindow.focus()
		return
	}
	if (!daemonReady) daemonReady = prepareDaemon()
	await daemonReady

	const window = new BrowserWindow({
		width: 1280,
		height: 840,
		minWidth: 880,
		minHeight: 640,
		show: false,
		webPreferences: {
			contextIsolation: true,
			nodeIntegration: false,
			sandbox: true,
			webviewTag: false,
			devTools: !app.isPackaged,
		},
	})
	mainWindow = window
	window.once("ready-to-show", () => window.show())
	window.once("closed", () => {
		if (mainWindow === window) mainWindow = undefined
	})
	await window.loadURL("simplex://local/")
}

function reportStartupError(error: unknown): void {
	const message = error instanceof Error ? error.message : String(error)
	dialog.showErrorBox("Simplex could not start", message)
	app.quit()
}

function launchWindow(): void {
	void createWindow().catch(reportStartupError)
}

if (!app.requestSingleInstanceLock()) {
	app.quit()
} else {
	app.on("second-instance", launchWindow)
	app.whenReady().then(createWindow).catch(reportStartupError)
	app.on("activate", launchWindow)
	app.on("window-all-closed", () => {
		if (process.platform !== "darwin") app.quit()
	})
}
