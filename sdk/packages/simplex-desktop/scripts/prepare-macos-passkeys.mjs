import { createRequire } from "node:module"
const { prepareMacPasskeySigning } = createRequire(import.meta.url)("./mac-passkey-signing.cjs")
const identity = prepareMacPasskeySigning(process.env)
process.stdout.write(
	JSON.stringify(
		{
			rpId: identity.rpId,
			applicationIdentifier: identity.applicationIdentifier,
			associationFile: identity.associationFile,
		},
		null,
		2,
	) + "\n",
)
