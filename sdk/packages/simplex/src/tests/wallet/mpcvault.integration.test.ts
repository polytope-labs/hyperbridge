import type { HexString } from "@hyperbridge/sdk"
import { createMpcVaultAccount, MpcVaultService } from "@/services/wallet/mpcvault"
import { describe, expect, it } from "vitest"
import { isHex } from "viem"
import "../setup"

/**
 * Live MPCVault API integration tests
 *
 * These tests hit the real MPC Vault API and require a running Docker client signer
 * to complete the MPC signing ceremony.
 *
 * ── Prerequisites ──
 *
 * 1. Generate an Ed25519 keypair (if not already done):
 *      ssh-keygen -t ed25519 -C "mpcvault-client-signer" -f ./client-signer-key -N ""
 *
 * 2. Register the PUBLIC key (client-signer-key.pub) in the MPC Vault console:
 *      Dashboard → Team & Policies → New Client Signer
 *    Approve both "Vault setting update" and "Grant key access" in the mobile app.
 *
 * 3. Whitelist your IP in the MPC Vault console (Settings → IP Whitelist).
 *
 * 4. Create the Docker client signer config:
 *      mkdir -p ~/mpcvault
 *      cat > ~/mpcvault/config.yml << EOF
 *      vault-uuid: "<your-vault-uuid>"
 *      ssh:
 *        private-key: |
 *          <contents of client-signer-key, indented 4 spaces>
 *        password: ""
 *      callback-url: "http://host.docker.internal:8088/callback"
 *      http-health:
 *        listening-addr: 0.0.0.0:8080
 *      EOF
 *    Note: On Linux servers, use "http://localhost:8088/callback" with --network host
 *    instead of "http://host.docker.internal:8088/callback".
 *
 * 5. Run the client signer Docker container:
 *      docker pull ghcr.io/mpcvault/client-signer:latest
 *      docker run -d --name mpcvault-signer \
 *        -v ~/mpcvault/config.yml:/data/config.yml \
 *        -p 8080:8080 \
 *        ghcr.io/mpcvault/client-signer:latest
 *    Verify with: docker logs mpcvault-signer (should show "switch to CONNECTED")
 *
 * 6. Run a callback server that auto-approves all signing requests:
 *      python3 -c "
 *      from http.server import HTTPServer, BaseHTTPRequestHandler
 *      class H(BaseHTTPRequestHandler):
 *          def do_POST(self):
 *              self.rfile.read(int(self.headers.get('Content-Length',0)))
 *              self.send_response(200); self.end_headers(); self.wfile.write(b'approved')
 *      HTTPServer(('0.0.0.0', 8088), H).serve_forever()
 *      " &
 *    Verify with: curl -X POST http://localhost:8088/callback
 *
 * 7. Set environment variables in .env.local:
 *      MPCVAULT_API_TOKEN=<your-api-token>
 *      MPCVAULT_VAULT_UUID=<your-vault-uuid>
 *      MPCVAULT_ACCOUNT_ADDRESS=<your-wallet-address>
 *      MPCVAULT_CALLBACK_CLIENT_SIGNER_PUBLIC_KEY=<contents of client-signer-key.pub>
 *
 * Optional (defaults: chain 1, tx nonce 0):
 *      MPCVAULT_TEST_CHAIN_ID=11155111
 *      MPCVAULT_TEST_TX_NONCE=0
 *
 * Tests are skipped automatically if the env vars above are not set.
 */
function hasMpcVaultCredentials(): boolean {
	return Boolean(
		process.env.MPCVAULT_API_TOKEN &&
			process.env.MPCVAULT_VAULT_UUID &&
			process.env.MPCVAULT_ACCOUNT_ADDRESS &&
			process.env.MPCVAULT_CALLBACK_CLIENT_SIGNER_PUBLIC_KEY,
	)
}

function createTestService(): MpcVaultService {
	return new MpcVaultService({
		apiToken: process.env.MPCVAULT_API_TOKEN as string,
		vaultUuid: process.env.MPCVAULT_VAULT_UUID as string,
		accountAddress: process.env.MPCVAULT_ACCOUNT_ADDRESS as HexString,
		callbackClientSignerPublicKey: process.env.MPCVAULT_CALLBACK_CLIENT_SIGNER_PUBLIC_KEY as string,
	})
}

function testChainId(): number {
	return Number.parseInt(process.env.MPCVAULT_TEST_CHAIN_ID ?? "1", 10)
}

function testTxNonce(): number {
	return Number.parseInt(process.env.MPCVAULT_TEST_TX_NONCE ?? "0", 10)
}

describe.skipIf(!hasMpcVaultCredentials())("MPCVaultService integration", () => {
	it("getAccountAddress returns the configured account address", () => {
		const service = createTestService()
		expect(service.getAccountAddress().toLowerCase()).toBe(
			(process.env.MPCVAULT_ACCOUNT_ADDRESS as string).toLowerCase(),
		)
	})

	it("signRawHash completes create + execute and returns a 65-byte ECDSA hex signature", async () => {
		const service = createTestService()

		const dummyHash = `0x${"ab".repeat(32)}` as HexString
		const sig = await service.signRawHash(dummyHash)

		expect(isHex(sig)).toBe(true)
		expect(sig.length).toBe(132)
	}, 120_000)

	it("signRawHashComponents returns r, s, yParity for the same raw hash flow", async () => {
		const service = createTestService()
		const dummyHash = `0x${"ba".repeat(32)}` as HexString
		const { r, s, yParity } = await service.signRawHashComponents(dummyHash)

		expect(isHex(r)).toBe(true)
		expect(isHex(s)).toBe(true)
		expect(r.length).toBe(66)
		expect(s.length).toBe(66)
		expect(yParity === 0 || yParity === 1).toBe(true)
	}, 120_000)

	it("signPersonalMessage completes and returns a 65-byte ECDSA hex signature", async () => {
		const service = createTestService()

		// A dummy 32-byte message hash, as if from keccak256("hello")
		const messageHash = `0x${"cd".repeat(32)}` as HexString
		const chainId = testChainId()

		const sig = await service.signPersonalMessage(messageHash, chainId)

		expect(isHex(sig)).toBe(true)
		expect(sig.length).toBe(132) // 65 bytes = 130 hex chars + 0x prefix
	}, 120_000)

	it("signTypedData completes and returns a 65-byte ECDSA hex signature", async () => {
		const service = createTestService()
		const chainId = testChainId()

		const typedData = {
			types: {
				EIP712Domain: [
					{ name: "name", type: "string" },
					{ name: "version", type: "string" },
					{ name: "chainId", type: "uint256" },
				],
				Test: [{ name: "value", type: "uint256" }],
			},
			primaryType: "Test",
			domain: {
				name: "TestDomain",
				version: "1",
				chainId,
			},
			message: {
				value: 12345,
			},
		}

		const sig = await service.signTypedData(JSON.stringify(typedData), chainId)

		expect(isHex(sig)).toBe(true)
		expect(sig.length).toBe(132)
	}, 120_000)

	it("signTransaction completes and returns a signed transaction hex", async () => {
		const service = createTestService()
		const chainId = testChainId()

		const sig = await service.signTransaction({
			chainId,
			to: "0x000000000000000000000000000000000000dEaD" as HexString,
			value: 0n,
			data: "0x" as HexString,
			nonce: testTxNonce(),
			gasLimit: 21000n,
			maxFeePerGas: 1000000000n,
			maxPriorityFeePerGas: 1000000n,
		})

		expect(isHex(sig)).toBe(true)
		expect(sig.length).toBeGreaterThan(2) // at least "0x" + some data
	}, 120_000)

	it("createMpcVaultAccount exposes matching addresses and signMessage rejects without chain context", async () => {
		const accountAddress = process.env.MPCVAULT_ACCOUNT_ADDRESS as HexString
		const { account, service } = createMpcVaultAccount({
			apiToken: process.env.MPCVAULT_API_TOKEN as string,
			vaultUuid: process.env.MPCVAULT_VAULT_UUID as string,
			accountAddress,
			callbackClientSignerPublicKey: process.env.MPCVAULT_CALLBACK_CLIENT_SIGNER_PUBLIC_KEY as string,
		})

		expect(account.address.toLowerCase()).toBe(accountAddress.toLowerCase())
		expect(service.getAccountAddress().toLowerCase()).toBe(accountAddress.toLowerCase())

		const dummyHash = `0x${"ef".repeat(32)}` as HexString
		expect(account.signMessage).toBeDefined()
		await expect(account.signMessage!({ message: { raw: dummyHash } })).rejects.toThrow(
			/MPCVault does not support signMessage without chain context/,
		)
	})
})
