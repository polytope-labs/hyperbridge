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

			<Field
				label="Allowlist (optional): only fill orders placed by these user addresses — one per line or comma-separated. Leave empty to fill for everyone."
				value={state.allowlist}
				onChange={(allowlist) => setState((s) => ({ ...s, allowlist }))}
				placeholder="0x…, 0x…"
			/>

			<p className="hint">
				Gas fee bump (8%/10%) and overfill protection (500 bps / 3 clamps) keep their defaults; the generated
				config file documents how to change them.
			</p>
		</div>
	)
}
