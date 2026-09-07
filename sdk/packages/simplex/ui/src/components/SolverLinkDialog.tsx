import { useState } from "react"
import { toast } from "sonner"
import { planSolverLink, SOLVER_NAME_MAX } from "../lib/solver-link"
import type { AdminStrategyDto } from "../types"
import { CopyIcon } from "./InterfaceIcons"
import { WizardDialog } from "./WizardDialog"

const NAME_KEY = "simplex.solverName"

function rememberedName(): string {
	try {
		return localStorage.getItem(NAME_KEY) || "Simplex"
	} catch {
		return "Simplex"
	}
}

/**
 * Builds a HyperFX solver link for one market from the running settings: the
 * filler's address, the chosen chain and the market's current curve prices.
 */
export function SolverLinkDialog(props: {
	open: boolean
	onClose: () => void
	strategy: AdminStrategyDto
	chains: number[]
	chainLabel: (id: number) => string
	solverAddress: string | undefined
}) {
	const { open, onClose, strategy, chains, chainLabel, solverAddress } = props
	const [chainId, setChainId] = useState<number>(chains[0] ?? 0)
	const [name, setName] = useState(rememberedName)
	const plan = planSolverLink(strategy, chainId, solverAddress, name)

	const copy = () => {
		if (!plan.ok) return
		try {
			localStorage.setItem(NAME_KEY, name.trim())
		} catch {
			// remembering the name is a convenience only
		}
		navigator.clipboard
			.writeText(plan.link)
			.then(() => toast.success("Solver link copied"))
			.catch(() => toast.error("Could not copy the link; select it and copy by hand"))
	}

	return (
		<WizardDialog
			open={open}
			onClose={onClose}
			title="Solver link"
			description="A HyperFX swap page locked to this filler, this pair and its current prices."
		>
			<div className="solver-link-form">
				<div className="solver-link-fields">
					{chains.length > 1 && (
						<label className="field">
							<span>Chain</span>
							<select value={chainId} onChange={(event) => setChainId(Number(event.target.value))}>
								{chains.map((id) => (
									<option key={id} value={id}>
										{chainLabel(id)}
									</option>
								))}
							</select>
						</label>
					)}
					<label className="field">
						<span>Solver name</span>
						<input
							type="text"
							maxLength={SOLVER_NAME_MAX}
							value={name}
							onChange={(event) => setName(event.target.value)}
							placeholder="Shown to the user on the swap page"
						/>
					</label>
				</div>
				{plan.ok ? (
					<>
						<dl className="solver-link-summary">
							<div>
								<dt>Swap</dt>
								<dd>
									{plan.from} → {plan.to} on {chainLabel(chainId)}
								</dd>
							</div>
							<div>
								<dt>Rate</dt>
								<dd>
									1 {plan.from} = {plan.rate} {plan.to}
								</dd>
							</div>
							{plan.reverseRate && (
								<div>
									<dt>Reverse</dt>
									<dd>
										1 {plan.from} = {plan.reverseRate} {plan.to} when buying {plan.from}
									</dd>
								</div>
							)}
						</dl>
						<code className="solver-link-url">{plan.link}</code>
						<p className="hint">
							Rates are the first point of this market's curves at the moment you copy; the link does not
							follow later curve edits.
						</p>
						<footer className="market-dialog-footer market-dialog-footer-end">
							<button type="button" className="primary" onClick={copy}>
								<CopyIcon aria-hidden="true" /> Copy link
							</button>
						</footer>
					</>
				) : (
					<p className="warning">{plan.reason}</p>
				)}
			</div>
		</WizardDialog>
	)
}
