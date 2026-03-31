import { Enum, Struct, Vector, u8 } from "scale-ts"

/**
 * Revive pallet account layout for child-trie proofs — mirrors
 * `ContractInfo`, `AccountType`, and `AccountInfo` in
 * `modules/ismp/state-machines/evm/src/substrate_evm.rs`.
 */
export const ReviveContractInfo = Struct({
	trie_id: Vector(u8),
})

export const ReviveAccountType = Enum({
	Contract: ReviveContractInfo,
})

export const ReviveAccountInfo = Struct({
	account_type: ReviveAccountType,
})

/**
 * Decode SCALE-encoded `AccountInfo` and return `ContractInfo::trie_id`.
 * Fails at decode time if bytes are not a valid `AccountInfo` or not `AccountType::Contract`.
 */
export function decodeReviveContractTrieId(accountData: Uint8Array): Uint8Array {
	const {
		account_type: { value },
	} = ReviveAccountInfo.dec(accountData)
	return value.trie_id
}
