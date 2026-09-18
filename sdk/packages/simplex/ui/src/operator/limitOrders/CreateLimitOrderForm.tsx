import { useState } from "react"
import { AppSelect } from "../../components/AppSelect"
import type { CreateLimitOrderRequest } from "../../types"
import { ApiError } from "../../api"

interface Draft {
	fillChain: string
	tokenIn: string
	amountIn: string
	tokenOut: string
	amountOut: string
	acceptedSources: string[]
}

/** A decimal amount in whole tokens, which is what the operator states and the handler scales. */
const AMOUNT = /^[0-9]+(\.[0-9]+)?$/

export function CreateLimitOrderForm(props: {
	symbols: string[]
	chains: string[]
	chainLabel: (id: string) => string
	onCreated: () => Promise<void> | void
	onCancel: () => void
	create: (request: CreateLimitOrderRequest) => Promise<void>
}) {
	const { symbols, chains, chainLabel, onCreated, onCancel, create } = props
	const [draft, setDraft] = useState<Draft>(() => ({
		fillChain: chains[0] ?? "",
		tokenIn: symbols[0] ?? "",
		amountIn: "",
		tokenOut: symbols.find((symbol) => symbol !== symbols[0]) ?? "",
		amountOut: "",
		// A swap can reach the fill chain from anywhere the filler watches, and an
		// order that accepts nothing is one the orderbook refuses outright.
		acceptedSources: chains,
	}))
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string>()
	const patch = (changes: Partial<Draft>) => setDraft((current) => ({ ...current, ...changes }))

	const amountsValid = AMOUNT.test(draft.amountIn) && AMOUNT.test(draft.amountOut)
	const ready =
		amountsValid &&
		Number(draft.amountIn) > 0 &&
		Number(draft.amountOut) > 0 &&
		draft.tokenIn !== draft.tokenOut &&
		draft.fillChain !== "" &&
		draft.acceptedSources.length > 0

	// What the two amounts imply, which is the number the operator is really
	// choosing: they state what they take and what they pay, not a rate.
	const rate =
		amountsValid && Number(draft.amountIn) > 0
			? (Number(draft.amountOut) / Number(draft.amountIn)).toLocaleString(undefined, { maximumFractionDigits: 6 })
			: null

	const submit = async () => {
		if (!ready || busy) return
		setBusy(true)
		setError(undefined)
		try {
			await create({
				fillChain: draft.fillChain,
				tokenIn: draft.tokenIn,
				amountIn: draft.amountIn,
				tokenOut: draft.tokenOut,
				amountOut: draft.amountOut,
				acceptedSources: draft.acceptedSources,
			})
			await onCreated()
		} catch (err) {
			setError(err instanceof ApiError ? err.message : "Could not create the limit order")
		} finally {
			setBusy(false)
		}
	}

	const toggleSource = (chain: string) =>
		patch({
			acceptedSources: draft.acceptedSources.includes(chain)
				? draft.acceptedSources.filter((source) => source !== chain)
				: [...draft.acceptedSources, chain],
		})

	return (
		<section className="market-editor operator-market-editor">
			<div className="market-editor-heading">
				<div>
					<span className="markets-kicker">What you are offering</span>
					<h3>State the two amounts</h3>
					<p className="hint">
						Amounts are whole tokens, as you would say them out loud. The rate is whatever the two imply, and
						the order fills at it until it runs out.
					</p>
				</div>
			</div>

			<div className="market-asset-grid">
				<label className="field">
					<span className="field-label">You take in</span>
					<AppSelect
						ariaLabel="Token taken in"
						value={draft.tokenIn}
						options={symbols.map((symbol) => ({ value: symbol, label: symbol }))}
						onValueChange={(value) => patch({ tokenIn: value })}
					/>
					<input
						type="text"
						inputMode="decimal"
						placeholder="1000"
						aria-label="Amount taken in"
						value={draft.amountIn}
						onChange={(event) => patch({ amountIn: event.target.value })}
					/>
				</label>
				<label className="field">
					<span className="field-label">You pay out</span>
					<AppSelect
						ariaLabel="Token paid out"
						value={draft.tokenOut}
						options={symbols.map((symbol) => ({ value: symbol, label: symbol }))}
						onValueChange={(value) => patch({ tokenOut: value })}
					/>
					<input
						type="text"
						inputMode="decimal"
						placeholder="1500000"
						aria-label="Amount paid out"
						value={draft.amountOut}
						onChange={(event) => patch({ amountOut: event.target.value })}
					/>
				</label>
			</div>

			{rate && draft.tokenIn !== draft.tokenOut ? (
				<p className="hint">
					That is <strong>{rate}</strong> {draft.tokenOut} per {draft.tokenIn}.
				</p>
			) : null}
			{draft.tokenIn === draft.tokenOut ? <p className="error">Choose two different tokens.</p> : null}

			<div className="field">
				<span className="field-label">Fills on</span>
				<AppSelect
					ariaLabel="Chain the order is filled on"
					value={draft.fillChain}
					options={chains.map((chain) => ({ value: chain, label: chainLabel(chain) }))}
					onValueChange={(value) => patch({ fillChain: value })}
				/>
				<small className="hint">Where you hold the token you are paying out.</small>
			</div>

			<div className="field">
				<span className="field-label">Accepts swaps from</span>
				<div className="limit-order-sources">
					{chains.map((chain) => (
						<label key={chain} className="limit-order-source">
							<input
								type="checkbox"
								checked={draft.acceptedSources.includes(chain)}
								onChange={() => toggleSource(chain)}
							/>
							<span>{chainLabel(chain)}</span>
						</label>
					))}
				</div>
				{draft.acceptedSources.length === 0 ? (
					<p className="error">Name at least one source chain, or the orderbook refuses the order.</p>
				) : null}
			</div>

			{error ? <p className="error">{error}</p> : null}

			<footer className="market-dialog-footer">
				<button type="button" onClick={onCancel} disabled={busy}>
					Cancel
				</button>
				<button type="button" className="primary" onClick={() => void submit()} disabled={!ready || busy}>
					{busy ? "Posting…" : "Post limit order"}
				</button>
			</footer>
		</section>
	)
}
