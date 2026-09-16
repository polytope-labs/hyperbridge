import type { Session, WebContents } from "electron"
import { describe, expect, it, vi } from "vitest"
import externalLinks from "../../../simplex/src/config/external-links.json"
import {
	approvedExternalUrl,
	installSessionSecurity,
	installWebContentsSecurity,
	isTrustedRendererUrl,
	RENDERER_CSP,
	rendererWebPreferences,
	responseHeadersWithCsp,
} from "../desktop-security"

describe("desktop renderer security", () => {
	it("trusts only the simplex local authority", () => {
		expect(isTrustedRendererUrl("simplex://local/")).toBe(true)
		expect(isTrustedRendererUrl("simplex://local/orders?id=1#activity")).toBe(true)
		expect(isTrustedRendererUrl("simplex://LOCAL/")).toBe(true)

		for (const url of [
			"https://local/",
			"simplex://remote/",
			"simplex://local.example/",
			"simplex://user@local/",
			"simplex://local:444/",
			"not a url",
		]) {
			expect(isTrustedRendererUrl(url), url).toBe(false)
		}
	})

	it("allows only known HTTPS destinations to leave Electron", () => {
		expect(approvedExternalUrl(`${externalLinks.hyperfxApp}/history/?id=1`)).toBe(
			`${externalLinks.hyperfxApp}/history/?id=1`,
		)
		for (const origin of [
			...Object.values(externalLinks.hyperbridgeExplorers),
			...Object.values(externalLinks.chainExplorers),
		]) {
			expect(approvedExternalUrl(`${origin}/test`), origin).toBe(`${origin}/test`)
		}

		for (const url of [
			"http://app.hyperfx.finance/",
			"https://app.hyperfx.finance.evil.example/",
			"https://user@app.hyperfx.finance/",
			"https://app.hyperfx.finance:8443/",
			"https://example.com/",
			"file:///tmp/private-key",
			"javascript:alert(1)",
		]) {
			expect(approvedExternalUrl(url), url).toBeUndefined()
		}
	})

	it("returns hardened preferences without a preload bridge", () => {
		expect(rendererWebPreferences(true)).toEqual({
			allowRunningInsecureContent: false,
			contextIsolation: true,
			devTools: false,
			navigateOnDragDrop: false,
			nodeIntegration: false,
			sandbox: true,
			webSecurity: true,
			webviewTag: false,
		})
		expect(rendererWebPreferences(false).devTools).toBe(true)
		expect(rendererWebPreferences(true)).not.toHaveProperty("preload")
	})

	it("replaces weak upstream CSP only on the desktop document", () => {
		const headers = responseHeadersWithCsp({
			"content-security-policy": ["frame-ancestors 'none'"],
			"X-Simplex": ["1"],
		})
		expect(headers).toEqual({
			"Content-Security-Policy": [RENDERER_CSP],
			"X-Simplex": ["1"],
		})
		expect(RENDERER_CSP).toContain("connect-src 'self'")
		expect(RENDERER_CSP).toContain("object-src 'none'")
		expect(RENDERER_CSP).toContain("script-src 'self'")
		expect(RENDERER_CSP).toContain("style-src 'self' 'unsafe-inline'")
		expect(RENDERER_CSP).not.toContain("'unsafe-eval'")
		expect(RENDERER_CSP).not.toContain("script-src 'unsafe-inline'")
	})

	it("denies permissions except clipboard writes from the trusted renderer", () => {
		let checkPermission: (...args: never[]) => boolean = () => false
		let requestPermission: (...args: never[]) => void = () => {}
		let headersReceived: (...args: never[]) => void = () => {}
		const desktopSession = {
			setPermissionCheckHandler: vi.fn((handler) => {
				checkPermission = handler
			}),
			setPermissionRequestHandler: vi.fn((handler) => {
				requestPermission = handler
			}),
			webRequest: {
				onHeadersReceived: vi.fn((handler) => {
					headersReceived = handler
				}),
			},
		} as unknown as Session
		installSessionSecurity(desktopSession)

		const trustedContents = {
			getURL: () => "simplex://local/",
			isDestroyed: () => false,
		} as WebContents
		const callback = vi.fn()
		expect(
			checkPermission(
				trustedContents as never,
				"clipboard-sanitized-write" as never,
				"simplex://local/" as never,
			),
		).toBe(true)
		expect(checkPermission(trustedContents as never, "clipboard-read" as never, "simplex://local/" as never)).toBe(
			false,
		)
		expect(
			checkPermission(
				trustedContents as never,
				"clipboard-sanitized-write" as never,
				"https://example.com/" as never,
			),
		).toBe(false)
		requestPermission(
			trustedContents as never,
			"clipboard-sanitized-write" as never,
			callback as never,
			{ requestingUrl: "simplex://local/" } as never,
		)
		expect(callback).toHaveBeenCalledWith(true)

		const cspCallback = vi.fn()
		headersReceived(
			{
				resourceType: "mainFrame",
				responseHeaders: { Existing: ["yes"] },
				url: "simplex://local/",
			} as never,
			cspCallback as never,
		)
		expect(cspCallback).toHaveBeenCalledWith({
			responseHeaders: { "Content-Security-Policy": [RENDERER_CSP], Existing: ["yes"] },
		})
		cspCallback.mockClear()
		headersReceived({ resourceType: "script", url: "simplex://local/assets/app.js" } as never, cspCallback as never)
		expect(cspCallback).toHaveBeenCalledWith({})
	})

	it("blocks navigation and denies every child window", async () => {
		const listeners = new Map<string, (event: { url: string; preventDefault: () => void }) => void>()
		let windowHandler: (details: { url: string }) => { action: string } = () => ({ action: "allow" })
		const webContents = {
			on: vi.fn((event, listener) => {
				listeners.set(event, listener)
			}),
			setWindowOpenHandler: vi.fn((handler) => {
				windowHandler = handler
			}),
		} as unknown as WebContents
		const openExternal = vi.fn(async () => {})
		const onOpenError = vi.fn()
		installWebContentsSecurity(webContents, openExternal, onOpenError)

		for (const eventName of ["will-navigate", "will-frame-navigate", "will-redirect"]) {
			const trusted = { url: "simplex://local/orders", preventDefault: vi.fn() }
			listeners.get(eventName)?.(trusted)
			expect(trusted.preventDefault).not.toHaveBeenCalled()

			const external = { url: "https://example.com/", preventDefault: vi.fn() }
			listeners.get(eventName)?.(external)
			expect(external.preventDefault).toHaveBeenCalledOnce()
		}
		const webview = { url: "simplex://local/", preventDefault: vi.fn() }
		listeners.get("will-attach-webview")?.(webview)
		expect(webview.preventDefault).toHaveBeenCalledOnce()

		expect(windowHandler({ url: `${externalLinks.hyperfxApp}/` })).toEqual({ action: "deny" })
		await vi.waitFor(() => expect(openExternal).toHaveBeenCalledWith(`${externalLinks.hyperfxApp}/`))
		expect(windowHandler({ url: "https://example.com/" })).toEqual({ action: "deny" })
		expect(openExternal).toHaveBeenCalledOnce()
		expect(onOpenError).not.toHaveBeenCalled()
	})
})
