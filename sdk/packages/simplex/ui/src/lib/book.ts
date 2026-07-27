// Hand-mirrored from FillerPricePolicy.assertBookNotCrossed
// (src/config/interpolated-curve.ts): flat-clamped piecewise-linear curves,
// crossed when bid <= ask at any breakpoint of either curve. The server
// preview gate re-runs the real check; this only surfaces the error early.

import type { EditorPoint } from "../components/CurveEditor"

interface XY {
	x: number
	y: number
}

function parse(points: EditorPoint[]): XY[] {
	return points
		.filter((p) => p.amount.trim() && p.value.trim())
		.map((p) => ({ x: Number(p.amount), y: Number(p.value) }))
		.filter((p) => Number.isFinite(p.x) && Number.isFinite(p.y))
		.sort((a, b) => a.x - b.x)
}

function evaluate(curve: XY[], x: number): number {
	if (x <= curve[0].x) return curve[0].y
	if (x >= curve[curve.length - 1].x) return curve[curve.length - 1].y
	for (let i = 1; i < curve.length; i++) {
		if (x <= curve[i].x) {
			const a = curve[i - 1]
			const b = curve[i]
			if (b.x === a.x) return b.y
			return a.y + ((b.y - a.y) * (x - a.x)) / (b.x - a.x)
		}
	}
	return curve[curve.length - 1].y
}

/** The first amount where bid <= ask (crossed or zero-spread book), or null when uncrossed. */
export function bookCrossedAt(bid: EditorPoint[], ask: EditorPoint[]): string | null {
	const bidCurve = parse(bid)
	const askCurve = parse(ask)
	if (bidCurve.length === 0 || askCurve.length === 0) return null
	const amounts = [...new Set([...bidCurve, ...askCurve].map((p) => p.x))].sort((a, b) => a - b)
	for (const amount of amounts) {
		if (evaluate(bidCurve, amount) <= evaluate(askCurve, amount)) return String(amount)
	}
	return null
}
