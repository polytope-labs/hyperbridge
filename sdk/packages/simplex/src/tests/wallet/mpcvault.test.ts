import * as grpc from "@grpc/grpc-js"
import { describe, expect, it, afterEach } from "vitest"
import { isHex } from "viem"
import {
	PlatformAPIService,
	CreateSigningRequestResponse,
	ExecuteSigningRequestsResponse,
	RejectSigningRequestResponse,
} from "@/proto/mpcvault/platform/v1/api"
import { MpcVaultService } from "@/services/wallet/mpcvault"
import type { Logger } from "@/services/Logger"

const HASH = "0x00000000000000000000000000000000000000000000000000000000000000aa" as const

type Handlers = Partial<Record<"createSigningRequest" | "executeSigningRequests" | "rejectSigningRequest", grpc.handleUnaryCall<unknown, unknown>>>

const cleanups: Array<() => void> = []

afterEach(() => {
	while (cleanups.length) cleanups.pop()?.()
})

async function startServer(handlers: Handlers): Promise<string> {
	const server = new grpc.Server()
	server.addService(PlatformAPIService, handlers as grpc.UntypedServiceImplementation)
	const port = await new Promise<number>((resolve, reject) =>
		server.bindAsync("127.0.0.1:0", grpc.ServerCredentials.createInsecure(), (err, bound) =>
			err ? reject(err) : resolve(bound),
		),
	)
	cleanups.push(() => server.forceShutdown())
	return `127.0.0.1:${port}`
}

function recordingLogger(): { logger: Logger; warns: string[] } {
	const warns: string[] = []
	const noop = (..._args: unknown[]) => {}
	const logger: Logger = {
		trace: noop,
		debug: noop,
		info: noop,
		warn: (...args: unknown[]) => {
			const msg = args.find((arg) => typeof arg === "string")
			warns.push(typeof msg === "string" ? msg : String(args[0]))
		},
		error: noop,
		fatal: noop,
	}
	return { logger, warns }
}

function makeService(target: string, logger: Logger): MpcVaultService {
	const service = new MpcVaultService({
		apiToken: "test-token",
		vaultUuid: "test-vault",
		accountAddress: "0x0000000000000000000000000000000000000001",
		callbackClientSignerPublicKey: "test-key",
		grpcTarget: target,
		credentials: grpc.credentials.createInsecure(),
		logger,
	})
	cleanups.push(() => service.close())
	return service
}

function createOk(uuid: string): grpc.handleUnaryCall<unknown, unknown> {
	return (_call, callback) =>
		callback(null, CreateSigningRequestResponse.fromPartial({ signingRequest: { uuid } }))
}

function executeOk(): grpc.handleUnaryCall<unknown, unknown> {
	return (_call, callback) =>
		callback(
			null,
			ExecuteSigningRequestsResponse.fromPartial({
				signatures: { signatures: [{ ecdsaSignature: { R: "1", S: "2", V: "27" } }] },
			}),
		)
}

function invalidUuidError(requestId?: string): grpc.ServiceError {
	const metadata = new grpc.Metadata()
	if (requestId) metadata.add("x-request-id", requestId)
	return Object.assign(new Error("Invalid uuid"), {
		code: grpc.status.INVALID_ARGUMENT,
		metadata,
	}) as grpc.ServiceError
}

describe("MpcVaultService", () => {
	it("signs after execute is retried past the create/execute race", async () => {
		let executeCalls = 0
		const target = await startServer({
			createSigningRequest: createOk("req-1"),
			executeSigningRequests: (_call, callback) => {
				executeCalls++
				if (executeCalls < 3) return callback(invalidUuidError(), null)
				executeOk()(_call, callback)
			},
		})
		const { logger, warns } = recordingLogger()
		const service = makeService(target, logger)

		const signature = await service.signRawHash(HASH)

		expect(isHex(signature)).toBe(true)
		expect(signature).toHaveLength(132)
		expect(executeCalls).toBe(3)
		expect(warns.filter((w) => w.includes("retrying"))).toHaveLength(2)
	})

	it("rejects the abandoned request and names the failure once retries exhaust", async () => {
		const rejected: string[] = []
		const target = await startServer({
			createSigningRequest: createOk("req-1"),
			executeSigningRequests: (_call, callback) => callback(invalidUuidError("test-req-id"), null),
			rejectSigningRequest: (call, callback) => {
				rejected.push((call.request as { uuid: string }).uuid)
				callback(null, RejectSigningRequestResponse.fromPartial({}))
			},
		})
		const { logger } = recordingLogger()
		const service = makeService(target, logger)

		await expect(service.signRawHash(HASH)).rejects.toThrow(
			/executeSigningRequests failed \(signing request req-1, x-request-id test-req-id\)/,
		)
		expect(rejected).toEqual(["req-1"])
	})

	it("fails on an app-level error that carries only a code", async () => {
		const target = await startServer({
			createSigningRequest: (_call, callback) =>
				callback(null, CreateSigningRequestResponse.fromPartial({ error: { message: "", serviceErrorCode: 1 } })),
		})
		const { logger } = recordingLogger()
		const service = makeService(target, logger)

		await expect(service.signRawHash(HASH)).rejects.toThrow(/createSigningRequest error: service error code 1/)
	})

	it("does not treat an UNSPECIFIED zero error code as a failure", async () => {
		const target = await startServer({
			createSigningRequest: (_call, callback) =>
				callback(
					null,
					CreateSigningRequestResponse.fromPartial({
						signingRequest: { uuid: "req-1" },
						error: { message: "", serviceErrorCode: 0, executeSigningRequestsErrorCode: 0 },
					}),
				),
			executeSigningRequests: executeOk(),
		})
		const { logger } = recordingLogger()
		const service = makeService(target, logger)

		const signature = await service.signRawHash(HASH)
		expect(isHex(signature)).toBe(true)
	})

	it("names createSigningRequest when create fails at the gRPC level", async () => {
		const target = await startServer({
			createSigningRequest: (_call, callback) =>
				callback(
					Object.assign(new Error("boom"), { code: grpc.status.UNAVAILABLE }) as grpc.ServiceError,
					null,
				),
		})
		const { logger } = recordingLogger()
		const service = makeService(target, logger)

		await expect(service.signRawHash(HASH)).rejects.toThrow(/createSigningRequest failed/)
	})
})
