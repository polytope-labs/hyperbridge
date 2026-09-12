/** Page numbers with the current page's neighbours, the ends, and ellipses between. */
export function pageNumbers(current: number, last: number): Array<number | "…"> {
	if (last <= 7) return Array.from({ length: last }, (_, index) => index + 1)
	const wanted = new Set([1, 2, last - 1, last, current - 1, current, current + 1])
	const pages = [...wanted].filter((page) => page >= 1 && page <= last).sort((a, b) => a - b)
	const out: Array<number | "…"> = []
	for (const page of pages) {
		const previous = out[out.length - 1]
		if (typeof previous === "number" && page - previous > 1) out.push("…")
		out.push(page)
	}
	return out
}

/** Shared pager for the history tables; `noun` names what is being counted. */
export function Pager(props: {
	page: number
	pageSize: number
	total: number
	noun: string
	onPage: (page: number) => void
}) {
	const { page, pageSize, total, noun, onPage } = props
	const last = Math.max(1, Math.ceil(total / pageSize))
	const from = total === 0 ? 0 : (page - 1) * pageSize + 1
	const to = Math.min(total, page * pageSize)
	// "1 order", not "1 orders"; every noun the tables use is a regular plural.
	const counted = total === 1 ? noun.replace(/s$/, "") : noun
	return (
		<nav className="history-pager" aria-label={`${noun} pages`}>
			<small>
				{total === 0 ? `No ${noun}` : `Showing ${from}–${to} of ${total.toLocaleString()} ${counted}`}
			</small>
			<div className="history-pager-pages">
				<button type="button" disabled={page <= 1} onClick={() => onPage(page - 1)} aria-label="Previous page">
					‹
				</button>
				{pageNumbers(page, last).map((entry, index) =>
					entry === "…" ? (
						// biome-ignore lint/suspicious/noArrayIndexKey: ellipses have no identity beyond their slot
						<span key={`gap-${index}`}>…</span>
					) : (
						<button
							type="button"
							key={entry}
							data-active={entry === page}
							aria-current={entry === page ? "page" : undefined}
							onClick={() => onPage(entry)}
						>
							{entry}
						</button>
					),
				)}
				<button type="button" disabled={page >= last} onClick={() => onPage(page + 1)} aria-label="Next page">
					›
				</button>
			</div>
		</nav>
	)
}
