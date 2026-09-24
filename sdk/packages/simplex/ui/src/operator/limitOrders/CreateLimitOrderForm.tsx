import { useId, useLayoutEffect, useMemo, useRef, useState } from "react"
import { AppSelect } from "../../components/AppSelect"
import { ChainMultiSelect } from "../../components/ChainMultiSelect"
import { TokenOnChainIcon } from "../../components/TokenIcon"
import { formatAmount } from "../../lib/format"
import type { BalanceSnapshot, CreateLimitOrderRequest, OrderbookBook } from "../../types"
import { ApiError } from "../../api"
import { AMOUNT_PATTERN, groupThousands, type OrderSide, parseAmount, requestFrom } from "./limitOrderModel"

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
	/** What each chain holds of the token the order pays out, shown beside it in "Fills on". */
	balances?: BalanceSnapshot
	onCreated: () => Promise<void> | void
	onCancel: () => void
	create: (request: CreateLimitOrderRequest) => Promise<void>
}) {
	const { books, chains, chainLabel, balances, onCreated, onCancel, create } = props
	const [bookId, setBookId] = useState(() => books[0]?.id ?? "")
	const [side, setSide] = useState<OrderSide>("BID")
	const [amount, setAmount] = useState("")
	const [rate, setRate] = useState("")
	/** Chains the operator picked for "Fills on"; null until they do, while it follows the balances. */
	const [pickedFillChains, setPickedFillChains] = useState<string[] | null>(null)
	// A swap can reach the fill chain from anywhere the filler watches, and an order that accepts
	// nothing is one the orderbook refuses outright.
	const [acceptedSources, setAcceptedSources] = useState<string[]>(chains)
	const [busy, setBusy] = useState(false)
	const [error, setError] = useState<string>()

	const book = books.find((entry) => entry.id === bookId)
	// Buying the base pays out the quote; selling it pays out the base.
	const paysOut = book ? (side === "BID" ? book.quote : book.base) : ""
	// An order pays out on one chain, so filling on several means one order on each. By default
	// that is every chain holding the token paid out: an order on an empty balance fills nothing.
	const heldOn = chains.filter((chain) => shownAsHeld(payOutAsset(balances, chain, paysOut)?.available))
	const fillChains = pickedFillChains ?? heldOn
	const requests = useMemo(
		() =>
			book
				? fillChains.map((fillChain) => requestFrom({ book, side, amount, rate, fillChain, acceptedSources }))
				: [],
		[book, side, amount, rate, fillChains, acceptedSources],
	)
	const valid = requests.length > 0 && requests.every((request) => request !== null)
	const request = valid ? requests[0] : null
	const amountsTyped = parseAmount(amount) !== null && parseAmount(rate) !== null
	const ready = valid && !busy
	const heldTotal = consolidated(balances, fillChains, paysOut)

	const amountId = useId()

	// A side or pair change changes the token paid out. The amount is in that token, so it is
	// cleared rather than silently reread in the new one; the fill-chain default follows it, so
	// a pick made for the old token does not carry over. The rate is quote per base either way.
	const choosePair = (id: string) => {
		setBookId(id)
		setAmount("")
		setPickedFillChains(null)
	}
	const chooseSide = (next: OrderSide) => {
		setSide(next)
		setAmount("")
		setPickedFillChains(null)
	}

	const submit = async () => {
		if (!ready || busy) return
		setBusy(true)
		setError(undefined)
		const posted: string[] = []
		try {
			for (const next of requests as CreateLimitOrderRequest[]) {
				await create(next)
				posted.push(next.fillChain)
			}
			await onCreated()
		} catch (err) {
			const reason = err instanceof ApiError ? err.message : "Could not create the limit order"
			// What went through must not be posted again by a retry.
			if (posted.length > 0) {
				setPickedFillChains(fillChains.filter((chain) => !posted.includes(chain)))
				setError(`Posted on ${posted.map(chainLabel).join(", ")}. The rest failed: ${reason}`)
			} else {
				setError(reason)
			}
		} finally {
			setBusy(false)
		}
	}

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
			<div className="field">
				<span className="field-label">Pair</span>
				<AppSelect
					ariaLabel="Pair"
					value={bookId}
					options={books.map((entry) => ({ value: entry.id, label: `${entry.base} / ${entry.quote}` }))}
					onValueChange={choosePair}
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
							onClick={() => chooseSide(value)}
							title={hint}
						>
							{label} {book?.base}
						</button>
					))}
				</div>
			</div>

			<div className="limit-order-row">
				{/* Not a <label>: it holds the Max button, which a label would claim as its control. */}
				<div className="field limit-order-amount">
					<div className="field-label">
						<label htmlFor={amountId}>Amount ({paysOut})</label>
						{heldTotal === undefined ? null : (
							<span className="limit-order-balance">
								{heldTotal === null ? "Balance unavailable" : `${formatAmount(heldTotal)} ${paysOut} held`}
								{heldTotal ? (
									<button type="button" className="limit-order-max" onClick={() => setAmount(maxAmount(heldTotal))}>
										Max
									</button>
								) : null}
							</span>
						)}
					</div>
					<DecimalInput
						id={amountId}
						value={amount}
						onChange={setAmount}
						placeholder={side === "BID" ? "15,900" : "10"}
						ariaLabel={`Amount in ${paysOut}`}
					/>
				</div>
				<label className="field">
					<span className="field-label">
						Rate ({book?.quote} per {book?.base})
					</span>
					<DecimalInput
						value={rate}
						onChange={setRate}
						placeholder="1,590"
						ariaLabel={`Rate in ${book?.quote} per ${book?.base}`}
					/>
				</label>
			</div>

			{request ? (
				<p className="hint limit-order-derived">
					{requests.length > 1 ? `${requests.length} orders, one on each chain. Each takes in ` : "Takes in "}
					<strong>
						{readable(request.amountIn)} {request.tokenIn}
					</strong>
					, pays out{" "}
					<strong>
						{readable(request.amountOut)} {request.tokenOut}
					</strong>
					.
				</p>
			) : null}
			{!request && amountsTyped && fillChains.length > 0 ? (
				<p className="error">State an amount and a rate above zero.</p>
			) : null}

			<div className="limit-order-row">
				<div className="field">
					<span className="field-label">Accepts swaps from</span>
					<ChainMultiSelect
						ariaLabel="Chains the order accepts swaps from"
						options={chains.map((chain) => ({ value: chain, label: chainLabel(chain) }))}
						value={acceptedSources}
						onValueChange={setAcceptedSources}
					/>
					{acceptedSources.length === 0 ? <p className="error">Pick at least one chain.</p> : null}
				</div>
				<div className="field">
					<span className="field-label">Fills on</span>
					<ChainMultiSelect
						ariaLabel="Chains the order is filled on"
						options={chains.map((chain) => ({
							value: chain,
							label: chainLabel(chain),
							leading: <TokenOnChainIcon symbol={paysOut} chain={chainLabel(chain)} />,
							trailing: payOutBalance(balances, chain, paysOut),
						}))}
						value={fillChains}
						onValueChange={setPickedFillChains}
					/>
					{fillChains.length === 0 ? <p className="error">Pick at least one chain.</p> : null}
				</div>
			</div>

			{error ? <p className="error">{error}</p> : null}

			<footer className="market-dialog-footer">
				<button type="button" onClick={onCancel} disabled={busy}>
					Cancel
				</button>
				<button type="button" className="primary" onClick={() => void submit()} disabled={!ready}>
					{busy ? "Posting…" : requests.length > 1 ? `Post ${requests.length} limit orders` : "Post limit order"}
				</button>
			</footer>
		</section>
	)
}

/**
 * What a chain holds of the token an order would pay out there, for "Fills on". A token the
 * chain does not carry reads "—"; one whose read failed says so rather than showing zero.
 */
function payOutBalance(balances: BalanceSnapshot | undefined, chain: string, symbol: string): string | undefined {
	if (!balances || !symbol) return undefined
	const asset = payOutAsset(balances, chain, symbol)
	if (!asset) return "—"
	if (asset.available === null) return "Unavailable"
	return `${formatAmount(asset.available)} ${symbol}`
}

/**
 * Whether the menu shows a balance at all. It prints four decimals, so dust under that reads as 0
 * there, and a chain the operator sees holding nothing is not picked for them either.
 */
function shownAsHeld(available: number | null | undefined): boolean {
	return available !== null && available !== undefined && Math.round(available * 10_000) > 0
}

/** The token as one chain's balances report it, or undefined when that chain does not carry it. */
function payOutAsset(balances: BalanceSnapshot | undefined, chain: string, symbol: string) {
	if (!balances || !symbol) return undefined
	const chainId = Number(chain.replace(/^EVM-/, ""))
	return balances.chains
		.find((row) => row.chainId === chainId)
		?.assets.find((entry) => entry.symbol.trim().toUpperCase() === symbol.trim().toUpperCase())
}

/**
 * The token held across the fill chains, beside the amount. Undefined before balances arrive;
 * null when any chain's read failed, since a total quietly missing one would understate it.
 */
function consolidated(
	balances: BalanceSnapshot | undefined,
	chains: string[],
	symbol: string,
): number | null | undefined {
	if (!balances || !symbol || chains.length === 0) return undefined
	let total = 0
	for (const chain of chains) {
		const available = payOutAsset(balances, chain, symbol)?.available
		if (available === null) return null
		total += available ?? 0
	}
	return total
}

/**
 * A request amount for the summary line: grouped, and cut to six decimals. Dividing by the rate
 * gives eighteen, which the order posts exactly but nobody reads.
 */
function readable(decimal: string): string {
	const [whole, fraction = ""] = decimal.split(".")
	const kept = fraction.slice(0, 6).replace(/0+$/, "")
	return groupThousands(kept ? `${whole}.${kept}` : whole)
}

/**
 * Everything held, as a figure the amount field takes. Balances arrive as floats, so this rounds
 * down to six decimals: an order a hair over the balance buys nothing, a figure in exponent
 * notation the field would refuse.
 */
function maxAmount(held: number): string {
	return (Math.floor(held * 1e6) / 1e6).toFixed(6).replace(/\.?0+$/, "")
}

/**
 * A decimal field that shows its figure grouped in thousands but hands back the bare one. The
 * caret goes back after the same number of digits it followed, or each comma a keystroke adds
 * would throw it to the end of the field.
 */
function DecimalInput(props: {
	id?: string
	value: string
	onChange: (value: string) => void
	placeholder: string
	ariaLabel: string
}) {
	const { id, value, onChange, placeholder, ariaLabel } = props
	const input = useRef<HTMLInputElement>(null)
	/** Digits and point before the caret after the last keystroke, until it is put back. */
	const caret = useRef<number | null>(null)
	const shown = groupThousands(value)

	useLayoutEffect(() => {
		const element = input.current
		if (!element || caret.current === null || document.activeElement !== element) return
		let seen = 0
		let position = 0
		while (position < shown.length && seen < caret.current) {
			if (shown[position] !== ",") seen++
			position++
		}
		element.setSelectionRange(position, position)
		caret.current = null
	}, [shown])

	return (
		<input
			ref={input}
			id={id}
			type="text"
			inputMode="decimal"
			placeholder={placeholder}
			aria-label={ariaLabel}
			value={shown}
			onChange={(event) => {
				const typed = event.target.value
				const bare = typed.replace(/,/g, "")
				if (!AMOUNT_PATTERN.test(bare)) return
				caret.current = typed.slice(0, event.target.selectionStart ?? typed.length).replace(/,/g, "").length
				onChange(bare)
			}}
		/>
	)
}
