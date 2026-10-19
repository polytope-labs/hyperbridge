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
		/**
		 * True when the batch is not known to have been submitted at all: the bundler's answer
		 * was lost and every lookup failed too. The calls may or may not be in its pool, so a
		 * report must say "may have been submitted", not "was submitted".
		 */
		readonly maybeSubmitted = false,
	) {
		super(message)
		this.name = "SubmittedUnconfirmedError"
	}
}

/**
 * The bundler refused a new op because an op from the same sender is already pending on that
 * EntryPoint nonce ("replacement underpriced" or "already known"). Nothing new was submitted, but
 * the earlier op may still land, so a caller must not read this as a rejection and send the same
 * calls natively: that would pay for them twice. Retry once the earlier op has landed or expired.
 */
export class PendingOpOnNonceError extends Error {
	constructor(
		message: string,
		/** Source state machine id the op was meant for, e.g. "EVM-8453". */
		readonly chain: string,
		/** The EntryPoint nonce and key the refused op was signed with. */
		readonly nonce: bigint,
		readonly nonceKey: bigint,
	) {
		super(message)
		this.name = "PendingOpOnNonceError"
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
	/**
	 * The wallet nonce the tx was sent with. A node or mempool that no longer lists the tx proves
	 * nothing; the wallet's nonce moving past this one does.
	 */
	nonce: number
	/** When the transaction was sent, in ms. */
	submittedAt: number
}
