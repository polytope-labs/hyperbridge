import { LOG_LEVELS } from "@/services/server/dto"
import { AddressListEditor } from "../../components/AddressListEditor"
import { Field } from "../../components/Field"
import type { StepProps } from "../Wizard"

export function StepAdvanced({ state, setState }: StepProps) {
	return (
		<div className="wizard-sections advanced-step">
			<section className="card">
				<h2>Runtime</h2>
				<p className="hint">
					Working defaults are already selected. Change these only when operating conditions require it.
				</p>
				<div className="row">
					<Field
						label="Max concurrent orders"
						style={{ maxWidth: "14rem" }}
						value={state.maxConcurrentOrders}
						onChange={(maxConcurrentOrders) => setState((s) => ({ ...s, maxConcurrentOrders }))}
					/>
					<label className="field" style={{ maxWidth: "10rem" }}>
						<span>Log level</span>
						<select
							value={state.logging}
							onChange={(e) => setState((s) => ({ ...s, logging: e.target.value }))}
						>
							{LOG_LEVELS.map((level) => (
								<option key={level}>{level}</option>
							))}
						</select>
					</label>
				</div>
				<p className="section-footnote">
					Lower concurrency if RPC providers rate-limit. Gas fee bump and overfill protection retain their
					safe defaults.
				</p>
			</section>

			<section className="card">
				<h2>Order access</h2>
				<p className="hint">Optionally restrict fills to orders submitted by these user addresses.</p>
				<AddressListEditor
					addresses={state.allowlistUsers}
					onChange={(allowlistUsers) => setState((s) => ({ ...s, allowlistUsers }))}
				/>
			</section>
		</div>
	)
}
