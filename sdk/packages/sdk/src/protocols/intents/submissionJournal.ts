import type { HexString, SelectBidResult } from "@/types"
import { CryptoUtils } from "./CryptoUtils"

export type SubmissionScope = {
	chainId: number
	gateway: HexString
	entryPoint: HexString
	commitment: HexString
}
export type PendingSubmission = SubmissionScope & { version: 1; submission: SelectBidResult }
type Storage = {
	getItem(key: string): Promise<string | null | undefined>
	setItem(key: string, value: string): Promise<unknown>
}

/** A single-writer journal. The backend has no cross-process compare-and-swap. */
export class SubmissionJournal {
	readonly key: string
	constructor(
		private readonly storage: Storage,
		private readonly scope: SubmissionScope,
	) {
		this.key = `pending-submission:v1:${scope.chainId}:${scope.gateway.toLowerCase()}:${scope.entryPoint.toLowerCase()}:${scope.commitment.toLowerCase()}`
	}
	async read(): Promise<PendingSubmission | undefined> {
		const raw = await this.storage.getItem(this.key)
		if (raw == null || raw === "null") return undefined
		try {
			const value = JSON.parse(raw)
			if (value.version !== 1 || value.chainId !== this.scope.chainId) throw new Error("version or chain")
			for (const field of ["gateway", "entryPoint", "commitment"] as const) {
				if (value[field]?.toLowerCase() !== this.scope[field].toLowerCase()) throw new Error(field)
			}
			const submission = value.submission
			const op = submission.userOp
			const hex = (value: unknown, bytes?: number) =>
				typeof value === "string" &&
				new RegExp(bytes === undefined ? "^0x(?:[0-9a-fA-F]{2})*$" : `^0x[0-9a-fA-F]{${bytes * 2}}$`).test(
					value,
				)
			if (
				!hex(op.sender, 20) ||
				!hex(submission.userOpHash, 32) ||
				!hex(submission.solverAddress, 20) ||
				submission.commitment?.toLowerCase() !== this.scope.commitment.toLowerCase() ||
				submission.solverAddress.toLowerCase() !== op.sender.toLowerCase()
			)
				throw new Error("identity")
			for (const field of ["initCode", "callData", "paymasterAndData", "signature"])
				if (!hex(op[field])) throw new Error(field)
			if (op.signature === "0x" || !hex(op.accountGasLimits, 32) || !hex(op.gasFees, 32))
				throw new Error("packed operation")
			for (const field of ["nonce", "preVerificationGas"]) {
				if (typeof op[field] !== "string" || !/^(0|[1-9][0-9]*)$/.test(op[field])) throw new Error(field)
				op[field] = BigInt(op[field])
				if (op[field] >= 1n << 256n) throw new Error(field)
			}
			if (
				CryptoUtils.computeUserOpHash(op, this.scope.entryPoint, BigInt(this.scope.chainId)).toLowerCase() !==
				submission.userOpHash.toLowerCase()
			)
				throw new Error("operation hash")
			// Pending records describe the signed operation, never unverified outcome metadata.
			return {
				...this.scope,
				version: 1,
				submission: {
					userOp: op,
					userOpHash: submission.userOpHash,
					solverAddress: submission.solverAddress,
					commitment: submission.commitment,
				},
			}
		} catch (error) {
			throw new Error(
				`Pending submission journal is corrupt; restore the durable record before resuming: ${error instanceof Error ? error.message : String(error)}`,
			)
		}
	}
	async write(submission: SelectBidResult): Promise<void> {
		const existing = await this.read()
		if (
			existing &&
			(existing.submission.userOpHash !== submission.userOpHash ||
				existing.submission.userOp.signature !== submission.userOp.signature)
		)
			throw new Error("An unresolved submission already exists")
		await this.storage.setItem(
			this.key,
			JSON.stringify({ ...this.scope, version: 1, submission }, (_key, value) =>
				typeof value === "bigint" ? value.toString() : value,
			),
		)
	}
	/** Call only after the terminal hash has been persisted. A tombstone works with get/set-only backends. */
	async clear(): Promise<void> {
		await this.storage.setItem(this.key, "null")
	}
}
