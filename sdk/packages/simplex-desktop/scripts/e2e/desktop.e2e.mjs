import assert from "node:assert/strict"
import { execFile } from "node:child_process"
import { createRequire } from "node:module"
import { existsSync } from "node:fs"
import { mkdtemp, readFile, readdir, realpath, rm, stat, writeFile } from "node:fs/promises"
import { request as httpRequest } from "node:http"
import { createServer as createNetServer } from "node:net"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import test from "node:test"
import { promisify } from "node:util"
import { _electron } from "playwright-core"
import { ActivityRecorder } from "../../../simplex/src/data/recorder.ts"
import { emitFillerToml } from "../../../simplex/src/cli/init/emit-toml.ts"
import externalLinks from "../../../simplex/src/config/external-links.json" with { type: "json" }
import { MemoryDataStore } from "../../../simplex/src/data/memory.ts"
import { UiServer } from "../../../simplex/src/services/server/UiServer.ts"
import { socketPathFor } from "../../src/desktop-paths.ts"

const execFileAsync = promisify(execFile)
const require = createRequire(import.meta.url)
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..")
const simplexRoot = resolve(packageRoot, "../simplex")
const electronExecutable = require("electron")
const TEST_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
const TEST_SEED = "bottom drive obey lake curtain smoke basket hold race lonely fit walk"
const FIXTURE_KEY = "operator-fixture-key-do-not-expose"
const FIXTURE_SEED = "operator-fixture-substrate-do-not-expose"

function delay(milliseconds) {
	return new Promise((resolveDelay) => setTimeout(resolveDelay, milliseconds))
}

async function temporaryUserData(label) {
	// Electron canonicalizes macOS' /var symlink to /private/var. Socket naming is
	// byte-sensitive, so derive expectations from the same canonical directory.
	return realpath(await mkdtemp(join(tmpdir(), `simplex-desktop-${label}-`)))
}

async function waitFor(check, description, timeoutMs = 30_000) {
	const deadline = Date.now() + timeoutMs
	let lastError
	while (Date.now() < deadline) {
		try {
			const value = await check()
			if (value) return value
		} catch (error) {
			lastError = error
		}
		await delay(100)
	}
	throw new Error(`Timed out waiting for ${description}${lastError ? `: ${lastError.message}` : ""}`)
}

function socketRequest(socketPath, path, options = {}) {
	return new Promise((resolveRequest, reject) => {
		const request = httpRequest(
			{
				socketPath,
				path,
				method: options.method ?? "GET",
				headers: options.headers,
			},
			(response) => {
				const chunks = []
				response.on("data", (chunk) => chunks.push(chunk))
				response.on("end", () =>
					resolveRequest({
						status: response.statusCode ?? 0,
						headers: response.headers,
						body: Buffer.concat(chunks).toString("utf8"),
					}),
				)
			},
		)
		request.on("error", reject)
		request.end(options.body)
	})
}

async function waitForHealth(socketPath, mode) {
	return waitFor(
		async () => {
			const response = await socketRequest(socketPath, "/health")
			if (response.status !== 200) return false
			const health = JSON.parse(response.body)
			return health.status === "ok" && (!mode || health.mode === mode) ? health : false
		},
		`Simplex ${mode ?? ""} health on ${socketPath}`,
		120_000,
	)
}

async function launchDesktop(userDataDir, options = {}) {
	const args = [packageRoot, `--user-data-dir=${userDataDir}`]
	if (options.hidden) args.push("--hidden")
	const electronApp = await _electron.launch({
		executablePath: electronExecutable,
		args,
		cwd: packageRoot,
		env: { ...process.env, ELECTRON_DISABLE_SECURITY_WARNINGS: "true" },
		timeout: 120_000,
	})
	if (options.hidden) return { electronApp }
	const page = await waitFor(() => electronApp.windows()[0], "the Simplex BrowserWindow", 120_000)
	await page.waitForURL("simplex://local/**", { timeout: 120_000 })
	return { electronApp, page }
}

async function processRows() {
	if (process.platform === "win32") {
		const script =
			'Get-CimInstance Win32_Process | Where-Object { $_.CommandLine } | ForEach-Object { "$($_.ProcessId)`t$($_.CommandLine)" }'
		const { stdout } = await execFileAsync("powershell.exe", ["-NoProfile", "-Command", script])
		return stdout.split(/\r?\n/)
	}
	const { stdout } = await execFileAsync("ps", ["-axo", "pid=,command="])
	return stdout.split("\n")
}

async function daemonPids(userDataDir) {
	const rows = await processRows()
	return rows
		.filter((row) => row.includes("simplex.js") && row.includes("--data-dir") && row.includes(userDataDir))
		.map((row) => Number(row.trim().split(/\s+/, 1)[0]))
		.filter(Number.isInteger)
}

async function waitForDaemonPids(userDataDir, count = 1) {
	return waitFor(
		async () => {
			const pids = await daemonPids(userDataDir)
			return pids.length === count ? pids : false
		},
		`${count} detached daemon process(es) for ${userDataDir}`,
		120_000,
	)
}

async function assertNoTcpListener(pid) {
	if (process.platform === "win32") {
		const script = `Get-NetTCPConnection -State Listen -OwningProcess ${pid} -ErrorAction SilentlyContinue | Select-Object -ExpandProperty LocalPort`
		const { stdout } = await execFileAsync("powershell.exe", ["-NoProfile", "-Command", script])
		assert.equal(stdout.trim(), "", `daemon ${pid} must not open a TCP listener`)
		return
	}
	try {
		const { stdout } = await execFileAsync("lsof", ["-a", "-Pan", "-p", String(pid), "-iTCP", "-sTCP:LISTEN"])
		assert.equal(stdout.trim(), "", `daemon ${pid} must not open a TCP listener`)
	} catch (error) {
		// lsof returns 1 when no file matches, which is the successful case here.
		if (error.code !== 1 || error.stdout?.trim()) throw error
	}
}

async function killProcess(pid) {
	try {
		if (process.platform === "win32") await execFileAsync("taskkill.exe", ["/PID", String(pid), "/T", "/F"])
		else process.kill(pid, "SIGTERM")
	} catch (error) {
		if (error.code !== "ESRCH") throw error
	}
}

async function hardKillElectron(electronApp) {
	const closed = new Promise((resolveClose) => electronApp.once("close", resolveClose))
	const child = electronApp.process()
	if (process.platform === "win32") await execFileAsync("taskkill.exe", ["/PID", String(child.pid), "/F"])
	else child.kill("SIGKILL")
	await closed
}

async function quitElectron(electronApp) {
	const closed = new Promise((resolveClose) => electronApp.once("close", resolveClose))
	await electronApp.evaluate(({ app }) => app.quit()).catch(() => {})
	await closed
}

async function cleanupDesktop(electronApp, userDataDir) {
	if (electronApp) {
		try {
			await quitElectron(electronApp)
		} catch {
			// A hard-killed Electron app is already closed.
		}
	}
	for (const pid of await daemonPids(userDataDir)) await killProcess(pid)
	await waitFor(async () => (await daemonPids(userDataDir)).length === 0, "detached daemon cleanup").catch(() => {})
	await rm(userDataDir, { recursive: true, force: true })
}

async function blackholeServer() {
	const sockets = new Set()
	const server = createNetServer((socket) => {
		sockets.add(socket)
		socket.on("close", () => sockets.delete(socket))
	})
	await new Promise((resolveListen, reject) => {
		server.once("error", reject)
		server.listen(0, "127.0.0.1", resolveListen)
	})
	const address = server.address()
	if (!address || typeof address === "string") throw new Error("Blackhole server did not bind TCP")
	return {
		port: address.port,
		close: async () => {
			for (const socket of sockets) socket.destroy()
			await new Promise((resolveClose) => server.close(resolveClose))
		},
	}
}

async function regularFilesUnder(directory) {
	let entries
	try {
		entries = await readdir(directory, { withFileTypes: true })
	} catch (error) {
		if (error.code === "ENOENT") return []
		throw error
	}
	const files = []
	for (const entry of entries) {
		const path = join(directory, entry.name)
		if (entry.isDirectory()) files.push(...(await regularFilesUnder(path)))
		else if (entry.isFile()) files.push(path)
	}
	return files
}

async function assertSecretsExistOnlyInConfig(userDataDir, configPath, secrets) {
	for (const path of await regularFilesUnder(userDataDir)) {
		if (path === configPath) continue
		const content = await readFile(path)
		for (const secret of secrets) {
			assert.equal(content.includes(Buffer.from(secret)), false, `${path} must not persist setup key material`)
		}
	}
}

function operatorFixture(socketPath, options = {}) {
	const data = new MemoryDataStore()
	const activity = new ActivityRecorder(data.activity)
	const originalOrderHistory = activity.orderHistory.bind(activity)
	let historyReads = 0
	let pauseWrites = 0
	let paused = false
	let server
	activity.orderHistory = (...args) => {
		historyReads += 1
		return originalOrderHistory(...args)
	}
	const config = {
		simplex: {
			signer: { type: "privateKey", key: FIXTURE_KEY },
			substratePrivateKey: FIXTURE_SEED,
			hyperbridgeWsUrl: "wss://example.invalid",
		},
		pairs: [],
		chains: [],
	}
	const operator = {
		strategies: [],
		filler: { pause() {}, resume() {}, isPaused: () => paused, getWatchOnly: () => ({}) },
		balances: { getSnapshot: () => ({ updatedAt: null, status: "loading", chains: [], issues: [] }) },
		haltControls: [],
		config,
		stop: async () => {
			if (options.stopBarrier) await options.stopBarrier
			server.stop()
		},
		activity,
		bids: data.bids,
		setPaused: async (value) => {
			pauseWrites += 1
			paused = value
		},
		setLogLevel() {},
		applyAllowlist() {},
		applyRebalancing() {},
		version: "0.0.0-test",
		startedAt: Date.now(),
		configPath: join(dirname(socketPath), "filler-config.toml"),
		chains: [],
		strategyTypes: [],
	}
	server = new UiServer({ mode: "operator", uiDistDir: join(simplexRoot, "dist/ui"), operator })
	return {
		server,
		activity,
		start: () => server.start({ socketPath }),
		stop: () => server.stop(),
		historyReads: () => historyReads,
		pauseWrites: () => pauseWrites,
		clientCount: () => server.sseClients.size,
	}
}

test("window close, app quit, hard crash, and second launch preserve one detached solver", async (t) => {
	const userDataDir = await temporaryUserData("lifecycle")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	t.after(async () => cleanupDesktop(electronApp, userDataDir))
	;({ electronApp } = await launchDesktop(userDataDir))
	await waitForHealth(socketPath, "init")
	const [daemonPid] = await waitForDaemonPids(userDataDir)
	await assertNoTcpListener(daemonPid)
	await waitFor(
		async () =>
			(await electronApp.evaluate(
				({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("solver-status")?.label,
			)) === "Solver: Setup required",
		"setup status in the native menu",
	)
	const nativeMenu = await electronApp.evaluate(({ Menu }) => {
		const menu = Menu.getApplicationMenu()
		return {
			items: menu?.items.map((entry) => ({
				label: entry.label,
				submenu: entry.submenu?.items.map((child) => ({
					label: child.label,
					role: child.role,
					accelerator: child.accelerator,
				})),
			})),
			quitAccelerator: menu?.getMenuItemById("quit-simplex")?.accelerator,
			updatesVisible: menu?.getMenuItemById("check-for-updates")?.visible,
		}
	})
	const editItems = nativeMenu.items.find((entry) => entry.label === "Edit")?.submenu ?? []
	const windowItems = nativeMenu.items.find((entry) => entry.label === "Window")?.submenu ?? []
	assert.equal(editItems.find((entry) => entry.role === "copy")?.accelerator, "CommandOrControl+C")
	assert.equal(editItems.find((entry) => entry.role === "paste")?.accelerator, "CommandOrControl+V")
	assert.equal(windowItems.find((entry) => entry.role === "close")?.accelerator, "CommandOrControl+W")
	assert.equal(windowItems.find((entry) => entry.role === "minimize")?.accelerator, "CommandOrControl+M")
	assert.equal(nativeMenu.quitAccelerator, "CmdOrCtrl+Q")
	assert.equal(nativeMenu.updatesVisible, false)
	const logFiles = await waitFor(async () => {
		const files = await regularFilesUnder(join(userDataDir, "logs"))
		return files.some((path) => /simplex-[\dT-]+\.log$/.test(path)) ? files : false
	}, "a persistent solver log")
	assert.ok(logFiles.length > 0)

	await electronApp.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0]?.close())
	await waitFor(
		async () => !(await electronApp.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0]?.isVisible())),
		"the closed window to hide",
	)
	await waitForHealth(socketPath, "init")
	assert.deepEqual(await daemonPids(userDataDir), [daemonPid], "closing the window must leave Simplex alive")

	await execFileAsync(electronExecutable, [packageRoot, `--user-data-dir=${userDataDir}`], {
		cwd: packageRoot,
		env: { ...process.env, ELECTRON_DISABLE_SECURITY_WARNINGS: "true" },
		timeout: 30_000,
	})
	await waitFor(
		async () => await electronApp.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0]?.isVisible()),
		"the first instance to focus its window",
	)
	assert.deepEqual(await daemonPids(userDataDir), [daemonPid], "a second desktop launch must not spawn a solver")

	const appQuit = new Promise((resolveClose) => electronApp.once("close", resolveClose))
	await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("quit-simplex")?.click())
	await appQuit
	electronApp = undefined
	await waitForHealth(socketPath, "init")
	assert.deepEqual(await daemonPids(userDataDir), [daemonPid], "Quit Simplex must leave the solver alive")
	;({ electronApp } = await launchDesktop(userDataDir))
	assert.deepEqual(await waitForDaemonPids(userDataDir), [daemonPid], "relaunch must attach after app-only quit")

	await hardKillElectron(electronApp)
	electronApp = undefined
	await waitForHealth(socketPath, "init")
	assert.deepEqual(await daemonPids(userDataDir), [daemonPid], "hard-closing Electron must leave Simplex alive")
	;({ electronApp } = await launchDesktop(userDataDir))
	await waitForHealth(socketPath, "init")
	assert.deepEqual(await waitForDaemonPids(userDataDir), [daemonPid], "relaunch must attach instead of spawning")
	await quitElectron(electronApp)
	electronApp = undefined
	await waitForHealth(socketPath, "init")
})

test("a hidden login launch starts the app and solver without opening a window", async (t) => {
	const userDataDir = await temporaryUserData("login")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	t.after(async () => cleanupDesktop(electronApp, userDataDir))
	;({ electronApp } = await launchDesktop(userDataDir, { hidden: true }))
	await waitForHealth(socketPath, "init")
	await waitForDaemonPids(userDataDir)
	assert.equal(await electronApp.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows().length), 0)
	assert.ok(
		await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("show-simplex")?.enabled),
	)
	await quitElectron(electronApp)
	electronApp = undefined
	await waitForHealth(socketPath, "init")
})

test("configured startup owns the socket before filling and relaunch attaches while booting", async (t) => {
	const userDataDir = await temporaryUserData("startup-lock")
	const socketPath = socketPathFor(userDataDir)
	const blackhole = await blackholeServer()
	let electronApp
	t.after(async () => {
		await cleanupDesktop(electronApp, userDataDir)
		await blackhole.close()
	})
	const config = {
		simplex: {
			signer: { type: "privateKey", key: TEST_KEY },
			maxConcurrentOrders: 5,
			substratePrivateKey: TEST_SEED,
			hyperbridgeWsUrl: `ws://127.0.0.1:${blackhole.port}`,
		},
		pairs: [
			{
				token0: "USDC",
				token1: "USDC",
				maxOrderSize: "100000",
				askPriceCurve: [
					{ amount: "100", price: "0.99" },
					{ amount: "100000", price: "0.999" },
				],
			},
		],
		chains: [
			{
				rpcUrls: [`http://127.0.0.1:${blackhole.port}`],
				bundlerUrl: `http://127.0.0.1:${blackhole.port}`,
			},
		],
	}
	await writeFile(join(userDataDir, "filler-config.toml"), emitFillerToml(config), { mode: 0o600 })
	;({ electronApp } = await launchDesktop(userDataDir, { hidden: true }))
	await waitFor(async () => {
		const response = await socketRequest(socketPath, "/health")
		return JSON.parse(response.body).status === "starting"
	}, "configured solver to bind its startup lock")
	const [daemonPid] = await waitForDaemonPids(userDataDir)

	await hardKillElectron(electronApp)
	electronApp = undefined
	;({ electronApp } = await launchDesktop(userDataDir, { hidden: true }))
	await waitFor(async () => {
		const response = await socketRequest(socketPath, "/health")
		return JSON.parse(response.body).status === "starting"
	}, "the relaunched desktop to observe startup in progress")
	assert.deepEqual(await daemonPids(userDataDir), [daemonPid], "relaunch must not spawn a second booting solver")
})

test("Stop solver and quit closes Electron after shutdown is accepted", async (t) => {
	const userDataDir = await temporaryUserData("stop-and-quit")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	let releaseDrain
	const stopBarrier = new Promise((resolveDrain) => {
		releaseDrain = resolveDrain
	})
	const fixture = operatorFixture(socketPath, { stopBarrier })
	t.after(async () => {
		releaseDrain?.()
		try {
			await fixture.stop()
		} catch {
			// The menu action is expected to have stopped it already.
		}
		await cleanupDesktop(electronApp, userDataDir)
	})

	await fixture.start()
	;({ electronApp } = await launchDesktop(userDataDir))
	const closed = new Promise((resolveClose) => electronApp.once("close", resolveClose))
	await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("stop-and-quit")?.click())
	await closed
	electronApp = undefined
	const health = JSON.parse((await socketRequest(socketPath, "/health")).body)
	assert.equal(health.status, "stopping", "the detached solver may continue draining after Electron exits")
	releaseDrain()
	await waitFor(async () => {
		try {
			await socketRequest(socketPath, "/health")
			return false
		} catch {
			return true
		}
	}, "the detached solver to finish draining")
})

test("graceful stop keeps the socket lock and disables restart until draining finishes", async (t) => {
	const userDataDir = await temporaryUserData("stopping-lock")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	let releaseDrain
	const stopBarrier = new Promise((resolveDrain) => {
		releaseDrain = resolveDrain
	})
	const fixture = operatorFixture(socketPath, { stopBarrier })
	t.after(async () => {
		releaseDrain?.()
		try {
			await fixture.stop()
		} catch {
			// The stop action may already have closed the fixture.
		}
		await cleanupDesktop(electronApp, userDataDir)
	})

	await fixture.start()
	;({ electronApp } = await launchDesktop(userDataDir))
	await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("stop-solver")?.click())
	await waitFor(async () => {
		const response = await socketRequest(socketPath, "/health")
		return JSON.parse(response.body).status === "stopping"
	}, "the stopping health state")
	await waitFor(
		async () =>
			(await electronApp.evaluate(
				({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("solver-status")?.label,
			)) === "Solver: Stopping…",
		"the stopping native state",
	)
	assert.equal(
		await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("restart-solver")?.enabled),
		false,
	)
	await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("restart-solver")?.click())
	await delay(500)
	assert.deepEqual(await daemonPids(userDataDir), [], "Restart must not spawn while the old solver drains")

	releaseDrain()
	await waitFor(async () => {
		try {
			await socketRequest(socketPath, "/health")
			return false
		} catch {
			return true
		}
	}, "the drained fixture to release its socket")
})

test("the native shell reports a crashed solver, prevents sleep while active, and offers restart", async (t) => {
	const userDataDir = await temporaryUserData("supervision")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	const fixture = operatorFixture(socketPath)
	t.after(async () => {
		try {
			await fixture.stop()
		} catch {
			// A crash scenario may already have closed the fixture server.
		}
		await cleanupDesktop(electronApp, userDataDir)
	})

	await fixture.start()
	;({ electronApp } = await launchDesktop(userDataDir))
	await waitFor(
		async () =>
			(await electronApp.evaluate(
				({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("solver-status")?.label,
			)) === "Solver: Running",
		"running status in the native menu",
	)
	assert.equal(
		await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("sleep-prevention")?.label),
		"Sleep prevention: On",
	)

	await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("toggle-pause")?.click())
	await waitFor(
		async () =>
			(await electronApp.evaluate(
				({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("solver-status")?.label,
			)) === "Solver: Paused",
		"paused status in the native menu",
	)
	assert.equal(
		await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("sleep-prevention")?.label),
		"Sleep prevention: Off",
	)
	await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("toggle-pause")?.click())
	await waitFor(
		async () =>
			(await electronApp.evaluate(
				({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("solver-status")?.label,
			)) === "Solver: Running",
		"resumed status in the native menu",
	)

	await fixture.stop()
	await waitFor(
		async () =>
			(await electronApp.evaluate(
				({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("solver-status")?.label,
			)) === "Solver: Stopped",
		"crashed solver status within one polling interval",
		10_000,
	)
	assert.match(
		await electronApp.evaluate(({ BrowserWindow }) => BrowserWindow.getAllWindows()[0]?.getTitle() ?? ""),
		/Stopped/,
	)
	assert.equal(
		await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("restart-solver")?.enabled),
		true,
	)

	await electronApp.evaluate(({ Menu }) => Menu.getApplicationMenu()?.getMenuItemById("restart-solver")?.click())
	await waitForHealth(socketPath, "init")
	await waitForDaemonPids(userDataDir)
})

test("the custom protocol reconnects Orders SSE and releases streams across 20 reloads", async (t) => {
	const userDataDir = await temporaryUserData("sse")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	let fixture = operatorFixture(socketPath)
	t.after(async () => {
		fixture.stop()
		await cleanupDesktop(electronApp, userDataDir)
	})

	await fixture.start()
	let page
	;({ electronApp, page } = await launchDesktop(userDataDir))
	await page.goto("simplex://local/orders")
	await page.getByText("live", { exact: true }).waitFor({ timeout: 30_000 })
	await waitFor(() => fixture.clientCount() === 1, "one renderer SSE client")

	const readsBeforeRestart = fixture.historyReads()
	fixture.stop()
	await page.getByText("reconnecting…", { exact: true }).waitFor({ timeout: 30_000 })

	fixture = operatorFixture(socketPath)
	await fixture.start()
	await page.getByText("live", { exact: true }).waitFor({ timeout: 30_000 })
	await waitFor(() => fixture.historyReads() > 0, "history reload after EventSource reconnect")
	assert.ok(readsBeforeRestart > 0, "Orders must have loaded history before restart")

	for (let reload = 0; reload < 20; reload += 1) {
		await page.reload()
		await page.getByText("live", { exact: true }).waitFor({ timeout: 30_000 })
	}
	await waitFor(() => fixture.clientCount() === 1, "exactly one SSE stream after 20 reloads")
})

test("the renderer enforces CSP and opens only approved links outside Electron", async (t) => {
	const userDataDir = await temporaryUserData("security")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	const fixture = operatorFixture(socketPath)
	t.after(async () => {
		fixture.stop()
		await cleanupDesktop(electronApp, userDataDir)
	})

	await fixture.start()
	let page
	;({ electronApp, page } = await launchDesktop(userDataDir))
	await waitForHealth(socketPath, "operator")
	await page.addInitScript(() => {
		globalThis.__simplexCspViolations = []
		window.addEventListener("securitypolicyviolation", (event) => {
			globalThis.__simplexCspViolations.push({ directive: event.effectiveDirective, blocked: event.blockedURI })
		})
	})
	await electronApp.evaluate(({ shell }) => {
		globalThis.__simplexOpenedUrls = []
		shell.openExternal = async (url) => {
			globalThis.__simplexOpenedUrls.push(url)
		}
	})

	const documentResponse = page.waitForResponse(
		(response) => response.request().resourceType() === "document" && response.url().startsWith("simplex://local/"),
	)
	await page.reload()
	const csp = await (await documentResponse).headerValue("content-security-policy")
	assert.match(csp ?? "", /default-src 'none'/)
	assert.match(csp ?? "", /script-src 'self'/)
	assert.match(csp ?? "", /connect-src 'self'/)
	assert.match(csp ?? "", /style-src 'self' 'unsafe-inline'/)
	assert.doesNotMatch(csp ?? "", /unsafe-eval/)
	assert.deepEqual(
		await page.evaluate(() => globalThis.__simplexCspViolations ?? []),
		[],
		"the operator page must not violate its CSP during load",
	)

	const inlineScriptRan = await page.evaluate(async () => {
		delete globalThis.__simplexInlineScriptRan
		const script = document.createElement("script")
		script.textContent = "globalThis.__simplexInlineScriptRan = true"
		document.head.append(script)
		await new Promise((resolveDelay) => setTimeout(resolveDelay, 25))
		return globalThis.__simplexInlineScriptRan === true
	})
	assert.equal(inlineScriptRan, false, "the CSP must reject injected inline scripts")

	const externalConnectSucceeded = await page.evaluate(async () => {
		try {
			await fetch("data:text/plain,not-simplex")
			return true
		} catch {
			return false
		}
	})
	assert.equal(externalConnectSucceeded, false, "the renderer must not connect outside simplex://local")

	const maskedConfig = await page.evaluate(async () => (await fetch("/api/config")).text())
	assert.doesNotMatch(
		maskedConfig,
		new RegExp(`${FIXTURE_KEY}|${FIXTURE_SEED}`),
		"the operator config response must not expose key material",
	)
	const futureWindowGuards = await electronApp.evaluate(({ BrowserWindow }) => {
		const attacker = new BrowserWindow({
			show: false,
			webPreferences: { contextIsolation: true, nodeIntegration: false, sandbox: true },
		})
		try {
			return {
				navigate: attacker.webContents.listenerCount("will-navigate"),
				frameNavigate: attacker.webContents.listenerCount("will-frame-navigate"),
				redirect: attacker.webContents.listenerCount("will-redirect"),
				webview: attacker.webContents.listenerCount("will-attach-webview"),
			}
		} finally {
			attacker.destroy()
		}
	})
	assert.deepEqual(futureWindowGuards, { navigate: 1, frameNavigate: 1, redirect: 1, webview: 1 })
	assert.equal(fixture.pauseWrites(), 0, "an untrusted page must not mutate the operator API")

	const approvedLink = `${externalLinks.hyperfxApp}/history/details/?id=1`
	const secondApprovedLink = `${externalLinks.chainExplorers.ethereum}/tx/0x1`
	await page.evaluate((url) => window.open(url, "_blank"), approvedLink)
	await waitFor(
		async () => (await electronApp.evaluate(() => globalThis.__simplexOpenedUrls ?? [])).length === 1,
		"approved external link",
	)
	assert.deepEqual(await electronApp.evaluate(() => globalThis.__simplexOpenedUrls), [approvedLink])
	assert.equal(electronApp.windows().length, 1, "target=_blank must not create an Electron window")

	await page.evaluate(() => window.open("https://example.com/", "_blank"))
	await page.evaluate((url) => window.open(url, "_blank"), secondApprovedLink)
	await waitFor(
		async () => (await electronApp.evaluate(() => globalThis.__simplexOpenedUrls ?? [])).length === 2,
		"second approved external link",
	)
	assert.deepEqual(await electronApp.evaluate(() => globalThis.__simplexOpenedUrls), [
		approvedLink,
		secondApprovedLink,
	])
	await page.evaluate(() => {
		window.location.href = "https://example.com/"
	})
	await delay(100)
	assert.match(page.url(), /^simplex:\/\/local\//, "external navigation must leave the desktop document in place")
})

test("first run writes a valid private config under Electron userData", async (t) => {
	const userDataDir = await temporaryUserData("first-run")
	const socketPath = socketPathFor(userDataDir)
	const configPath = join(userDataDir, "filler-config.toml")
	let electronApp
	t.after(async () => cleanupDesktop(electronApp, userDataDir))

	let page
	;({ electronApp, page } = await launchDesktop(userDataDir))
	await page.addInitScript(() => {
		globalThis.__simplexCspViolations = []
		window.addEventListener("securitypolicyviolation", (event) => {
			globalThis.__simplexCspViolations.push({ directive: event.effectiveDirective, blocked: event.blockedURI })
		})
	})
	await page.reload()
	await waitForHealth(socketPath, "init")
	await page.locator(".wizard-shell").waitFor({ timeout: 30_000 })
	assert.deepEqual(
		await page.evaluate(() => globalThis.__simplexCspViolations ?? []),
		[],
		"the setup wizard must not violate its CSP during load",
	)
	const [daemonPid] = await waitForDaemonPids(userDataDir)

	const config = {
		simplex: {
			signer: { type: "privateKey", key: TEST_KEY },
			maxConcurrentOrders: 5,
			substratePrivateKey: TEST_SEED,
			hyperbridgeWsUrl: "ws://127.0.0.1:9",
		},
		pairs: [
			{
				token0: "USDC",
				token1: "USDC",
				maxOrderSize: "100000",
				askPriceCurve: [
					{ amount: "100", price: "0.99" },
					{ amount: "100000", price: "0.999" },
				],
			},
		],
		chains: [{ rpcUrls: ["http://127.0.0.1:9"], bundlerUrl: "http://127.0.0.1:9" }],
	}
	const result = await page.evaluate(async (body) => {
		const response = await fetch("/api/setup/save-and-start", {
			method: "POST",
			headers: { "Content-Type": "application/json", "X-Simplex-UI": "1" },
			body: JSON.stringify({ config: body }),
		})
		return { status: response.status, json: await response.json() }
	}, config)

	assert.equal(result.status, 202)
	assert.equal(result.json.configPath, configPath)
	await waitFor(() => existsSync(configPath), "first-run config file")
	const written = await readFile(configPath, "utf8")
	assert.match(written, /WARNING: contains secrets/)
	assert.match(written, /\[simplex\.signer\]/)
	assert.match(written, /\[\[pairs\]\]/)
	assert.equal(written.match(new RegExp(TEST_KEY, "g"))?.length, 1)
	if (process.platform !== "win32") assert.equal((await stat(configPath)).mode & 0o777, 0o600)
	await delay(250)
	await assertNoTcpListener(daemonPid)
	const rendererStorage = await page.evaluate(() => ({
		local: { ...localStorage },
		session: { ...sessionStorage },
	}))
	assert.doesNotMatch(JSON.stringify(rendererStorage), new RegExp(TEST_KEY))
	assert.doesNotMatch(JSON.stringify(rendererStorage), new RegExp(TEST_SEED))
	await quitElectron(electronApp)
	electronApp = undefined
	await waitForHealth(socketPath, "init")
	await assertSecretsExistOnlyInConfig(userDataDir, configPath, [TEST_KEY, TEST_SEED])
})
