import { createContext, useCallback, useContext, useMemo, useRef, useState, type ReactNode } from "react"
import { toast } from "sonner"
import { api } from "../api"
import { DownloadIcon } from "../components/InterfaceIcons"
import { usePolling } from "../lib/hooks"
import { isNativeDesktopProtocol } from "../lib/runtime"

/** The desktop shell's updater, as `GET /api/desktop/update` reports it. */
interface UpdateStatus {
	state: string
	targetVersion?: string
	detail?: string
}

function inProgress(status: UpdateStatus | undefined): boolean {
	return status?.state === "stopping-solver" || status?.state === "installing"
}

interface DesktopUpdate {
	/** The downloaded version, from when it is ready until it is installed. */
	version?: string
	/** The operator asked for the install: the solver is stopping, or the installer is running. */
	installing: boolean
	install(): void
}

const POLL_MS = 15_000
const INSTALLING_POLL_MS = 2_000

const DesktopUpdateContext = createContext<DesktopUpdate>({ installing: false, install: () => {} })

export function useDesktopUpdate(): DesktopUpdate {
	return useContext(DesktopUpdateContext)
}

/**
 * Follows the desktop shell's updater. The shell downloads an update by itself and installs it
 * only when `install` is called. A browser session has no shell, so nothing is ever offered there.
 */
export function DesktopUpdateProvider(props: { children: ReactNode }) {
	const native = isNativeDesktopProtocol(window.location.protocol)
	const [status, setStatus] = useState<UpdateStatus>()
	const [requested, setRequested] = useState(false)
	// Refs, because `load` runs from a timer: whether this dashboard's own request is still open, and
	// whether the last poll saw an install in progress.
	const requestOpen = useRef(false)
	const sawInstall = useRef(false)
	const load = useCallback(async () => {
		if (!native) return
		let next: UpdateStatus
		try {
			next = await api.get<UpdateStatus>("/api/desktop/update")
		} catch {
			// The last state stands until the shell answers again.
			return
		}
		// An install started from the menu, or one that failed after the shell accepted it, has no
		// open request here to carry the failure back.
		if (sawInstall.current && next.state === "error" && !requestOpen.current) {
			toast.error("The update was not installed", { description: next.detail })
		}
		sawInstall.current = inProgress(next)
		setStatus(next)
	}, [native])
	const installing = requested || inProgress(status)
	usePolling(load, native ? (installing ? INSTALLING_POLL_MS : POLL_MS) : undefined)

	const install = useCallback(() => {
		requestOpen.current = true
		setRequested(true)
		void api
			.post("/api/desktop/update")
			.catch((cause) =>
				toast.error("The update was not installed", {
					description: cause instanceof Error ? cause.message : String(cause),
				}),
			)
			// The state is read again before the request is closed, so the dashboard does not flash back.
			.then(load)
			.finally(() => {
				requestOpen.current = false
				setRequested(false)
			})
	}, [load])

	const version = status?.state === "ready" || installing ? status?.targetVersion : undefined
	const value = useMemo(() => ({ version, installing, install }), [version, installing, install])
	return <DesktopUpdateContext.Provider value={value}>{props.children}</DesktopUpdateContext.Provider>
}

/** The sidebar's update button. It shows once an update is downloaded, and installs it. */
export function UpdateButton() {
	const { version, install } = useDesktopUpdate()
	if (!version) return null
	return (
		<button
			type="button"
			className="operator-nav-item operator-update-nav"
			aria-label={`Update to ${version}`}
			title="Stops the solver, installs the update and reopens Simplex"
			onClick={install}
		>
			<DownloadIcon aria-hidden="true" />
			<span>
				<strong>
					<span className="operator-update-prefix">Update to </span>
					{version}
				</strong>
				<small>Restarts the solver</small>
			</span>
		</button>
	)
}

/** Shown in place of the dashboard while an update installs: the solver it reads from is going away. */
export function UpdatingScreen() {
	const { version } = useDesktopUpdate()
	return (
		<div className="connection-error update-progress" role="status">
			<span className="eyebrow">Simplex update</span>
			<h1>Updating{version ? ` to ${version}` : ""}</h1>
			<p>The solver finishes its in-flight fills and stops. Simplex then installs the update and reopens.</p>
		</div>
	)
}
