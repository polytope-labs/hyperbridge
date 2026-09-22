import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import type { NotificationSettings as AlertSettings, NotificationStatus, StoredPushSubscription } from "../types"

type DeviceState = "checking" | "desktop" | "unsupported" | "unavailable" | "denied" | "enabled" | "disabled"

function applicationServerKey(value: string): Uint8Array<ArrayBuffer> {
	const padding = "=".repeat((4 - (value.length % 4)) % 4)
	const binary = atob((value + padding).replace(/-/g, "+").replace(/_/g, "/"))
	const bytes = new Uint8Array(new ArrayBuffer(binary.length))
	for (let index = 0; index < binary.length; index++) bytes[index] = binary.charCodeAt(index)
	return bytes
}

function storedSubscription(subscription: PushSubscription): StoredPushSubscription {
	const json = subscription.toJSON()
	if (!json.endpoint || !json.keys?.p256dh || !json.keys.auth)
		throw new Error("The browser returned an incomplete subscription")
	return {
		endpoint: json.endpoint,
		expirationTime: json.expirationTime,
		keys: { p256dh: json.keys.p256dh, auth: json.keys.auth },
	}
}

function sameApplicationServerKey(subscription: PushSubscription, expected: Uint8Array<ArrayBuffer>): boolean {
	const current = subscription.options.applicationServerKey
	if (!current) return false
	const bytes = new Uint8Array(current)
	return bytes.length === expected.length && bytes.every((value, index) => value === expected[index])
}

async function browserRegistration(): Promise<ServiceWorkerRegistration> {
	let timeout: number | undefined
	try {
		return await Promise.race([
			navigator.serviceWorker.ready,
			new Promise<never>((_, reject) => {
				timeout = window.setTimeout(
					() => reject(new Error("The notification service worker did not become ready")),
					10_000,
				)
			}),
		])
	} finally {
		if (timeout !== undefined) window.clearTimeout(timeout)
	}
}

async function browserSubscription(registration?: ServiceWorkerRegistration): Promise<PushSubscription | null> {
	registration ??= await browserRegistration()
	return registration.pushManager.getSubscription()
}

export function NotificationSettings() {
	const [status, setStatus] = useState<NotificationStatus>()
	const [threshold, setThreshold] = useState("")
	const [swaps, setSwaps] = useState(false)
	const [device, setDevice] = useState<DeviceState>("checking")
	const [pending, setPending] = useState(false)
	const [message, setMessage] = useState<string>()
	const [error, setError] = useState<string>()
	const desktop = window.location.protocol === "simplex:"

	const load = useCallback(async () => {
		const next = await api.get<NotificationStatus>("/api/notifications")
		setStatus(next)
		setThreshold(next.settings.lowLiquidityThresholdUsd?.toString() ?? "")
		setSwaps(next.settings.swaps)
		if (desktop) return setDevice("desktop")
		if (!("Notification" in window) || !("serviceWorker" in navigator) || !("PushManager" in window)) {
			return setDevice("unsupported")
		}
		if (Notification.permission === "denied") return setDevice("denied")
		const registration = await browserRegistration()
		const subscription = await browserSubscription(registration)
		if (!subscription) return setDevice("disabled")
		const key = applicationServerKey(next.vapidPublicKey)
		if (!sameApplicationServerKey(subscription, key)) {
			await api.del("/api/notifications/subscription", { endpoint: subscription.endpoint })
			await subscription.unsubscribe()
			return setDevice("disabled")
		}
		// Browser state can outlive the solver database. Re-registering the same
		// endpoint makes the visible "enabled" state truthful after recovery.
		setStatus(
			await api.post<NotificationStatus>("/api/notifications/subscription", storedSubscription(subscription)),
		)
		setDevice("enabled")
	}, [desktop])

	useEffect(() => {
		void load().catch((reason) => {
			setDevice("unavailable")
			setError(reason instanceof Error ? reason.message : String(reason))
		})
	}, [load])

	const perform = async (action: () => Promise<void>, success: string) => {
		setPending(true)
		setMessage(undefined)
		setError(undefined)
		try {
			await action()
			setMessage(success)
		} catch (reason) {
			setError(reason instanceof Error ? reason.message : String(reason))
		} finally {
			setPending(false)
		}
	}

	const save = () =>
		perform(async () => {
			const parsed = threshold.trim() === "" ? null : Number(threshold)
			if (parsed !== null && (!Number.isFinite(parsed) || parsed <= 0)) {
				throw new Error("Enter a positive USD threshold or leave it blank")
			}
			const settings: AlertSettings = { lowLiquidityThresholdUsd: parsed, swaps }
			setStatus(await api.put<NotificationStatus>("/api/notifications", settings))
		}, "Alert rules saved")

	const enableDevice = () =>
		perform(async () => {
			const permission = await Notification.requestPermission()
			if (permission !== "granted") {
				setDevice(permission === "denied" ? "denied" : "disabled")
				throw new Error("Notification permission was not granted")
			}
			const registration = await browserRegistration()
			const key = applicationServerKey(status!.vapidPublicKey)
			let subscription = await registration.pushManager.getSubscription()
			if (subscription && !sameApplicationServerKey(subscription, key)) {
				await api.del("/api/notifications/subscription", { endpoint: subscription.endpoint })
				await subscription.unsubscribe()
				subscription = null
			}
			subscription ??= await registration.pushManager.subscribe({
				userVisibleOnly: true,
				applicationServerKey: key,
			})
			setStatus(
				await api.post<NotificationStatus>("/api/notifications/subscription", storedSubscription(subscription)),
			)
			setDevice("enabled")
		}, "Notifications enabled on this device")

	const disableDevice = () =>
		perform(async () => {
			const subscription = await browserSubscription()
			if (subscription) {
				setStatus(
					await api.del<NotificationStatus>("/api/notifications/subscription", {
						endpoint: subscription.endpoint,
					}),
				)
				await subscription.unsubscribe()
			}
			setDevice("disabled")
		}, "Notifications disabled on this device")

	const test = () =>
		perform(async () => {
			const subscription = desktop ? null : await browserSubscription()
			if (!desktop && !subscription) throw new Error("This browser is no longer subscribed")
			await api.post(
				"/api/notifications/test",
				subscription ? { endpoint: subscription.endpoint } : { native: true },
			)
		}, "Test notification sent")

	return (
		<div className="operator-panel-form notification-settings">
			<h2>Alert rules</h2>
			<p className="hint">
				Rules run in the Simplex solver, so alerts can arrive while this dashboard is closed. A low-liquidity
				alert fires once when available stablecoin liquidity crosses below the threshold and rearms after
				recovery.
			</p>

			<label className="field">
				<span>Low-liquidity threshold (USD)</span>
				<input
					type="number"
					min="0.01"
					step="0.01"
					placeholder="Disabled"
					value={threshold}
					onChange={(event) => setThreshold(event.target.value)}
				/>
				<small>Leave blank to disable low-liquidity alerts.</small>
			</label>

			<label className="checkbox-row">
				<input type="checkbox" checked={swaps} onChange={(event) => setSwaps(event.target.checked)} />
				<span>
					<strong>Completed swaps</strong>
					<small>Notify after this solver fills an order on-chain.</small>
				</span>
			</label>

			<button type="button" className="primary" disabled={pending || !status} onClick={() => void save()}>
				Save alert rules
			</button>

			<div className="notification-device">
				<h2>This device</h2>
				{device === "desktop" ? (
					<p className="hint">The installed Simplex desktop app delivers native alerts.</p>
				) : null}
				{device === "unsupported" ? (
					<p className="error">This browser does not support Web Push notifications.</p>
				) : null}
				{device === "unavailable" ? <p className="error">Web Push is unavailable on this device.</p> : null}
				{device === "denied" ? (
					<p className="error">
						Notifications are blocked. Allow them in this site’s browser settings, then reopen this panel.
					</p>
				) : null}
				{device === "disabled" ? (
					<button
						type="button"
						className="primary"
						disabled={pending || !status}
						onClick={() => void enableDevice()}
					>
						Enable on this device
					</button>
				) : null}
				{device === "enabled" ? (
					<button type="button" className="secondary" disabled={pending} onClick={() => void disableDevice()}>
						Disable on this device
					</button>
				) : null}
				{device === "desktop" || device === "enabled" ? (
					<button type="button" className="secondary" disabled={pending} onClick={() => void test()}>
						Send test notification
					</button>
				) : null}
				{status ? (
					<small>
						{status.subscriptionCount} browser device{status.subscriptionCount === 1 ? "" : "s"} subscribed
					</small>
				) : null}
			</div>

			{message ? <p className="text-ok">✓ {message}</p> : null}
			{error ? <p className="error">{error}</p> : null}
		</div>
	)
}
