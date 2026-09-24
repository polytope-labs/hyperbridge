import { useMemo, useState } from "react"
import { AppSelect } from "../../components/AppSelect"
import type { CreateLimitOrderRequest, OrderbookBook } from "../../types"
import { ApiError } from "../../api"
import { AMOUNT_PATTERN, type LimitOrderDraft, type OrderSide, parseAmount, requestFrom } from "./limitOrderModel"

/**
 * States a limit order the way a book is quoted: a pair, a side, a size in the book's base, and a
 * rate in quote per base. The two amounts the orderbook wants follow from those and are derived
 * here rather than asked for, which is what stopped an operator from having to do the arithmetic
 * and post a rate they did not mean.
 *
 * Only the books the orderbook lists are offered: it refuses a pair it does not keep, and the
 * spelling of a symbol is the book's own.
 */
export function CreateLimitOrderForm(props: {
	books: OrderbookBook[]
	chains: string[]
	chainLabel: (id: string) => string
	onCreated: () => Promise<void> | void
	onCancel: () => void
	create: (request: CreateLimitOrderRequest) => Promise<void>
}) {
	const { books, chains, chainLabel, onCreated, onCancel, create } = props
	const [bookId, setBookId] = useState(() => books[0]?.id ?? "")
	const [side, setSide] = useState<OrderSide>("BID")
	const [amount, setAmount] = useState("")
	const [rate, setRate] = useState("")
	const [fillChain, setFillChain] = useState(() => chains[0] ?? "")
	// A swap can reach the fill chain from anywhere the filler watches, and an order that accepts
	// nothing is one the orderbook refuses outright.
	const [acceptedSources, setAcceptedSources] = useState<string[]>(chains)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string>()

	const book = books.find((entry) => entry.id === bookId)
	const draft: LimitOrderDraft | null = book
		? { book, side, amount, rate, fillChain, acceptedSources }
		: null
	const request = useMemo(() => (draft ? requestFrom(draft) : null), [draft])
	const amountsTyped = parseAmount(amount) !== null && parseAmount(rate) !== null
	const ready = request !== null && !busy

	const submit = async () => {
		if (!request || busy) return
		setBusy(true)
		setError(undefined)
		try {
			await create(request)
			await onCreated()
		} catch (err) {
			setError(err instanceof ApiError ? err.message : "Could not create the limit order")
		} finally {
			setBusy(false)
		}
	}

	const toggleSource = (chain: string) =>
		setAcceptedSources((current) =>
			current.includes(chain) ? current.filter((source) => source !== chain) : [...current, chain],
		)

	if (books.length === 0) {
		return (
			<section className="market-editor operator-market-editor">
				<p className="hint">The orderbook lists no books to trade against yet.</p>
				<footer className="market-dialog-footer">
					<button type="button" onClick={onCancel}>
						Close
					</button>
				</footer>
			</section>
		)
	}

	return (
		<section className="market-editor operator-market-editor">
			<div className="market-editor-heading">
				<div>
					<span className="markets-kicker">What you are offering</span>
					<h3>Pick a pair and a rate</h3>
					<p className="hint">
						Amounts are whole tokens and the rate is {book ? `${book.quote} per ${book.base}` : "quote per base"}.
						The order fills at that rate until it runs out.
					</p>
				</div>
			</div>

			<div className="field">
				<span className="field-label">Pair</span>
				<AppSelect
					ariaLabel="Pair"
					value={bookId}
					options={books.map((entry) => ({ value: entry.id, label: `${entry.base} / ${entry.quote}` }))}
					onValueChange={setBookId}
				/>
			</div>

			<div className="field">
				<span className="field-label">Side</span>
				<div className="limit-order-sides" role="tablist" aria-label="Side">
					{(
						[
							["BID", "Buy", book ? `Take ${book.base} in, pay ${book.quote} out` : ""],
							["ASK", "Sell", book ? `Take ${book.quote} in, pay ${book.base} out` : ""],
						] as [OrderSide, string, string][]
					).map(([value, label, hint]) => (
						<button
							key={value}
							type="button"
							role="tab"
							aria-selected={side === value}
							// The side drives its colour: buying reads green, selling red.
							data-side={value}
							className={`limit-order-side${side === value ? " is-selected" : ""}`}
							onClick={() => setSide(value)}
							title={hint}
						>
							{label} {book?.base}
						</button>
					))}
				</div>
				<small className="hint">
					{side === "BID"
						? `You take ${book?.base} in and pay ${book?.quote} out.`
						: `You take ${book?.quote} in and pay ${book?.base} out.`}
				</small>
			</div>

			<div className="market-asset-grid">
				<label className="field">
					<span className="field-label">Amount ({book?.base})</span>
					<input
						type="text"
						inputMode="decimal"
						placeholder="10"
						aria-label={`Amount in ${book?.base}`}
						value={amount}
						onChange={(event) => AMOUNT_PATTERN.test(event.target.value) && setAmount(event.target.value)}
					/>
				</label>
				<label className="field">
					<span className="field-label">
						Rate ({book?.quote} per {book?.base})
					</span>
					<input
						type="text"
						inputMode="decimal"
						placeholder="1590"
						aria-label={`Rate in ${book?.quote} per ${book?.base}`}
						value={rate}
						onChange={(event) => AMOUNT_PATTERN.test(event.target.value) && setRate(event.target.value)}
					/>
				</label>
			</div>

			{request ? (
				<p className="hint limit-order-derived">
					Takes in{" "}
					<strong>
						{request.amountIn} {request.tokenIn}
					</strong>
					, pays out{" "}
					<strong>
						{request.amountOut} {request.tokenOut}
					</strong>
					.
				</p>
			) : null}
			{!request && amountsTyped ? <p className="error">State an amount and a rate above zero.</p> : null}

			<div className="field">
				<span className="field-label">Fills on</span>
				<AppSelect
					ariaLabel="Chain the order is filled on"
					value={fillChain}
					options={chains.map((chain) => ({ value: chain, label: chainLabel(chain) }))}
					onValueChange={setFillChain}
				/>
				<small className="hint">Where you hold the token you are paying out.</small>
			</div>

			<div className="field">
				<span className="field-label">Accepts swaps from</span>
				<div className="limit-order-sources">
					{chains.map((chain) => (
						<label key={chain} className="limit-order-source">
							<input type="checkbox" checked={acceptedSources.includes(chain)} onChange={() => toggleSource(chain)} />
							<span>{chainLabel(chain)}</span>
						</label>
					))}
				</div>
				{acceptedSources.length === 0 ? (
					<p className="error">Name at least one source chain, or the orderbook refuses the order.</p>
				) : null}
			</div>

			{error ? <p className="error">{error}</p> : null}

			<footer className="market-dialog-footer">
				<button type="button" onClick={onCancel} disabled={busy}>
					Cancel
				</button>
				<button type="button" className="primary" onClick={() => void submit()} disabled={!ready}>
					{busy ? "Posting…" : "Post limit order"}
				</button>
			</footer>
		</section>
	)
}
