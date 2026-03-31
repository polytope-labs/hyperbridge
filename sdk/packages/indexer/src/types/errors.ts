export class ErrTokenPriceUnavailable extends Error {
	constructor(message: string) {
		super()
		this.name = "ErrTokenPriceUnavailable"
		this.message = message
	}

	static isError(error: unknown): error is ErrTokenPriceUnavailable {
		return error instanceof ErrTokenPriceUnavailable
	}
}
