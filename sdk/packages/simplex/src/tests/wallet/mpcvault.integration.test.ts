import type { HexString } from "@hyperbridge/sdk"
import { MpcVaultService } from "@/services/wallet/mpcvault"
import { mpcVaultSigner } from "@/services/wallet/accounts/mpc"
import { describe, expect, it } from "vitest"
import {
	isHex,
	parseTransaction,
	recoverAddress,
	recoverMessageAddress,
	recoverTransactionAddress,
	recoverTypedDataAddress,
} from "viem"
import { hashAuthorization } from "viem/utils"
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

/** The signer the solver runs on, built from the same credentials. */
function createTestSigner() {
	return mpcVaultSigner({
		apiToken: process.env.MPCVAULT_API_TOKEN as string,
		vaultUuid: process.env.MPCVAULT_VAULT_UUID as string,
		accountAddress: process.env.MPCVAULT_ACCOUNT_ADDRESS as HexString,
		callbackClientSignerPublicKey: process.env.MPCVAULT_CALLBACK_CLIENT_SIGNER_PUBLIC_KEY as string,
	})
}

/** Arbitrary delegate target for the authorization checks. */
const DELEGATE_TARGET = "0x0000000000000000000000000000000000000001" as HexString

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
		const recovered = await recoverAddress({ hash: dummyHash, signature: sig })
		expect(recovered.toLowerCase()).toBe(service.getAccountAddress().toLowerCase())
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
		const recovered = await recoverAddress({ hash: dummyHash, signature: { r, s, yParity } })
		expect(recovered.toLowerCase()).toBe(service.getAccountAddress().toLowerCase())
	}, 120_000)

	it("signPersonalMessage completes and returns a 65-byte ECDSA hex signature", async () => {
		const service = createTestService()

		// A dummy 32-byte message hash, as if from keccak256("hello")
		const messageHash = `0x${"cd".repeat(32)}` as HexString
		const chainId = testChainId()

		const sig = await service.signPersonalMessage(messageHash, chainId)

		expect(isHex(sig)).toBe(true)
		expect(sig.length).toBe(132) // 65 bytes = 130 hex chars + 0x prefix
		const recovered = await recoverMessageAddress({ message: { raw: messageHash }, signature: sig })
		expect(recovered.toLowerCase()).toBe(service.getAccountAddress().toLowerCase())
	}, 120_000)

	// Recovery assertion is the load-bearing check: MPCVault has returned well-formed
	// typed-data signatures that verify against nothing (#1132), which the previous
	// length-only assertion could not catch. This payload is the canonical v4 shape
	// (EIP712Domain listed in types, numeric chainId) that packedUserOpTypedData now emits.
	it("signTypedData returns a signature that recovers the wallet address", async () => {
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
		const recovered = await recoverTypedDataAddress({
			...typedData,
			signature: sig,
		} as unknown as Parameters<typeof recoverTypedDataAddress>[0])
		expect(recovered.toLowerCase()).toBe(service.getAccountAddress().toLowerCase())
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
		const recovered = await recoverTransactionAddress({ serializedTransaction: sig as never })
		expect(recovered.toLowerCase()).toBe((process.env.MPCVAULT_ACCOUNT_ADDRESS as string).toLowerCase())
	}, 120_000)

	// The three operations the solver actually calls, through the signer that ships.
	it("mpcVaultSigner signs typed data that recovers the wallet address", async () => {
		const signer = createTestSigner()
		// `EIP712Domain` must be listed: MPCVault hashes the payload server-side from
		// this JSON, and viem omits the domain type when it hashes locally. Leave it
		// out and the vault signs a different digest than the one recovery checks
		// against — the failure #1134 was about.
		const typedData = {
			types: {
				EIP712Domain: [
					{ name: "name", type: "string" },
					{ name: "version", type: "string" },
					{ name: "chainId", type: "uint256" },
				],
				Bid: [{ name: "amount", type: "uint256" }],
			},
			primaryType: "Bid",
			domain: { name: "Simplex", version: "1", chainId: testChainId() },
			message: { amount: 1n },
		}

		const signature = await signer.signTypedData(typedData)
		const recovered = await recoverTypedDataAddress({ ...typedData, signature } as never)
		expect(recovered.toLowerCase()).toBe(signer.address.toLowerCase())
	}, 120_000)

	// MPCVault has no structured EIP-7702 encoding, so the adapter hashes the tuple
	// itself — this pins that the hash it signs is the one the protocol defines.
	it("mpcVaultSigner signs an EIP-7702 authorization that recovers the wallet address", async () => {
		const signer = createTestSigner()
		const auth = { chainId: testChainId(), contractAddress: DELEGATE_TARGET, nonce: 0 }

		const { r, s: sHex, yParity } = await signer.signAuthorization(auth)
		expect([0, 1]).toContain(yParity)

		const recovered = await recoverAddress({
			hash: hashAuthorization({ address: DELEGATE_TARGET, chainId: auth.chainId, nonce: auth.nonce }),
			signature: { r: r as `0x${string}`, s: sHex as `0x${string}`, v: BigInt(yParity) + 27n },
		})
		expect(recovered.toLowerCase()).toBe(signer.address.toLowerCase())
	}, 120_000)

	// The structured path: the vault sees to/value/data, not a digest. It also
	// builds the transaction itself, so this checks the bytes that come back are
	// the transaction we asked for, signed by the account we asked to sign it.
	it("mpcVaultSigner signs a transaction through the vault's structured request", async () => {
		const signer = createTestSigner()
		// Non-zero value on purpose: RLP encodes 0 as empty bytes and viem parses it
		// back as undefined, so a zero here would assert nothing about the field
		// surviving the vault. The transaction is signed, never broadcast.
		const request = {
			chainId: testChainId(),
			to: signer.address,
			value: 1_000n,
			nonce: testTxNonce(),
			gas: 21_000n,
			maxFeePerGas: 1_000_000_000n,
			maxPriorityFeePerGas: 1_000_000n,
		}

		const serialized = await signer.signTransaction(request)
		expect(isHex(serialized)).toBe(true)

		const parsed = parseTransaction(serialized as never)
		expect(parsed.chainId).toBe(request.chainId)
		expect(parsed.nonce).toBe(request.nonce)
		expect(parsed.to?.toLowerCase()).toBe(request.to.toLowerCase())
		expect(parsed.value).toBe(request.value)

		const recovered = await recoverTransactionAddress({ serializedTransaction: serialized as never })
		expect(recovered.toLowerCase()).toBe(signer.address.toLowerCase())
	}, 120_000)

	// The one transaction MPCVault's structured request cannot carry: the adapter
	// serialises and raw-signs it instead, or the delegation is silently dropped.
	it("mpcVaultSigner serialises and signs a set-code transaction", async () => {
		const signer = createTestSigner()
		const chainId = testChainId()
		const auth = await signer.signAuthorization({ chainId, contractAddress: DELEGATE_TARGET, nonce: 1 })

		const serialized = await signer.signTransaction({
			chainId,
			type: "eip7702",
			to: signer.address,
			value: 0n,
			nonce: testTxNonce(),
			gas: 650_000n,
			maxFeePerGas: 1_000_000_000n,
			maxPriorityFeePerGas: 1_000_000n,
			authorizationList: [{ chainId, address: DELEGATE_TARGET, nonce: 1, ...auth }],
		})

		// Type-0x04 envelope, signed locally from the vault's raw-hash signature.
		expect(serialized.startsWith("0x04")).toBe(true)

		// The delegation has to survive into the signed bytes — the whole reason
		// this branch exists — and the signature has to be the solver's.
		const parsed = parseTransaction(serialized as never)
		expect(parsed.authorizationList).toHaveLength(1)
		expect(parsed.authorizationList?.[0]?.address?.toLowerCase()).toBe(DELEGATE_TARGET.toLowerCase())
		expect(parsed.chainId).toBe(chainId)

		const recovered = await recoverTransactionAddress({ serializedTransaction: serialized as never })
		expect(recovered.toLowerCase()).toBe(signer.address.toLowerCase())
	}, 120_000)
})
