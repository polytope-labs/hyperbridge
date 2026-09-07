import QRCode from "qrcode"
import { useCallback, useEffect, useState } from "react"
import { api } from "../api"
import { CopyHash } from "../components/CopyHash"
import { PillTabs } from "../components/PillTabs"
import { useAction, usePolling } from "../lib/hooks"
import type { TunnelConnectionDto, TunnelNewDeviceDto, TunnelStatusDto } from "../types"

const STATE_LABEL: Record<TunnelStatusDto["state"], string> = {
	disabled: "Off",
	connecting: "Connecting",
	connected: "Connected",
	reconnecting: "Reconnecting",
	disconnected: "Disconnected",
	error: "Error",
}

const STATE_BADGE: Record<TunnelStatusDto["state"], string> = {
	disabled: "",
	connecting: "warn",
	connected: "ok",
	reconnecting: "warn",
	disconnected: "warn",
	error: "err",
}

type PairMode = "paste" | "generate"

/** The fields an operator types into their SSH app, ready to copy one at a time. */
function ConnectionFields(props: { connection: TunnelConnectionDto; enabled: boolean }) {
	const { connection, enabled } = props
	return (
		<dl className="tunnel-facts">
			<div>
				<dt>Host</dt>
				<dd>
					<CopyHash value={connection.host} chars={48} copyLabel="Copy host" />
				</dd>
			</div>
			<div>
				<dt>Port</dt>
				<dd>
					{connection.port ? (
						<CopyHash value={String(connection.port)} chars={8} copyLabel="Copy port" />
					) : (
						<span className="tunnel-facts-pending">
							{enabled ? "Assigned once connected" : "Turn remote access on"}
						</span>
					)}
				</dd>
			</div>
			<div>
				<dt>Username</dt>
				<dd className="mono">{connection.username}</dd>
			</div>
			<div>
				<dt>Host key fingerprint</dt>
				<dd>
					<CopyHash value={connection.hostFingerprint} chars={22} copyLabel="Copy host key fingerprint" />
				</dd>
			</div>
			<div>
				<dt>Local port forward</dt>
				<dd>
					<CopyHash value={connection.localForward} chars={32} copyLabel="Copy local forward" />
				</dd>
			</div>
		</dl>
	)
}

const PAIR_MODES: ReadonlyArray<{ value: PairMode; label: string }> = [
	{ value: "paste", label: "Paste a public key" },
	{ value: "generate", label: "Generate a key pair" },
]

/**
 * Remote access panel: turns the relay tunnel on or off, pairs phones, and
 * shows what a phone needs to connect. A paired device key opens the whole
 * UI, so a generated private key is shown exactly once and never stored.
 */
export function RemoteAccess() {
	const [status, setStatus] = useState<TunnelStatusDto>()
	const [unavailable, setUnavailable] = useState(false)
	const [label, setLabel] = useState("")
	const [publicKey, setPublicKey] = useState("")
	// Paste is the default: the phone's app makes the key and the private half
	// never leaves it. Generating here is the fallback for apps that cannot.
	const [mode, setMode] = useState<PairMode>("paste")
	const [fresh, setFresh] = useState<TunnelNewDeviceDto>()
	const { run: act, message, error, isPending } = useAction()

	const load = useCallback(async () => {
		try {
			setStatus(await api.get<TunnelStatusDto>("/api/tunnel"))
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
		return <p className="operator-empty">Remote access is not available for this filler.</p>
	}
	if (!status) return <p className="operator-empty">Loading…</p>

	const canPair = Boolean(label.trim()) && (mode === "generate" || Boolean(publicKey.trim()))

	const pair = () =>
		act(
			async () => {
				const body =
					mode === "paste" ? { label: label.trim(), publicKey: publicKey.trim() } : { label: label.trim() }
				setFresh(await api.post<TunnelNewDeviceDto>("/api/tunnel/devices", body))
				setLabel("")
				setPublicKey("")
			},
			undefined,
			"pair",
		)

	return (
		<div className="tunnel-panel">
			<section className="card tunnel-card">
				<div className="card-heading">
					<div>
						<span className="eyebrow">Tunnel</span>
						<h2>Relay connection</h2>
					</div>
					<span className={`badge ${STATE_BADGE[status.state]}`}>{STATE_LABEL[status.state]}</span>
				</div>
				<p className="hint">
					An outbound SSH tunnel to a relay gives this dashboard a public address your phone's SSH app can
					reach. The relay only carries encrypted bytes: your phone's session ends here, in Simplex.
				</p>

				<label className="chain-enable-toggle tunnel-toggle">
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
					<span className="chain-enable-switch" aria-hidden="true" />
					<span>{status.enabled ? "Remote access is on" : "Remote access is off"}</span>
				</label>

				<ConnectionFields connection={status.connection} enabled={status.enabled} />
				{status.lastError ? (
					<div className="operator-alert tunnel-alert">
						<div>
							<strong>Tunnel problem</strong>
							<p>{status.lastError}</p>
						</div>
					</div>
				) : null}
			</section>

			<section className="card tunnel-card">
				<div className="card-heading">
					<div>
						<span className="eyebrow">Devices</span>
						<h2>Paired devices</h2>
					</div>
					<small className="tunnel-count">
						{status.devices.length} paired
						{status.activeConnections ? ` · ${status.activeConnections} connected` : ""}
					</small>
				</div>
				{status.devices.length === 0 ? (
					<p className="operator-empty">No devices yet. Pair one below.</p>
				) : (
					<ul className="tunnel-device-list">
						{status.devices.map((device) => (
							<li key={device.fingerprint} className="tunnel-device">
								<div>
									<strong>{device.label}</strong>
									<small className="mono">{device.fingerprint}</small>
									{device.addedAt ? (
										<small>Paired {new Date(device.addedAt).toLocaleDateString()}</small>
									) : null}
								</div>
								<button
									type="button"
									className="secondary"
									aria-label={`Revoke ${device.label}`}
									onClick={() => {
										if (
											!window.confirm(
												`Revoke "${device.label}"? It will no longer be able to open this UI.`,
											)
										)
											return
										void act(async () => {
											await api.post("/api/tunnel/devices/revoke", {
												fingerprint: device.fingerprint,
											})
											await load()
										}, "Device revoked")
									}}
								>
									Revoke
								</button>
							</li>
						))}
					</ul>
				)}
			</section>

			{fresh ? (
				<NewDevice
					device={fresh}
					onDone={() => {
						setFresh(undefined)
						void load()
					}}
				/>
			) : (
				<section className="card tunnel-card">
					<div className="card-heading">
						<div>
							<span className="eyebrow">Pairing</span>
							<h2>Pair a device</h2>
						</div>
					</div>
					<PillTabs options={PAIR_MODES} value={mode} onChange={setMode} ariaLabel="Pairing method" />
					<p className="hint">
						{mode === "paste"
							? "Create a key in the phone's SSH app and paste its public key here. The private key never leaves the phone."
							: "Simplex generates the key pair and shows the private key once, for apps that cannot create their own."}
					</p>
					<label className="field">
						<span className="field-label">Device name</span>
						<input
							type="text"
							placeholder="e.g. Seun's iPhone"
							value={label}
							maxLength={64}
							onChange={(e) => setLabel(e.target.value)}
						/>
					</label>
					{mode === "paste" ? (
						<label className="field">
							<span className="field-label">Public key</span>
							<textarea
								className="mono tunnel-key-input"
								rows={3}
								placeholder="ssh-ed25519 AAAA… (the .pub line from the phone's SSH app)"
								value={publicKey}
								onChange={(e) => setPublicKey(e.target.value)}
							/>
						</label>
					) : null}
					<div className="tunnel-actions">
						<button
							type="button"
							className="primary"
							disabled={!canPair || isPending("pair")}
							onClick={() => void pair()}
						>
							{mode === "paste" ? "Pair device" : "Generate and pair"}
						</button>
					</div>
				</section>
			)}

			{message && <p className="hint">✓ {message}</p>}
			{error && <p className="error">{error}</p>}
		</div>
	)
}

/** Everything the phone needs, shown once. */
function NewDevice(props: { device: TunnelNewDeviceDto; onDone: () => void }) {
	const { device } = props
	const [qr, setQr] = useState<string>()
	const [saved, setSaved] = useState(false)
	useEffect(() => {
		if (!device.privateKey) return
		let cancelled = false
		QRCode.toDataURL(device.privateKey, { errorCorrectionLevel: "M", margin: 1, width: 220 })
			.then((url) => {
				if (!cancelled) setQr(url)
			})
			.catch(() => setQr(undefined))
		return () => {
			cancelled = true
		}
	}, [device.privateKey])

	const { connection } = device
	const command = [
		"ssh",
		"-i",
		"<key file>",
		"-N",
		"-L",
		connection.localForward,
		"-p",
		connection.port ? String(connection.port) : "<port>",
		`${connection.username}@${connection.host}`,
	].join(" ")
	const localPort = connection.localForward.split(":")[0]

	return (
		<section className="card tunnel-card" role="note">
			<div className="card-heading">
				<div>
					<span className="eyebrow">Paired</span>
					<h2>{device.device.label}</h2>
				</div>
				<span className="badge ok">Ready</span>
			</div>

			{device.privateKey ? (
				<>
					<div className="operator-alert tunnel-alert">
						<div>
							<strong>This private key is shown once and not stored.</strong>
							<p>
								Anyone holding it can open this dashboard, including Send and the treasury tools. Keep
								it on the device and avoid key stores that sync to a cloud.
							</p>
						</div>
					</div>
					<div className="tunnel-secret">
						{qr ? <img src={qr} alt="Private key as a QR code" width={220} height={220} /> : null}
						<div className="tunnel-secret-text">
							<textarea readOnly rows={7} className="mono tunnel-key-input" value={device.privateKey} />
							<CopyHash value={device.privateKey} copyLabel="Copy private key">
								Copy private key
							</CopyHash>
						</div>
					</div>
				</>
			) : (
				<p className="hint">
					The phone keeps its private key. Anyone holding it can open this dashboard, including Send and the
					treasury tools, so revoke this device here if the phone is lost.
				</p>
			)}

			<p className="hint">
				In the SSH app: add the key, save a connection using the host, port, username and host key above, add
				the local port forward, connect, then open <code>http://localhost:{localPort}</code> in the phone's
				browser. Equivalent command:
			</p>
			<pre className="tunnel-command mono">{command}</pre>
			<div className="tunnel-actions">
				{device.privateKey ? (
					<label className="tunnel-ack">
						<input type="checkbox" checked={saved} onChange={(e) => setSaved(e.target.checked)} />
						<span>I have saved the key on the device</span>
					</label>
				) : null}
				<button
					type="button"
					className="primary"
					disabled={Boolean(device.privateKey) && !saved}
					onClick={props.onDone}
				>
					Done
				</button>
			</div>
		</section>
	)
}
