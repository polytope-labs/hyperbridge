/**
 * Piecewise linear interpolation over sorted (amount, price) points.
 * Below the minimum amount, returns the first point's price.
 * Above the maximum amount, returns the last point's price.
 * Between two points, linearly interpolates.
 *
 * @param points - Array of { amount, price } sorted by amount ascending
 * @param amount - The input amount to interpolate at
 * @returns The interpolated price
 */
export function interpolatePrice(points: { amount: number; price: number }[], amount: number): number {
	if (points.length === 0) {
		throw new Error("interpolatePrice: points array must not be empty")
	}

	if (points.length === 1 || amount <= points[0].amount) {
		return points[0].price
	}

	const last = points[points.length - 1]
	if (amount >= last.amount) {
		return last.price
	}

	for (let i = 0; i < points.length - 1; i++) {
		const p1 = points[i]
		const p2 = points[i + 1]
		if (amount >= p1.amount && amount <= p2.amount) {
			const t = (amount - p1.amount) / (p2.amount - p1.amount)
			return p1.price + t * (p2.price - p1.price)
		}
	}

	return last.price
}
