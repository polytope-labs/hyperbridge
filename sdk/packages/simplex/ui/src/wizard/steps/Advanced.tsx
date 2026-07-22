import { AddressListEditor } from "../../components/AddressListEditor"
import { Field } from "../../components/Field"
import type { StepProps } from "../Wizard"

export function StepAdvanced({ state, setState }: StepProps) {
	return (
		<div className="card">
			<h2>Advanced</h2>
			<p className="hint">Everything here has working defaults — tune only if you know why.</p>

			<div className="row">
				<Field label="Max concurrent orders (lower if your RPCs rate-limit)" style={{ maxWidth: "14rem" }} value={state.maxConcurrentOrders} onChange={(maxConcurrentOrders) => setState((s) => ({ ...s, maxConcurrentOrders }))} />
				<Field label="Max re-checks per queued order" style={{ maxWidth: "14rem" }} value={state.maxRechecks} onChange={(maxRechecks) => setState((s) => ({ ...s, maxRechecks }))} />
				<Field label="Re-check delay (ms)" style={{ maxWidth: "14rem" }} value={state.recheckDelayMs} onChange={(recheckDelayMs) => setState((s) => ({ ...s, recheckDelayMs }))} />
				<label className="field" style={{ maxWidth: "10rem" }}>
					<span>Log level</span>
					<select value={state.logging} onChange={(e) => setState((s) => ({ ...s, logging: e.target.value }))}>
						{["trace", "debug", "info", "warn", "error"].map((level) => (
							<option key={level}>{level}</option>
						))}
					</select>
				</label>
			</div>

			<div className="field">
				<span style={{ display: "block", marginBottom: "0.3rem", color: "var(--muted)", fontSize: "0.9rem" }}>
					Allowlist (optional): only fill orders placed by these user addresses. Leave empty to fill for everyone.
				</span>
				<AddressListEditor
					addresses={state.allowlistUsers}
					onChange={(allowlistUsers) => setState((s) => ({ ...s, allowlistUsers }))}
				/>
			</div>

			<p className="hint">
				Gas fee bump (8%/10%) and overfill protection (500 bps / 3 clamps) keep their defaults; the generated
				config file documents how to change them.
			</p>
		</div>
	)
}
