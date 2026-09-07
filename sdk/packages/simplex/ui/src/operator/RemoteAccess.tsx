import QRCode from "qrcode"
import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import { Field } from "../components/Field"
import { useAction, usePolling } from "../lib/hooks"
import type { TunnelNewDeviceDto, TunnelStatusDto } from "../types"

const STATE_LABEL: Record<TunnelStatusDto["state"], string> = {
	disabled: "Off",
	connecting: "Connecting…",
	connected: "Connected",
	reconnecting: "Reconnecting…",
	disconnected: "Disconnected",
	error: "Error",
}

/**
 * Remote access panel: turns the relay tunnel on or off, pairs phones, and
 * shows what a phone needs to connect. Nothing here touches funds directly,
 * but a paired device key opens the whole UI, so the private key is shown
 * exactly once and never stored.
 */
export function RemoteAccess() {
	const [status, setStatus] = useState<TunnelStatusDto>()
	const [unavailable, setUnavailable] = useState(false)
	const [relayDraft, setRelayDraft] = useState<string>()
	const [label, setLabel] = useState("")
	const [publicKey, setPublicKey] = useState("")
	// Paste is the default: the phone's app makes the key and the private half
	// never leaves it. Generating here is the fallback for apps that cannot.
	const [mode, setMode] = useState<"paste" | "generate">("paste")
	const [fresh, setFresh] = useState<TunnelNewDeviceDto>()
	const { run: act, message, error, isPending } = useAction()

	const load = useCallback(async () => {
		try {
			const next = await api.get<TunnelStatusDto>("/api/tunnel")
			setStatus(next)
			setRelayDraft((draft) => draft ?? next.relay)
		} catch (err) {
			if ((err as { status?: number }).status === 404) setUnavailable(true)
			else throw err
		}
	}, [])
	usePolling(
		useCallback(() => act(load, undefined, "poll"), [act, load]),
		3000,
	)

	if (unavailable) {
		return <p className="hint">Remote access is not available for this filler.</p>
	}
	if (!status) return <p className="hint">Loading…</p>

	const relayChanged = relayDraft !== undefined && relayDraft.trim() !== status.relay

	return (
		<div className="operator-panel-form">
			<h2>Remote access</h2>
			<p className="hint">
				Opens an outbound SSH tunnel to a relay so your phone's SSH client can reach this UI from anywhere. The
				relay only carries encrypted bytes: your phone's session terminates here, in Simplex.
			</p>

			<section className="tunnel-status">
				<div className="row">
					<strong>Status</strong>
					<span className={`pill tunnel-state-${status.state}`}>{STATE_LABEL[status.state]}</span>
				</div>
				{status.port ? (
					<div className="row">
						<span>Public endpoint</span>
						<CopyHash
							value={`${status.relay.split(":")[0]}:${status.port}`}
							chars={64}
							copyLabel="Copy endpoint"
						/>
					</div>
				) : null}
				<div className="row">
					<span>Host key (pin this on your phone)</span>
					<CopyHash value={status.hostFingerprint} chars={18} copyLabel="Copy host key fingerprint" />
				</div>
				{status.relayFingerprint ? (
					<div className="row">
						<span>Relay host key</span>
						<CopyHash value={status.relayFingerprint} chars={18} copyLabel="Copy relay fingerprint" />
					</div>
				) : null}
				{status.activeConnections ? (
					<div className="row">
						<span>Open device sessions</span>
						<span>{status.activeConnections}</span>
					</div>
				) : null}
				{status.lastError ? <p className="error">{status.lastError}</p> : null}
			</section>

			<div className="row" style={{ marginTop: "1rem" }}>
				<label className="row" style={{ gap: "0.5rem" }}>
					<input
						type="checkbox"
						checked={status.enabled}
						disabled={isPending("toggle")}
						onChange={(e) =>
							act(
								async () => {
									setStatus(
										await api.put<TunnelStatusDto>("/api/tunnel", { enabled: e.target.checked }),
									)
								},
								e.target.checked ? "Remote access enabled" : "Remote access disabled",
								"toggle",
							)
						}
					/>
					<span>Enable remote access</span>
				</label>
			</div>

			<Field label="Relay" value={relayDraft ?? status.relay} onChange={setRelayDraft} placeholder="host:port" />
			{relayChanged ? (
				<div className="row">
					<button
						type="button"
						disabled={isPending("relay")}
						onClick={() =>
							act(
								async () => {
									setStatus(
										await api.put<TunnelStatusDto>("/api/tunnel", { relay: relayDraft!.trim() }),
									)
								},
								"Relay updated",
								"relay",
							)
						}
					>
						Save relay
					</button>
					<button type="button" onClick={() => setRelayDraft(status.relay)}>
						Cancel
					</button>
				</div>
			) : null}

			<h3 style={{ marginTop: "1.5rem" }}>Paired devices</h3>
			{status.devices.length === 0 ? <p className="hint">No devices yet. Pair one below.</p> : null}
			{status.devices.map((device) => (
				<div className="row" key={device.fingerprint} style={{ marginBottom: "0.35rem" }}>
					<span style={{ flex: 1 }}>
						{device.label}
						<br />
						<small className="mono">{device.fingerprint}</small>
					</span>
					<button
						type="button"
						aria-label={`Revoke ${device.label}`}
						onClick={() => {
							if (!window.confirm(`Revoke "${device.label}"? It will no longer be able to open this UI.`))
								return
							void act(async () => {
								await api.post("/api/tunnel/devices/revoke", { fingerprint: device.fingerprint })
								await load()
							}, "Device revoked")
						}}
					>
						Revoke
					</button>
				</div>
			))}

			<h3 style={{ marginTop: "1rem" }}>Pair a device</h3>
			<p className="hint">
				{mode === "paste"
					? "Create a key in the phone's SSH app, then paste its public key here. The private key stays on the phone."
					: "Simplex will generate a key pair and show the private key once, for apps that cannot create their own."}
			</p>
			<div className="row">
				<input
					type="text"
					aria-label="Device label"
					style={{ flex: 1 }}
					placeholder="e.g. Seun's iPhone"
					value={label}
					maxLength={64}
					onChange={(e) => setLabel(e.target.value)}
				/>
			</div>
			{mode === "paste" ? (
				<textarea
					aria-label="Device public key"
					className="mono"
					rows={3}
					style={{ width: "100%" }}
					placeholder="ssh-ed25519 AAAA… (the .pub line from the phone's SSH app)"
					value={publicKey}
					onChange={(e) => setPublicKey(e.target.value)}
				/>
			) : null}
			<div className="row">
				<button type="button" disabled={!canPair() || isPending("pair")} onClick={() => void pair()}>
					{mode === "paste" ? "Pair with this public key" : "Generate a key pair"}
				</button>
				<button type="button" onClick={() => setMode(mode === "paste" ? "generate" : "paste")}>
					{mode === "paste" ? "Generate a key pair instead" : "Paste a public key instead"}
				</button>
			</div>

			{fresh ? (
				<NewDevice
					device={fresh}
					onDone={() => {
						setFresh(undefined)
						void load()
					}}
				/>
			) : null}

			{message && <p className="hint">✓ {message}</p>}
			{error && <p className="error">{error}</p>}
		</div>
	)

	function canPair() {
		return Boolean(label.trim()) && (mode === "generate" || Boolean(publicKey.trim()))
	}

	function pair() {
		return act(
			async () => {
				const body =
					mode === "paste" ? { label: label.trim(), publicKey: publicKey.trim() } : { label: label.trim() }
				const created = await api.post<TunnelNewDeviceDto>("/api/tunnel/devices", body)
				setFresh(created)
				setLabel("")
				setPublicKey("")
			},
			undefined,
			"pair",
		)
	}
}

/** Everything the phone needs, shown once. */
function NewDevice(props: { device: TunnelNewDeviceDto; onDone: () => void }) {
	const { device } = props
	const [qr, setQr] = useState<string>()
	const [saved, setSaved] = useState(false)
	useEffect(() => {
		if (!device.privateKey) return
		let cancelled = false
		QRCode.toDataURL(device.privateKey, { errorCorrectionLevel: "M", margin: 1, width: 240 })
			.then((url) => {
				if (!cancelled) setQr(url)
			})
			.catch(() => setQr(undefined))
		return () => {
			cancelled = true
		}
	}, [device.privateKey])

	const endpoint = device.connection.port
		? `${device.connection.host}:${device.connection.port}`
		: `${device.connection.host}:<port shown here once the tunnel connects>`
	const command = [
		"ssh",
		"-i",
		"<key file>",
		"-N",
		"-L",
		device.connection.localForward,
		"-p",
		device.connection.port ? String(device.connection.port) : "<port>",
		`${device.connection.username}@${device.connection.host}`,
	].join(" ")

	return (
		<section className="tunnel-new-device" role="note">
			<h3>{device.device.label} is paired</h3>
			{device.privateKey ? (
				<>
					<p className="error">
						<strong>This private key is shown once and not stored.</strong> Anyone holding it can open this
						UI, including its Send and treasury tools. Keep it on the device; avoid key stores that sync to
						a cloud.
					</p>
					<div className="row" style={{ alignItems: "flex-start", gap: "1rem" }}>
						{qr ? <img src={qr} alt="Private key as a QR code" width={240} height={240} /> : null}
						<div style={{ flex: 1 }}>
							<textarea
								readOnly
								rows={8}
								className="mono"
								style={{ width: "100%" }}
								value={device.privateKey}
							/>
							<div className="row">
								<CopyHash value={device.privateKey} copyLabel="Copy private key">
									Copy private key
								</CopyHash>
							</div>
						</div>
					</div>
				</>
			) : (
				<p className="hint">
					The phone keeps its private key. Anyone holding it can open this UI, including its Send and treasury
					tools, so revoke this device here if the phone is lost.
				</p>
			)}
			<dl className="tunnel-connection">
				<dt>Host and port</dt>
				<dd className="mono">{endpoint}</dd>
				<dt>Username</dt>
				<dd className="mono">{device.connection.username}</dd>
				<dt>Host key fingerprint</dt>
				<dd className="mono">{device.connection.hostFingerprint}</dd>
				<dt>Local forward</dt>
				<dd className="mono">{device.connection.localForward}</dd>
			</dl>
			<p className="hint">
				In your SSH app, add the key, save a connection to the host and port above, and add a local port forward
				of <code>{device.connection.localForward}</code>. Then open <code>http://localhost:8686</code> in the
				phone's browser while connected. Equivalent command line:
			</p>
			<pre className="mono">{command}</pre>
			{device.privateKey ? (
				<label className="row" style={{ gap: "0.5rem" }}>
					<input type="checkbox" checked={saved} onChange={(e) => setSaved(e.target.checked)} />
					<span>I have saved the key on the device</span>
				</label>
			) : null}
			<button type="button" disabled={Boolean(device.privateKey) && !saved} onClick={props.onDone}>
				Done
			</button>
		</section>
	)
}
