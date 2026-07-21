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

/** Exact RPC message for a consensus update that Hyperbridge no longer retains. */
export const MISSING_CONSENSUS_UPDATE_TIME_MESSAGE = "Consensus update time not found"

/**
 * Hyperbridge no longer retains the consensus update for a state-machine
 * height. Proofs derived from that height can never be submitted again.
 */
export class MissingConsensusUpdateTimeError extends Error {
	constructor(message = MISSING_CONSENSUS_UPDATE_TIME_MESSAGE, options?: { cause?: unknown }) {
		super(message, options)
		this.name = "Hyperbridge/SDK/MissingConsensusUpdateTimeError"
	}

	static isError(error: unknown): error is MissingConsensusUpdateTimeError {
		return (
			error instanceof MissingConsensusUpdateTimeError ||
			(error instanceof Error &&
				error.message.toLowerCase().includes(MISSING_CONSENSUS_UPDATE_TIME_MESSAGE.toLowerCase()))
		)
	}
}
