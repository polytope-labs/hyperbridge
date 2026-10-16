import type { HexString } from "@hyperbridge/sdk"

/**
 * A batch that was submitted — accepted by the bundler, or a signed native transaction the node
 * took — but whose receipt never arrived. It is not a plain failure: the calls may still land, so
 * a caller must not conclude that nothing was escrowed, and must not fall back to a second send.
 *
 * Kept in its own module so both senders can throw it without importing each other. The message
 * text is unchanged from before this type existed: callers that catch a generic error (the UI's
 * send endpoint, delegation setup, vault funding) keep the message they relied on.
 */
export class SubmittedUnconfirmedError extends Error {
	constructor(
		message: string,
		/** Source state machine id the batch was submitted on, e.g. "EVM-8453". */
		readonly chain: string,
		readonly submission: SubmittedUserOp | SubmittedTx,
	) {
		super(message)
		this.name = "SubmittedUnconfirmedError"
	}
}

/** A sponsored UserOp the bundler accepted but had no receipt for. */
export interface SubmittedUserOp {
	kind: "userOp"
	userOpHash: HexString
	/** The EntryPoint nonce and nonce key the op was signed with, for a later `getNonce` check. */
	nonce: bigint
	nonceKey: bigint
	/**
	 * Whether the paymaster attached a validity deadline. A Permit2-mode op does, so once its
	 * permit deadline has passed it can no longer land; a PERMIT bootstrap op does not, so it can
	 * land any time until its EntryPoint nonce is consumed.
	 */
	expires: boolean
	/** When the bundler accepted the op, in ms. */
	submittedAt: number
}

/** A native transaction the node accepted but that got no receipt within the wait. */
export interface SubmittedTx {
	kind: "tx"
	txHash: HexString
	/** When the transaction was sent, in ms. */
	submittedAt: number
}
