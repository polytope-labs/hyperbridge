import { spawn, type ChildProcess } from "node:child_process"
import { mkdtempSync, writeFileSync } from "node:fs"
import { createServer } from "node:net"
import { tmpdir } from "node:os"
import { fileURLToPath } from "node:url"
import { join } from "node:path"
import { afterAll, beforeAll, describe, expect, it } from "vitest"
import type { HexString } from "@hyperbridge/sdk"
import { MemoryDataStore } from "@/data/memory"
import { OrderbookClient } from "@/orderbook/client"
import { LimitOrderService } from "@/orderbook/limit-orders"
import { ORDERBOOK_SCALE } from "@/orderbook/amounts"
import { BASE_CHAIN, baseAssetRegistry, postingRig } from "../helpers/posting"

/**
 * One limit order through a running HyperFX orderbook, start to finish.
 *
 * Skipped unless the environment says where to find one:
 *
 *   HYPERFX_ORDERBOOK_URL=http://127.0.0.1:8080/graphql   a server already running
 *   HYPERFX_ORDERBOOK_BIN=/path/to/hyperfx-orderbook      a binary to run on a config written here
 *
 * The binary comes from polytope-labs/hyperfx-orderbook:
 * `cargo build --release -p hyperfx-server --bin hyperfx-orderbook`. CI runs the
 * published image instead and passes the URL, which is what
 * `.github/workflows/test-simplex-orderbook.yml` does.
 *
 * Nothing else here talks to a real server, so this is the only test that can
 * fail on what the schema check cannot see: an argument the server reads
 * differently, an op it verifies for itself, an order it files under a book we
 * did not expect. It needs no chain. The config below runs the server in the
 * mode its own `config.dev.toml` uses, where the protocol fee is a constant and
 * the validation cycle reads nothing, so an order surfaces at its quoted size
 * without a balance behind it.
 */

const RUNNING = process.env.HYPERFX_ORDERBOOK_URL
const BINARY = process.env.HYPERFX_ORDERBOOK_BIN
const GATEWAY = "0xAe041F7B0CB581876832830baeB6a2Aa2a3C9716" as HexString
const CHAIN_ID = 8453
/** EntryPoint v0.8, which is what the solver account validates against. */
const ENTRY_POINT = "0x4337084D9E255Ff0702461CF8895CE9E3b5Ff108" as HexString
const ONE = ORDERBOOK_SCALE

/** Take in 1,500,000 cNGN, pay out 1,000.5 USDC: a cNGN to USDC order at about 1,499. */
const REQUEST = {
	fillChain: BASE_CHAIN,
	tokenIn: "cNGN",
	amountIn: "1500000",
	tokenOut: "USDC",
	amountOut: "1000.5",
	acceptedSources: [BASE_CHAIN, "EVM-1"],
}

/** The stub this repo ships, which CI runs the same way. */
function fakeIndexer(): string {
	return fileURLToPath(new URL("../../../scripts/fake-indexer.mjs", import.meta.url))
}

function freePort(): Promise<number> {
	return new Promise((resolve, reject) => {
		const probe = createServer()
		probe.on("error", reject)
		probe.listen(0, "127.0.0.1", () => {
			const address = probe.address()
			const port = typeof address === "object" && address !== null ? address.port : 0
			probe.close(() => resolve(port))
		})
	})
}

/** One USDC/cNGN book on two chains, with the indexer and both gateways stubbed. */
function configFor(port: number, directory: string, indexer: string): string {
	return `
[server]
listen = "127.0.0.1:${port}"
database = "sqlite://${join(directory, "orderbook.db")}"
max_orders_per_solver = 200
max_unvalidated_orders_per_solver = 100
max_batch_size = 50

[lifecycle]
min_order_ttl_secs = 900
sweep_interval_secs = 10
heartbeat_interval_secs = 60
signature_skew_secs = 60

[indexer]
url = "${indexer}/graphql"

[[pairs]]
base = "USDC"
quote = "cNGN"
price_granularity = "1"

[min_order_size]
cNGN = 100
USDC = 100

[chains."EVM-8453"]
rpc_url = "${indexer}/rpc/EVM-8453"
gateway = "${GATEWAY}"
solver_accounts = ["0x7cb55539d1144F62422099c3FA3405092022c88C"]

[chains."EVM-1"]
rpc_url = "${indexer}/rpc/EVM-1"
gateway = "${GATEWAY}"
solver_accounts = ["0x7cb55539d1144F62422099c3FA3405092022c88C"]
`
}

/** Waits for the server to answer a real query rather than merely accept a socket. */
async function awaitReady(client: OrderbookClient, deadlineMs: number): Promise<void> {
	const until = Date.now() + deadlineMs
	let last: unknown
	while (Date.now() < until) {
		try {
			await client.limits()
			return
		} catch (err) {
			last = err
			await new Promise((resolve) => setTimeout(resolve, 250))
		}
	}
	throw new Error(`The orderbook never came up: ${last}`)
}

describe.skipIf(!RUNNING && !BINARY)("a limit order on a running orderbook", () => {
	let server: ChildProcess | undefined
	let indexer: ChildProcess | undefined
	let client: OrderbookClient
	let service: LimitOrderService
	let store: MemoryDataStore

	beforeAll(async () => {
		let url = RUNNING
		if (!url) {
			const directory = mkdtempSync(join(tmpdir(), "hyperfx-"))
			const [port, stubPort] = [await freePort(), await freePort()]
			indexer = spawn(process.execPath, [fakeIndexer(), "--port", String(stubPort)], { stdio: "ignore" })
			const config = join(directory, "config.toml")
			writeFileSync(config, configFor(port, directory, `http://127.0.0.1:${stubPort}`))
			server = spawn(BINARY!, ["--config", config], { stdio: "ignore" })
			url = `http://127.0.0.1:${port}/graphql`
		}

		client = new OrderbookClient(url, 10_000)
		await awaitReady(client, 30_000)

		store = new MemoryDataStore()
		const { service: contractService, signer } = await postingRig({ gateway: GATEWAY, chainId: CHAIN_ID })
		service = new LimitOrderService(
			store.limitOrders,
			client,
			contractService,
			// biome-ignore lint/suspicious/noExplicitAny: the chain and the EntryPoint are all this path reads
			{ getConfiguredChainIds: () => [CHAIN_ID], getEntryPointAddress: () => ENTRY_POINT } as any,
			// biome-ignore lint/suspicious/noExplicitAny: two symbols on one chain
			baseAssetRegistry() as any,
			signer,
			900,
		)
	}, 60_000)

	afterAll(() => {
		server?.kill()
		indexer?.kill()
	})

	it("is posted, surfaced, resized and withdrawn", async () => {
		const limits = await client.limits()
		expect(limits.serverInfo.minOrderTtlSecs).toBe(900)
		expect(limits.books.map((book) => book.id)).toContain("USDC-cNGN")

		const { order, result } = await service.create(REQUEST)
		expect(result.kind).toBe("accepted")
		expect(order.status).toBe("open")
		expect(order.commitment).not.toBeNull()
		// The orderbook shades a posting by the protocol fee, which is zero here.
		expect(order.bookPrice).not.toBeNull()

		const posted = await client.myOrders(signerAddress(service))
		expect(posted.orders.map((entry) => entry.commitment)).toEqual([order.commitment])

		const beat = await service.heartbeat()
		expect(beat).toMatchObject({ kind: "accepted", status: "ACTIVE" })

		// A fill takes 500 of the 1,000.5 USDC offered; the rest goes back up.
		const resized = await service.settleFill(order.id, 500n * ONE)
		expect(resized?.status).toBe("open")
		expect(resized?.commitment).not.toBe(order.commitment)
		const afterResize = await client.myOrders(signerAddress(service))
		expect(afterResize.orders.map((entry) => entry.commitment)).toEqual([resized?.commitment])

		expect(await service.reconcile()).toEqual({ cancelled: 0, reposted: 0, underFunded: 0 })

		const { result: cancelled } = await service.cancel(order.id)
		expect(cancelled.kind).toBe("cancelled")
		expect((await client.myOrders(signerAddress(service))).orders).toEqual([])
	}, 60_000)
})

/** The solver the service signs as, which is the address its orders are filed under. */
function signerAddress(service: LimitOrderService): HexString {
	// biome-ignore lint/suspicious/noExplicitAny: the signer is the service's own, not a second one
	return (service as any).signer.address
}
