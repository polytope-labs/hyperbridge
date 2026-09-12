import { LOG_LEVELS } from "@/services/server/dto"
import { AddressListEditor } from "../../components/AddressListEditor"
import { SelectField } from "../../components/AppSelect"
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
					<SelectField
						label="Log level"
						style={{ maxWidth: "10rem" }}
						value={state.logging}
						options={LOG_LEVELS.map((level) => ({ value: level, label: level }))}
						onValueChange={(logging) => setState((state) => ({ ...state, logging }))}
					/>
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
