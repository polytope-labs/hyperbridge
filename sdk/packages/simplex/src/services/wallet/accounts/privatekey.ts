import type { HexString } from "@hyperbridge/sdk"
import { privateKeyToAccount, sign } from "viem/accounts"
import type { SigningAccount } from "../types"

export function createPrivateKeySigningAccount(privateKey: HexString): SigningAccount {
	const account = privateKeyToAccount(privateKey)
	const signRawHash = async (hash: HexString) => {
		const signature = await sign({
			hash,
			privateKey,
		})
		const yParity =
			signature.yParity ??
			(signature.v !== undefined ? Number(signature.v >= 27n ? signature.v - 27n : signature.v) : undefined)
		if (yParity !== 0 && yParity !== 1) {
			throw new Error("Failed to derive yParity from private key signature")
		}
		return {
			r: signature.r as HexString,
			s: signature.s as HexString,
			yParity,
		}
	}
	return {
		mode: "privateKey",
		account,
		signMessage: (messageHash: HexString, _chainId: number) =>
			account.signMessage({ message: { raw: messageHash } }),
		signRawHash,
		sendEip7702DelegationTransaction: async (args) =>
			(await args.walletClient.sendTransaction({
				to: args.authorityAddress,
				value: 0n,
				authorizationList: [args.authorization],
				chain: args.walletClient.chain,
			})) as HexString,
	}
}
