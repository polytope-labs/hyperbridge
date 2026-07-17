import type { StepProps } from "../Wizard"

export function StepAdvanced({ state, setState }: StepProps) {
	return (
		<div className="card">
			<h2>Advanced</h2>
			<p className="hint">Everything here has working defaults — tune only if you know why.</p>

			<div className="row">
				<label className="field" style={{ maxWidth: "14rem" }}>
					<span>Max concurrent orders (lower if your RPCs rate-limit)</span>
					<input
						type="text"
						value={state.maxConcurrentOrders}
						onChange={(e) => setState((s) => ({ ...s, maxConcurrentOrders: e.target.value }))}
					/>
				</label>
				<label className="field" style={{ maxWidth: "14rem" }}>
					<span>Max re-checks per queued order</span>
					<input
						type="text"
						value={state.maxRechecks}
						onChange={(e) => setState((s) => ({ ...s, maxRechecks: e.target.value }))}
					/>
				</label>
				<label className="field" style={{ maxWidth: "14rem" }}>
					<span>Re-check delay (ms)</span>
					<input
						type="text"
						value={state.recheckDelayMs}
						onChange={(e) => setState((s) => ({ ...s, recheckDelayMs: e.target.value }))}
					/>
				</label>
				<label className="field" style={{ maxWidth: "10rem" }}>
					<span>Log level</span>
					<select value={state.logging} onChange={(e) => setState((s) => ({ ...s, logging: e.target.value }))}>
						{["trace", "debug", "info", "warn", "error"].map((level) => (
							<option key={level}>{level}</option>
						))}
					</select>
				</label>
			</div>

			<label className="field">
				<span>
					Allowlist (optional): only fill orders placed by these user addresses — one per line or comma-separated.
					Leave empty to fill for everyone.
				</span>
				<input
					type="text"
					value={state.allowlist}
					onChange={(e) => setState((s) => ({ ...s, allowlist: e.target.value }))}
					placeholder="0x…, 0x…"
				/>
			</label>

			<p className="hint">
				Gas fee bump (8%/10%) and overfill protection (500 bps / 3 clamps) keep their defaults; the generated
				config file documents how to change them.
			</p>
		</div>
	)
}
