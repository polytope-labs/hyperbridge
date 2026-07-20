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

/**
 * Hyperbridge no longer retains the consensus update for a state-machine
 * height. Proofs derived from that height can never be submitted again.
 */
export class MissingConsensusUpdateTimeError extends Error {
	constructor(message = "Error fetching Consensus update time", options?: { cause?: unknown }) {
		super(message, options)
		this.name = "Hyperbridge/SDK/MissingConsensusUpdateTimeError"
	}

	static isError(error: unknown): error is MissingConsensusUpdateTimeError {
		if (error instanceof MissingConsensusUpdateTimeError) return true

		const messages: string[] = []
		let current = error
		while (current && typeof current === "object" && messages.length < 4) {
			if ("message" in current && typeof current.message === "string") messages.push(current.message)
			current = "cause" in current ? current.cause : undefined
		}

		return messages.some((message) => /error fetching consensus (update time|state)|consensus update time.*(?:missing|not found|prun)/i.test(message))
	}
}
