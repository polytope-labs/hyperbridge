export class AbortSignalInternal extends Error {
	constructor(message: string) {
		super()

		this.name = "Hyperbridge/SDK/AbortSignalInternal"
		this.message = message
	}

	static isError(error: unknown): error is AbortSignalInternal {
		return error instanceof AbortSignalInternal
	}
}

export class ExpectedError extends Error {
	constructor(message: string) {
		super()

		this.name = "Hyperbridge/SDK/ExpectedError"
		this.message = message
	}

	static isError(error: unknown): error is ExpectedError {
		return error instanceof ExpectedError
	}
}
