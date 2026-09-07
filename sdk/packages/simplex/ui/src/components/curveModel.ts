import type { CurvePoint, PricePoint } from "../types"

export interface EditorPoint {
	amount: string
	value: string
}

export interface CurvePreview {
	line: string
	area: string
	plotted: Array<{ x: number; y: number; index: number }>
	minX: number
	maxX: number
	minY: number
	maxY: number
}

export function fromPricePoints(points: PricePoint[] | undefined): EditorPoint[] {
	return (points ?? []).map((point) => ({ amount: point.amount, value: point.price }))
}

export function toPricePoints(points: EditorPoint[]): PricePoint[] {
	const result: PricePoint[] = []
	for (const point of points) {
		const amount = point.amount.trim()
		const price = point.value.trim()
		if (amount && price) result.push({ amount, price })
	}
	return result
}

export function toCurvePoints(points: EditorPoint[]): CurvePoint[] {
	const result: CurvePoint[] = []
	for (const point of points) {
		const amount = point.amount.trim()
		const value = point.value.trim()
		if (amount && value) result.push({ amount, value: Number(value) })
	}
	return result
}

export function createCurvePreview(points: EditorPoint[]): CurvePreview | null {
	const parsed: Array<{ x: number; y: number }> = []
	for (const point of points) {
		const amount = point.amount.trim()
		const value = point.value.trim()
		if (!amount || !value) continue
		const x = Number(amount)
		const y = Number(value)
		if (Number.isFinite(x) && Number.isFinite(y)) parsed.push({ x, y })
	}

	if (parsed.length === 0) return null
	parsed.sort((left, right) => left.x - right.x)

	let minX = parsed[0].x
	let maxX = parsed[0].x
	let minY = parsed[0].y
	let maxY = parsed[0].y
	for (const point of parsed) {
		minX = Math.min(minX, point.x)
		maxX = Math.max(maxX, point.x)
		minY = Math.min(minY, point.y)
		maxY = Math.max(maxY, point.y)
	}

	const spanX = maxX - minX || 1
	const spanY = maxY - minY || 1
	const plotted = parsed.map((point, index) => ({
		x: parsed.length === 1 ? 42 : 42 + ((point.x - minX) / spanX) * 292,
		y: minY === maxY ? 66 : 112 - ((point.y - minY) / spanY) * 88,
		index,
	}))

	let path = ""
	for (const [index, point] of plotted.entries()) {
		path += `${index === 0 ? "M" : " L"}${point.x.toFixed(1)},${point.y.toFixed(1)}`
	}
	const first = plotted[0]
	const last = plotted.at(-1) ?? first
	const line = parsed.length === 1 ? "M42,66 L334,66" : path
	const areaLastX = parsed.length === 1 ? 334 : last.x
	const area = `${line} L${areaLastX.toFixed(1)},122 L${first.x.toFixed(1)},122 Z`

	return { line, area, plotted, minX, maxX, minY, maxY }
}
