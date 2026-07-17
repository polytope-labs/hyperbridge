export function withTimeout<T>(promise: Promise<T>, ms: number, label = "operation"): Promise<T> {
	return Promise.race([
		promise,
		new Promise<never>((_, reject) => {
			const timer = setTimeout(() => reject(new Error(`${label} timed out after ${ms}ms`)), ms)
			timer.unref?.()
		}),
	])
}
