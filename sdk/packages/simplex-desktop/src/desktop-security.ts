import type { Session, WebContents, WebPreferences } from "electron"

const TRUSTED_PROTOCOL = "simplex:"
const TRUSTED_HOST = "local"
const CLIPBOARD_WRITE_PERMISSION = "clipboard-sanitized-write"

// This is intentionally an exact allowlist, not a suffix check. Keep it in sync
// with the fixed explorer and HyperFX links compiled into the existing SPA.
const EXTERNAL_HOSTS = new Set([
	"amoy.polygonscan.com",
	"app.hyperfx.finance",
	"arbiscan.io",
	"basescan.org",
	"bscscan.com",
	"etherscan.io",
	"gargantua.statescan.io",
	"nexus.statescan.io",
	"polygonscan.com",
	"sepolia.arbiscan.io",
	"sepolia.basescan.org",
	"sepolia.etherscan.io",
	"testnet.bscscan.com",
])

export const RENDERER_CSP = [
	"default-src 'none'",
	"base-uri 'none'",
	"connect-src 'self'",
	"font-src 'self'",
	"form-action 'self'",
	"frame-ancestors 'none'",
	"frame-src 'none'",
	"img-src 'self' data:",
	"manifest-src 'self'",
	"media-src 'none'",
	"object-src 'none'",
	"script-src 'self'",
	"style-src 'self'",
	"style-src-attr 'unsafe-inline'",
	"worker-src 'self'",
].join("; ")

export function isTrustedRendererUrl(rawUrl: string): boolean {
	try {
		const url = new URL(rawUrl)
		return (
			url.protocol === TRUSTED_PROTOCOL &&
			url.hostname.toLowerCase() === TRUSTED_HOST &&
			url.port === "" &&
			url.username === "" &&
			url.password === ""
		)
	} catch {
		return false
	}
}

export function approvedExternalUrl(rawUrl: string): string | undefined {
	try {
		const url = new URL(rawUrl)
		if (
			url.protocol !== "https:" ||
			url.port !== "" ||
			url.username !== "" ||
			url.password !== "" ||
			!EXTERNAL_HOSTS.has(url.hostname)
		) {
			return undefined
		}
		return url.href
	} catch {
		return undefined
	}
}

export function rendererWebPreferences(isPackaged: boolean): WebPreferences {
	return {
		allowRunningInsecureContent: false,
		contextIsolation: true,
		devTools: !isPackaged,
		navigateOnDragDrop: false,
		nodeIntegration: false,
		sandbox: true,
		webSecurity: true,
		webviewTag: false,
	}
}

export function responseHeadersWithCsp(
	responseHeaders: Record<string, string[]> | undefined,
): Record<string, string[]> {
	const headers = Object.fromEntries(
		Object.entries(responseHeaders ?? {}).filter(([name]) => name.toLowerCase() !== "content-security-policy"),
	)
	headers["Content-Security-Policy"] = [RENDERER_CSP]
	return headers
}

function canWriteClipboard(webContents: WebContents | null, requestingUrl: string): boolean {
	return Boolean(
		webContents &&
			!webContents.isDestroyed() &&
			isTrustedRendererUrl(webContents.getURL()) &&
			isTrustedRendererUrl(requestingUrl),
	)
}

export function installSessionSecurity(session: Session): void {
	session.setPermissionCheckHandler((webContents, permission, requestingOrigin) => {
		return permission === CLIPBOARD_WRITE_PERMISSION && canWriteClipboard(webContents, requestingOrigin)
	})
	session.setPermissionRequestHandler((webContents, permission, callback, details) => {
		callback(permission === CLIPBOARD_WRITE_PERMISSION && canWriteClipboard(webContents, details.requestingUrl))
	})
	session.webRequest.onHeadersReceived((details, callback) => {
		if (details.resourceType === "mainFrame" && isTrustedRendererUrl(details.url)) {
			callback({ responseHeaders: responseHeadersWithCsp(details.responseHeaders) })
			return
		}
		callback({})
	})
}

export function installWebContentsSecurity(
	webContents: WebContents,
	openExternal: (url: string) => Promise<unknown>,
	onOpenError: () => void,
): void {
	const preventUntrustedNavigation = (event: Electron.Event<{ url: string }>) => {
		if (!isTrustedRendererUrl(event.url)) event.preventDefault()
	}
	webContents.on("will-frame-navigate", preventUntrustedNavigation)
	webContents.on("will-navigate", preventUntrustedNavigation)
	webContents.on("will-redirect", preventUntrustedNavigation)
	webContents.on("will-attach-webview", (event) => event.preventDefault())
	webContents.setWindowOpenHandler(({ url }) => {
		const externalUrl = approvedExternalUrl(url)
		if (externalUrl) void openExternal(externalUrl).catch(onOpenError)
		return { action: "deny" }
	})
}
