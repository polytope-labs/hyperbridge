import { createHash, generateKeyPairSync, randomBytes, sign } from "node:crypto"
import { isoCBOR } from "@simplewebauthn/server/helpers"
import type { PasskeyCredential } from "../../security/passkeys"

const hash = (data: string | Buffer) => createHash("sha256").update(data).digest()
export function authenticator() {
	const { privateKey, publicKey } = generateKeyPairSync("ec", { namedCurve: "prime256v1" })
	const jwk = publicKey.export({ format: "jwk" })
	const id = randomBytes(32)
	const cose = Buffer.from(
		isoCBOR.encode(
			new Map<number, number | Uint8Array>([
				[1, 2],
				[3, -7],
				[-1, 1],
				[-2, Buffer.from(jwk.x!, "base64url")],
				[-3, Buffer.from(jwk.y!, "base64url")],
			]),
		),
	)
	const credential: PasskeyCredential = {
		id: id.toString("base64url"),
		publicKey: cose.toString("base64url"),
		counter: 0,
		transports: ["internal"],
	}
	function response(
		kind: string,
		challenge: string,
		origin: string,
		{
			rp = "localhost",
			uv = true,
			counter = 1,
			transports = ["internal"],
			userHandle = undefined as string | undefined,
		} = {},
	) {
		const clientData = Buffer.from(
			JSON.stringify({ type: kind === "register" ? "webauthn.create" : "webauthn.get", challenge, origin }),
		)
		const header = Buffer.alloc(5)
		header[0] = 1 | (uv ? 4 : 0) | (kind === "register" ? 64 : 0)
		header.writeUInt32BE(kind === "register" ? 0 : counter, 1)
		const base = Buffer.concat([hash(rp), header])
		const length = Buffer.alloc(2)
		length.writeUInt16BE(id.length)
		const authData = kind === "register" ? Buffer.concat([base, Buffer.alloc(16), length, id, cose]) : base
		const common = {
			id: credential.id,
			rawId: credential.id,
			type: "public-key",
			authenticatorAttachment: "platform",
			clientExtensionResults: {},
		}
		return {
			...common,
			response:
				kind === "register"
					? {
							clientDataJSON: clientData.toString("base64url"),
							transports,
							attestationObject: Buffer.from(
								isoCBOR.encode(
									new Map<string, unknown>([
										["fmt", "none"],
										["attStmt", new Map()],
										["authData", authData],
									]) as Parameters<typeof isoCBOR.encode>[0],
								),
							).toString("base64url"),
						}
					: {
							clientDataJSON: clientData.toString("base64url"),
							authenticatorData: authData.toString("base64url"),
							userHandle,
							signature: sign("sha256", Buffer.concat([authData, hash(clientData)]), privateKey).toString(
								"base64url",
							),
						},
		}
	}
	return { credential, response }
}
