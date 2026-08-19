import type { HexString } from "@hyperbridge/sdk"
import { privateKeyToAccount } from "viem/accounts"
import type { Signer } from "../types"
import { viemSigner } from "./viem"

/**
 * Signs with a raw private key held in this process. The whole implementation is
 * the viem adapter over `privateKeyToAccount` — nothing about it is privileged.
 */
export function privateKeySigner(privateKey: HexString): Signer {
	return viemSigner(privateKeyToAccount(privateKey))
}
