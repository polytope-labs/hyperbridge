export interface RetryConfig {
	maxRetries: number;
	backoffMs: number;
	logMessage?: string;
}

export async function retryPromise<T>(operation: () => Promise<T>, retryConfig: RetryConfig): Promise<T> {
	const { logMessage = "Retry operation failed" } = retryConfig

	let lastError: unknown
	for (let i = 0; i < retryConfig.maxRetries; i++) {
		try {
			return await operation()
		} catch (error) {
			logger.trace(`Retrying(${i + 1}/${retryConfig.maxRetries}) > ${logMessage}`)
			lastError = error
			await new Promise((resolve) => setTimeout(resolve, retryConfig.backoffMs * 2 ** i))
		}
	}

	throw lastError
}