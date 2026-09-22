export type EndpointVerificationState = "checking" | "success" | "warning" | "error"

interface EndpointVerificationStatusProps {
	state?: EndpointVerificationState
	message?: string
}

const ICONS: Record<EndpointVerificationState, string> = {
	checking: "…",
	success: "✓",
	warning: "!",
	error: "×",
}

/** Persistent, row-local result for an RPC and bundler verification attempt. */
export function EndpointVerificationStatus({ state, message }: EndpointVerificationStatusProps) {
	if (!state || !message) return null
	return (
		<p
			className="chain-endpoint-status"
			data-status={state}
			role={state === "error" ? "alert" : "status"}
			aria-live="polite"
		>
			<span className="chain-endpoint-status-icon" aria-hidden="true">
				{ICONS[state]}
			</span>
			<span>{message}</span>
		</p>
	)
}
