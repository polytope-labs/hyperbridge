// Local stand-in for the upstream `@subql/types-solana` payload shapes.
// Replace these imports with `@subql/types-solana` once that dep is added
// to package.json and `pnpm install` runs.

export interface SolanaBlock {
	blockHeight: number
	slot: number
	blockTime: number | null
	blockhash: string
	parentSlot: number
}

export interface SolanaTransaction {
	signature: string
	slot: number
	err: unknown
}

export interface SolanaAccount {
	pubkey: string
	isSigner: boolean
	isWritable: boolean
}

export interface SolanaInstruction {
	programId: string
	// 8-byte discriminator + Borsh-encoded params.
	data: Uint8Array
	accounts: SolanaAccount[]
	block: SolanaBlock
	transaction: SolanaTransaction
	transactionIndex: number
	instructionIndex: number
}
