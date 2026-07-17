import { useMemo } from "react"

export interface EditorPoint {
	amount: string
	value: string
}

/**
 * Table editor for (amount, value) curve points with a live piecewise-linear
 * preview — the same visual model the interpolated runtime curves follow.
 */
export function CurveEditor(props: {
	points: EditorPoint[]
	onChange: (points: EditorPoint[]) => void
	amountLabel: string
	valueLabel: string
	minPoints?: number
}) {
	const { points, onChange, amountLabel, valueLabel, minPoints = 1 } = props

	const update = (index: number, key: keyof EditorPoint, value: string) => {
		const next = points.map((p, i) => (i === index ? { ...p, [key]: value } : p))
		onChange(next)
	}

	const preview = useMemo(() => {
		const parsed = points
			.map((p) => ({ x: Number(p.amount), y: Number(p.value) }))
			.filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y))
			.sort((a, b) => a.x - b.x)
		if (parsed.length < 2) return null
		const xs = parsed.map((p) => p.x)
		const ys = parsed.map((p) => p.y)
		const minX = Math.min(...xs)
		const maxX = Math.max(...xs)
		const minY = Math.min(...ys)
		const maxY = Math.max(...ys)
		const spanX = maxX - minX || 1
		const spanY = maxY - minY || 1
		const d = parsed
			.map((p, i) => {
				const x = 8 + ((p.x - minX) / spanX) * 264
				const y = 52 - ((p.y - minY) / spanY) * 44
				return `${i === 0 ? "M" : "L"}${x.toFixed(1)},${y.toFixed(1)}`
			})
			.join(" ")
		return { d, minY, maxY }
	}, [points])

	return (
		<div>
			<table>
				<thead>
					<tr>
						<th>{amountLabel}</th>
						<th>{valueLabel}</th>
						<th />
					</tr>
				</thead>
				<tbody>
					{points.map((point, index) => (
						// biome-ignore lint/suspicious/noArrayIndexKey: rows are positional
						<tr key={index}>
							<td>
								<input type="text" value={point.amount} onChange={(e) => update(index, "amount", e.target.value)} />
							</td>
							<td>
								<input type="text" value={point.value} onChange={(e) => update(index, "value", e.target.value)} />
							</td>
							<td>
								<button
									type="button"
									disabled={points.length <= minPoints}
									onClick={() => onChange(points.filter((_, i) => i !== index))}
								>
									✕
								</button>
							</td>
						</tr>
					))}
				</tbody>
			</table>
			<div className="row" style={{ marginTop: "0.5rem" }}>
				<button type="button" onClick={() => onChange([...points, { amount: "", value: "" }])}>
					+ Add point
				</button>
				{preview && (
					<svg width="280" height="60" role="img" aria-label="curve preview">
						<path d={preview.d} fill="none" stroke="var(--accent)" strokeWidth="2" />
					</svg>
				)}
			</div>
		</div>
	)
}
