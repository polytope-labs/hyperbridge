import { useId, useMemo } from "react"
import { createCurvePreview, type EditorPoint } from "./curveModel"
import { ChartLineIcon, CloseIcon } from "./InterfaceIcons"

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
	const gradientId = useId()

	const update = (index: number, key: keyof EditorPoint, value: string) => {
		const next = points.map((p, i) => (i === index ? { ...p, [key]: value } : p))
		onChange(next)
	}

	const preview = useMemo(() => createCurvePreview(points), [points])

	return (
		<div className="curve-editor">
			<div className="curve-editor-visual">
				<div className="curve-editor-visual-heading">
					<span>Price curve</span>
					<small>
						{preview
							? `${preview.plotted.length} ${preview.plotted.length === 1 ? "point" : "points"}`
							: "Add a price below"}
					</small>
				</div>
				{preview ? (
					<svg viewBox="0 0 360 148" role="img" aria-label="Price by order size preview">
						<defs>
							<linearGradient id={gradientId} x1="0" x2="0" y1="0" y2="1">
								<stop offset="0%" stopColor="var(--focus)" stopOpacity="0.22" />
								<stop offset="100%" stopColor="var(--focus)" stopOpacity="0" />
							</linearGradient>
						</defs>
						<g className="curve-grid-lines" aria-hidden="true">
							<path d="M42 24H334 M42 66H334 M42 108H334" />
							<path d="M42 24V122 M188 24V122 M334 24V122" />
						</g>
						<path d={preview.area} fill={`url(#${gradientId})`} />
						<path className="curve-preview-line" d={preview.line} />
						{preview.plotted.map((point) => (
							<circle key={point.index} className="curve-preview-point" cx={point.x} cy={point.y} r="4" />
						))}
						<text x="42" y="140">
							{preview.minX}
						</text>
						<text x="334" y="140" textAnchor="end">
							{preview.maxX}
						</text>
					</svg>
				) : (
					<div className="curve-editor-empty">
						<ChartLineIcon aria-hidden="true" />
						<p>Your curve will appear as prices are entered.</p>
					</div>
				)}
			</div>
			<table className="curve-editor-table">
				<thead>
					<tr>
						<th>{amountLabel}</th>
						<th>{valueLabel}</th>
						<th />
					</tr>
				</thead>
				<tbody>
					{points.map((point, index) => (
						<tr key={index}>
							<td>
								<input
									type="text"
									aria-label={`${amountLabel}, point ${index + 1}`}
									value={point.amount}
									onChange={(e) => update(index, "amount", e.target.value)}
								/>
							</td>
							<td>
								<input
									type="text"
									aria-label={`${valueLabel}, point ${index + 1}`}
									value={point.value}
									onChange={(e) => update(index, "value", e.target.value)}
								/>
							</td>
							<td>
								<button
									type="button"
									className="curve-point-remove"
									aria-label={`Remove curve point ${index + 1}`}
									disabled={points.length <= minPoints}
									onClick={() => onChange(points.filter((_, i) => i !== index))}
								>
									<CloseIcon aria-hidden="true" />
								</button>
							</td>
						</tr>
					))}
				</tbody>
			</table>
			<div className="curve-editor-footer">
				<button
					className="curve-editor-add"
					type="button"
					onClick={() => onChange([...points, { amount: "1", value: "" }])}
				>
					Add point
				</button>
			</div>
		</div>
	)
}
