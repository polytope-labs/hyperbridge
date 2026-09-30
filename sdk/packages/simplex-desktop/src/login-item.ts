import { existsSync, mkdirSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from "node:fs"
import { homedir } from "node:os"
import { dirname, join } from "node:path"

export const HIDDEN_LAUNCH_ARGUMENT = "--hidden"
const LINUX_AUTOSTART_MARKER = "X-Simplex-Managed=true"

/** AppImage mounts are ephemeral; its original image path is the stable login target. */
export function loginItemExecutable(
	platform: NodeJS.Platform,
	executable: string,
	appImage = process.env.APPIMAGE,
): string {
	return platform === "linux" && appImage ? appImage : executable
}

type LoginApp = {
	getLoginItemSettings: (options?: Electron.LoginItemSettingsOptions) => {
		openAtLogin: boolean
		wasOpenedAtLogin: boolean
	}
	setLoginItemSettings: (settings: Electron.Settings) => void
}

function quoteDesktopExecArgument(value: string): string {
	return `"${value
		.replace(/\\/g, "\\\\")
		.replace(/"/g, '\\"')
		.replace(/`/g, "\\`")
		.replace(/\$/g, "\\$")
		.replace(/%/g, "%%")}"`
}

export function linuxAutostartPath(configHome = process.env.XDG_CONFIG_HOME || join(homedir(), ".config")): string {
	return join(configHome, "autostart", "simplex.desktop")
}

export function linuxAutostartEntry(executable: string): string {
	return [
		"[Desktop Entry]",
		"Type=Application",
		"Version=1.0",
		"Name=Simplex",
		"Comment=Start the Simplex desktop supervisor",
		`Exec=${quoteDesktopExecArgument(executable)} ${HIDDEN_LAUNCH_ARGUMENT}`,
		"Terminal=false",
		"X-GNOME-Autostart-enabled=true",
		LINUX_AUTOSTART_MARKER,
		"",
	].join("\n")
}

export class LoginItemController {
	readonly supported: boolean
	private readonly windowsOptions: Electron.LoginItemSettingsOptions
	private readonly autostartPath: string

	constructor(
		private readonly options: {
			app: LoginApp
			platform: NodeJS.Platform
			isPackaged: boolean
			executable: string
			argv?: string[]
			linuxConfigHome?: string
		},
	) {
		this.supported = options.isPackaged && ["darwin", "linux", "win32"].includes(options.platform)
		this.windowsOptions = { path: options.executable, args: [HIDDEN_LAUNCH_ARGUMENT] }
		this.autostartPath = linuxAutostartPath(options.linuxConfigHome)
	}

	isEnabled(): boolean {
		if (!this.supported) return false
		if (this.options.platform === "linux") {
			try {
				return readFileSync(this.autostartPath, "utf8").includes(LINUX_AUTOSTART_MARKER)
			} catch {
				return false
			}
		}
		try {
			return this.options.app.getLoginItemSettings(
				this.options.platform === "win32" ? this.windowsOptions : undefined,
			).openAtLogin
		} catch {
			return false
		}
	}

	wasOpenedAtLogin(): boolean {
		if ((this.options.argv ?? process.argv).includes(HIDDEN_LAUNCH_ARGUMENT)) return true
		if (!this.supported || this.options.platform !== "darwin") return false
		try {
			return this.options.app.getLoginItemSettings().wasOpenedAtLogin
		} catch {
			return false
		}
	}

	setEnabled(enabled: boolean): void {
		if (!this.supported) throw new Error("Launch at login is available only in an installed Simplex app")
		if (this.options.platform !== "linux") {
			this.options.app.setLoginItemSettings({
				openAtLogin: enabled,
				...(this.options.platform === "win32" ? this.windowsOptions : {}),
			})
			return
		}

		if (!enabled) {
			if (!existsSync(this.autostartPath)) return
			const existing = readFileSync(this.autostartPath, "utf8")
			if (!existing.includes(LINUX_AUTOSTART_MARKER)) {
				throw new Error(`Refusing to remove an unmanaged autostart file at ${this.autostartPath}`)
			}
			rmSync(this.autostartPath)
			return
		}
		if (existsSync(this.autostartPath)) {
			const existing = readFileSync(this.autostartPath, "utf8")
			if (!existing.includes(LINUX_AUTOSTART_MARKER)) {
				throw new Error(`Refusing to replace an unmanaged autostart file at ${this.autostartPath}`)
			}
		}

		mkdirSync(dirname(this.autostartPath), { recursive: true })
		const temporary = `${this.autostartPath}.${process.pid}.tmp`
		try {
			writeFileSync(temporary, linuxAutostartEntry(this.options.executable), { encoding: "utf8", mode: 0o600 })
			renameSync(temporary, this.autostartPath)
		} finally {
			if (existsSync(temporary)) rmSync(temporary)
		}
	}
}

export function latestLogPath(dataDir: string): string | undefined {
	const logsDir = join(dataDir, "logs")
	try {
		const latest = readdirSync(logsDir)
			.filter((name) => /^simplex-[\dT-]+\.log$/.test(name))
			.sort()
			.at(-1)
		return latest ? join(logsDir, latest) : undefined
	} catch {
		return undefined
	}
}
