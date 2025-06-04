/* tslint:disable */
/* eslint-disable */
export function generate_root_with_proof(calldata_bytes: Uint8Array, tree_size: bigint): string;
export function verify_proof(root_hex: string, proof_hex: string[], mmr_size: bigint, leaf_position: bigint, calldata_bytes: Uint8Array): boolean;
export class KeccakMerge {
  private constructor();
  free(): void;
}
