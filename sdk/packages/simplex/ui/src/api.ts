export class ApiError extends Error {
	status: number
	constructor(status: number, message: string) {
		super(message)
		this.status = status
	}
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
	const response = await fetch(path, {
		...init,
		headers: {
			"Content-Type": "application/json",
			// CSRF hygiene: the server rejects mutations without this header.
			"X-Simplex-UI": "1",
			...init?.headers,
		},
	})
	const body = await response.json().catch(() => ({}))
	if (!response.ok) {
		throw new ApiError(response.status, (body as { error?: string }).error ?? `HTTP ${response.status}`)
	}
	return body as T
}

export const api = {
	get: <T>(path: string) => request<T>(path),
	post: <T>(path: string, body?: unknown) =>
		request<T>(path, { method: "POST", body: JSON.stringify(body ?? {}) }),
	put: <T>(path: string, body: unknown) => request<T>(path, { method: "PUT", body: JSON.stringify(body) }),
}
