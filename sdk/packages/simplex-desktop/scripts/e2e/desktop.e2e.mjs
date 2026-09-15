import assert from "node:assert/strict"
import { execFile } from "node:child_process"
import { createRequire } from "node:module"
import { existsSync } from "node:fs"
import { mkdtemp, readdir, readFile, readlink, realpath, rm, stat } from "node:fs/promises"
import { request as httpRequest } from "node:http"
import { tmpdir } from "node:os"
import { dirname, join, resolve } from "node:path"
import { fileURLToPath } from "node:url"
import test from "node:test"
import { promisify } from "node:util"
import { _electron } from "playwright-core"
import { ActivityRecorder } from "../../../simplex/src/data/recorder.ts"
import { MemoryDataStore } from "../../../simplex/src/data/memory.ts"
import { UiServer } from "../../../simplex/src/services/server/UiServer.ts"
import { socketPathFor } from "../../src/desktop-paths.ts"

const execFileAsync = promisify(execFile)
const require = createRequire(import.meta.url)
const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "../..")
const simplexRoot = resolve(packageRoot, "../simplex")
const electronExecutable = require("electron")
const TEST_KEY = "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"

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

async function launchDesktop(userDataDir) {
	const electronApp = await _electron.launch({
		executablePath: electronExecutable,
		args: [packageRoot, `--user-data-dir=${userDataDir}`],
		cwd: packageRoot,
		env: { ...process.env, ELECTRON_DISABLE_SECURITY_WARNINGS: "true" },
		timeout: 120_000,
	})
	const page = await electronApp.firstWindow({ timeout: 120_000 })
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

async function assertNoElectronDescriptors(pid) {
	// libuv on Linux forks without closing descriptors that lack close-on-exec,
	// and Electron's main process leaves Chromium's open that way. Its resource
	// files are the ones only Electron opens, so any of them here means the
	// solver inherited Electron's descriptor table.
	if (process.platform !== "linux") return
	const electronDir = dirname(electronExecutable)
	const inherited = []
	for (const fd of await readdir(`/proc/${pid}/fd`)) {
		const target = await readlink(`/proc/${pid}/fd/${fd}`).catch(() => "")
		if (target.startsWith(electronDir)) inherited.push(`${fd} -> ${target}`)
	}
	assert.deepEqual(inherited, [], `daemon ${pid} must not hold Electron's descriptors`)
}

async function killProcess(pid) {
	try {
		if (process.platform === "win32") await execFileAsync("taskkill.exe", ["/PID", String(pid), "/T", "/F"])
		else process.kill(pid, "SIGTERM")
	} catch (error) {
		if (error.code !== "ESRCH") throw error
	}
}

// Playwright's close() and "close" event wait for Electron's stdio pipes to
// close, not for the process to exit. On Windows the detached solver can inherit
// those handles (libuv always spawns with handle inheritance) and keep them open
// while it runs. Wait for the process itself instead.
function electronExit(electronApp, timeoutMs = 30_000) {
	const child = electronApp.process()
	if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve()
	return new Promise((resolveExit, reject) => {
		const timer = setTimeout(() => reject(new Error(`Electron ${child.pid} did not exit`)), timeoutMs)
		child.once("exit", () => {
			clearTimeout(timer)
			resolveExit()
		})
	})
}

async function quitElectron(electronApp) {
	const exited = electronExit(electronApp)
	// The inspector connection drops as the app quits, which can reject this call.
	await electronApp.evaluate(({ app }) => app.quit()).catch(() => {})
	await exited
}

async function hardKillElectron(electronApp) {
	const exited = electronExit(electronApp)
	const child = electronApp.process()
	if (process.platform === "win32") await execFileAsync("taskkill.exe", ["/PID", String(child.pid), "/F"])
	else child.kill("SIGKILL")
	await exited
}

async function cleanupDesktop(electronApp, userDataDir) {
	if (electronApp) {
		try {
			await hardKillElectron(electronApp)
		} catch {
			// A test that already quit or killed Electron leaves nothing to stop.
		}
	}
	for (const pid of await daemonPids(userDataDir)) await killProcess(pid)
	await waitFor(async () => (await daemonPids(userDataDir)).length === 0, "detached daemon cleanup").catch(() => {})
	await rm(userDataDir, { recursive: true, force: true })
}

function operatorFixture(socketPath) {
	const data = new MemoryDataStore()
	const activity = new ActivityRecorder(data.activity)
	const originalOrderHistory = activity.orderHistory.bind(activity)
	let historyReads = 0
	activity.orderHistory = (...args) => {
		historyReads += 1
		return originalOrderHistory(...args)
	}
	const config = {
		simplex: {
			signer: { type: "privateKey", key: "0xab" },
			substratePrivateKey: "seed",
			hyperbridgeWsUrl: "wss://example.invalid",
		},
		pairs: [],
		chains: [],
	}
	const operator = {
		strategies: [],
		filler: { pause() {}, resume() {}, isPaused: () => false, getWatchOnly: () => ({}) },
		balances: { getSnapshot: () => ({ updatedAt: null, status: "loading", chains: [], issues: [] }) },
		haltControls: [],
		config,
		stop: async () => {},
		activity,
		bids: data.bids,
		setPaused: async () => {},
		setLogLevel() {},
		applyAllowlist() {},
		applyRebalancing() {},
		version: "0.0.0-test",
		startedAt: Date.now(),
		configPath: join(dirname(socketPath), "filler-config.toml"),
		chains: [],
		strategyTypes: [],
	}
	const server = new UiServer({ mode: "operator", uiDistDir: join(simplexRoot, "dist/ui"), operator })
	return {
		server,
		activity,
		start: () => server.start({ socketPath }),
		stop: () => server.stop(),
		historyReads: () => historyReads,
		clientCount: () => server.sseClients.size,
	}
}

test("Electron survives a hard close, reattaches, and never opens a TCP listener", async (t) => {
	const userDataDir = await temporaryUserData("lifecycle")
	const socketPath = socketPathFor(userDataDir)
	let electronApp
	t.after(async () => cleanupDesktop(electronApp, userDataDir))
	;({ electronApp } = await launchDesktop(userDataDir))
	await waitForHealth(socketPath, "init")
	const [daemonPid] = await waitForDaemonPids(userDataDir)
	await assertNoElectronDescriptors(daemonPid)
	await assertNoTcpListener(daemonPid)

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

test("first run writes a valid private config under Electron userData", async (t) => {
	const userDataDir = await temporaryUserData("first-run")
	const socketPath = socketPathFor(userDataDir)
	const configPath = join(userDataDir, "filler-config.toml")
	let electronApp
	t.after(async () => cleanupDesktop(electronApp, userDataDir))

	let page
	;({ electronApp, page } = await launchDesktop(userDataDir))
	await waitForHealth(socketPath, "init")
	await page.locator(".wizard-shell").waitFor({ timeout: 30_000 })
	await waitForDaemonPids(userDataDir)

	const config = {
		simplex: {
			signer: { type: "privateKey", key: TEST_KEY },
			maxConcurrentOrders: 5,
			substratePrivateKey: "bottom drive obey lake curtain smoke basket hold race lonely fit walk",
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
	assert.match(written, /\[simplex\.signer\]/)
	assert.match(written, /\[\[pairs\]\]/)
	if (process.platform !== "win32") assert.equal((await stat(configPath)).mode & 0o777, 0o600)
})
