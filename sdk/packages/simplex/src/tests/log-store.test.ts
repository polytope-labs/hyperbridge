import { existsSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { describe, expect, it } from "vitest"
import { LoggerContext } from "@/services/Logger"
import { LogStore, matchesLogQuery } from "@/services/server/LogStore"
import type { LogRecordDto } from "@/services/server/dto"

/**
 * The dashboard's Logs page reads this store, so what it keeps, what it writes
 * to disk and what it filters is the page's behaviour. It is fed by registering
 * its sink on a `LoggerContext` exactly as the console sink is, so the tests
 * drive it through a real logger wherever they can.
 */

type Level = "trace" | "debug" | "info" | "warn" | "error"

function storeAt(level: Level, options: { capacity?: number; dir?: string } = {}) {
	const store = new LogStore({ capacity: options.capacity, dir: options.dir })
	const loggers = new LoggerContext({ level })
	loggers.addSink(store.sink())
	return { store, loggers, log: loggers.get("filler") }
}

function tempDir(): string {
	return mkdtempSync(join(tmpdir(), "simplex-logs-"))
}

/** Lets the write stream flush before a test reads the launch file back. */
function flushed(): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, 30))
}

describe("LogStore capture", () => {
	it("turns pino records into display rows, splitting the message from its fields", async () => {
		const { store, log } = storeAt("info")
		log.info({ orderId: "0x7f2c", pair: "USDC/CNGN" }, "Order detected")

		const [record] = await store.recent({})
		expect(record.level).toBe("info")
		expect(record.module).toBe("filler")
		expect(record.msg).toBe("Order detected")
		expect(JSON.parse(record.detail!)).toEqual({ orderId: "0x7f2c", pair: "USDC/CNGN" })
		expect(record.seq).toBe(1)
	})

	/**
	 * pino writes its own numeric level first and the caller's merge object after,
	 * so a payload field named `level` produces two `level` keys and JSON.parse
	 * keeps the caller's. Reading the numeric one off the line prefix is what
	 * stops such a record from vanishing — including the UI server's own
	 * `warn({ level }, "Log level changed from the UI")`.
	 */
	it("keeps a record whose payload carries its own `level` field, and shows that value", async () => {
		const { store, loggers } = storeAt("info")
		loggers.get("ui").warn({ level: "debug" }, "Log level changed from the UI")

		const [record] = await store.recent({})
		expect(record).toBeDefined()
		expect(record.msg).toBe("Log level changed from the UI")
		expect(record.level).toBe("warn")
		expect(JSON.parse(record.detail!)).toEqual({ level: "debug" })
	})

	it("keeps a record whose payload carries any other envelope-shaped field", async () => {
		const { store, log } = storeAt("info")
		log.info({ time: "yesterday" }, "Real message")

		const [record] = await store.recent({})
		expect(record.msg).toBe("Real message")
		expect(record.level).toBe("info")
	})

	it("omits the module for records logged without one, and detail for records with no fields", async () => {
		const { store, loggers } = storeAt("info")
		loggers.get().info("Booting")

		const [record] = await store.recent({})
		expect(record.msg).toBe("Booting")
		expect(record.module).toBeUndefined()
		expect(record.detail).toBeUndefined()
	})

	it("keeps fatal, which pino emits but no operator can select as a level", async () => {
		const { store, loggers } = storeAt("trace")
		loggers.get("boot").fatal("Unrecoverable")

		expect((await store.recent({}))[0].level).toBe("fatal")
		expect(await store.recent({ level: "error" })).toHaveLength(1)
	})

	it("captures nothing below the context's level — the page cannot recover what was never written", async () => {
		const { store, log } = storeAt("info")
		log.debug("Curve quote resolved")
		log.info("Order detected")

		expect((await store.recent({})).map((r) => r.msg)).toEqual(["Order detected"])
	})

	it("follows the context's level in both directions", async () => {
		const { store, loggers, log } = storeAt("info")
		log.debug("before raise")
		loggers.setLevel("debug")
		log.debug("after raise")
		loggers.setLevel("error")
		log.warn("after lower")
		log.error("still recorded")

		expect((await store.recent({})).map((r) => r.msg)).toEqual(["after raise", "still recorded"])
	})

	it("ignores lines that are not pino records, without consuming a seq", async () => {
		const store = new LogStore()
		const sink = store.sink()
		sink.write("not json\n")
		sink.write(`${JSON.stringify({ time: 1, msg: "no level" })}\n`)
		sink.write(`${JSON.stringify([1, 2, 3])}\n`)
		sink.write(`${JSON.stringify({ level: 30, time: 1, msg: "kept" })}\n`)

		const records = await store.recent({})
		expect(records.map((r) => r.msg)).toEqual(["kept"])
		expect(records[0].seq).toBe(1)
		expect(store.stats().captured).toBe(1)
	})

	it("splits a write that carries several lines", async () => {
		const store = new LogStore()
		store
			.sink()
			.write(
				`${JSON.stringify({ level: 30, time: 1, msg: "one" })}\n${JSON.stringify({ level: 40, time: 2, msg: "two" })}\n`,
			)

		expect((await store.recent({})).map((r) => r.msg)).toEqual(["one", "two"])
	})

	it("caps a runaway detail and stores it as a flat copy, not a view onto the original", async () => {
		const store = new LogStore()
		store.sink().write(`${JSON.stringify({ level: 30, time: 1, msg: "big", blob: "x".repeat(50_000) })}\n`)

		const { detail } = (await store.recent({}))[0]
		expect(detail!.length).toBeLessThan(4100)
		expect(detail!.endsWith("…")).toBe(true)
		expect(Buffer.byteLength(detail!, "utf-8")).toBeLessThan(4200)
	})

	it("evicts the oldest records once memory is full, and keeps counting seq", async () => {
		const { store, log } = storeAt("info", { capacity: 3 })
		for (let i = 1; i <= 5; i++) log.info(`line ${i}`)

		const records = await store.recent({})
		expect(records.map((r) => r.msg)).toEqual(["line 3", "line 4", "line 5"])
		expect(records.map((r) => r.seq)).toEqual([3, 4, 5])
		expect(store.stats().captured).toBe(5)
	})
})

describe("LogStore queries", () => {
	function populated() {
		const { store, loggers } = storeAt("trace")
		loggers.get("scanner").info({ chain: "EVM-8453" }, "Scanned block")
		loggers.get("filler").debug({ notional: "1500" }, "Curve quote resolved")
		loggers.get("quorum").warn({ endpoint: "https://rpc.example" }, "Provider fell behind")
		loggers.get("filler").error({ reason: "insufficient allowance" }, "Fill reverted")
		return store
	}

	it("level means that level and above", async () => {
		const store = populated()
		expect((await store.recent({ level: "warn" })).map((r) => r.msg)).toEqual([
			"Provider fell behind",
			"Fill reverted",
		])
		expect(await store.recent({ level: "trace" })).toHaveLength(4)
	})

	it("search matches the message, the module and the serialized fields", async () => {
		const store = populated()
		expect((await store.recent({ q: "reverted" })).map((r) => r.msg)).toEqual(["Fill reverted"])
		expect((await store.recent({ q: "quorum" })).map((r) => r.msg)).toEqual(["Provider fell behind"])
		expect((await store.recent({ q: "rpc.example" })).map((r) => r.msg)).toEqual(["Provider fell behind"])
	})

	it("search is case-insensitive", async () => {
		expect(await populated().recent({ q: "SCANNED" })).toHaveLength(1)
	})

	it("combines level and search", async () => {
		const store = populated()
		expect(await store.recent({ level: "error", q: "quorum" })).toHaveLength(0)
		expect(await store.recent({ level: "warn", q: "quorum" })).toHaveLength(1)
	})

	it("after resumes past a seq the caller already holds", async () => {
		expect((await populated().recent({ after: 2 })).map((r) => r.seq)).toEqual([3, 4])
	})

	/** seq restarts at 1 each launch, so a mark from a dead process must not empty the page. */
	it("ignores an after beyond anything captured this launch", async () => {
		expect(await populated().recent({ after: 99_999 })).toHaveLength(4)
	})

	it("limit keeps the most recent matches, not the first", async () => {
		expect((await populated().recent({ limit: 2 })).map((r) => r.msg)).toEqual([
			"Provider fell behind",
			"Fill reverted",
		])
	})
})

describe("LogStore persistence", () => {
	it("writes every record to this launch's file and reports where", async () => {
		const dir = tempDir()
		const { store, log } = storeAt("info", { dir })
		log.info({ chain: "EVM-8453" }, "Scanned block")
		log.error("Fill reverted")
		await flushed()

		const stats = store.stats()
		expect(stats.persisted).toBe(true)
		expect(stats.captured).toBe(2)
		expect(existsSync(stats.path!)).toBe(true)
		const lines = readFileSync(stats.path!, "utf-8")
			.trim()
			.split("\n")
			.map((l) => JSON.parse(l) as LogRecordDto)
		expect(lines.map((r) => r.msg)).toEqual(["Scanned block", "Fill reverted"])
		store.close()
	})

	/** The whole point of the file: history outlives the in-memory tail. */
	it("answers from the file once records have been evicted from memory", async () => {
		const dir = tempDir()
		const { store, log } = storeAt("info", { dir, capacity: 3 })
		for (let i = 1; i <= 10; i++) log.info(`line ${i}`)
		await flushed()

		const all = await store.recent({})
		expect(all.map((r) => r.msg)).toEqual(Array.from({ length: 10 }, (_, i) => `line ${i + 1}`))
		expect(all.map((r) => r.seq)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
		store.close()
	})

	it("filters and limits history read back from the file", async () => {
		const dir = tempDir()
		const { store, log } = storeAt("trace", { dir, capacity: 2 })
		log.info("Scanned block")
		log.error({ reason: "allowance" }, "Fill reverted")
		for (let i = 0; i < 5; i++) log.info(`filler noise ${i}`)
		await flushed()

		expect((await store.recent({ level: "error" })).map((r) => r.msg)).toEqual(["Fill reverted"])
		expect((await store.recent({ q: "allowance" })).map((r) => r.msg)).toEqual(["Fill reverted"])
		expect(await store.recent({ limit: 3 })).toHaveLength(3)
		store.close()
	})

	/** Records captured before --data-dir was parsed still belong to this launch. */
	it("backfills what it already held when the launch file is opened later", async () => {
		const dir = tempDir()
		const { store, log } = storeAt("info")
		log.info("logged before the file existed")
		store.openLaunchFile(dir)
		log.info("logged after")
		await flushed()

		const written = readFileSync(store.stats().path!, "utf-8")
			.trim()
			.split("\n")
			.map((l) => JSON.parse(l).msg)
		expect(written).toEqual(["logged before the file existed", "logged after"])
		store.close()
	})

	it("prunes older launches, keeping the most recent ones", async () => {
		const dir = tempDir()
		for (const stamp of ["2026-09-01T00-00-00", "2026-09-02T00-00-00", "2026-09-03T00-00-00"]) {
			writeFileSync(join(dir, `simplex-${stamp}.log`), "")
		}
		writeFileSync(join(dir, "unrelated.txt"), "keep me")

		const store = new LogStore({ dir, launchesKept: 2, startedAt: new Date("2026-09-04T00:00:00Z") })
		await flushed()
		const names = readdirSync(dir).sort()
		expect(names.filter((n) => n.startsWith("simplex-"))).toEqual([
			"simplex-2026-09-03T00-00-00.log",
			"simplex-2026-09-04T00-00-00-000.log",
		])
		expect(names).toContain("unrelated.txt")
		store.close()
	})

	/**
	 * `RestartSec=0` puts two launches inside one second. With a second-resolution
	 * stamp and an appending stream they shared a file, so the dead launch's
	 * seq-1..N run was read back as this launch's history and collided with the
	 * ring on seq.
	 */
	it("gives each launch its own file when restarts land in the same second", async () => {
		const dir = tempDir()
		const first = new LogStore({ dir, startedAt: new Date("2026-09-10T08:49:27.100Z") })
		const firstLoggers = new LoggerContext({ level: "info" })
		firstLoggers.addSink(first.sink())
		firstLoggers.get("filler").info("from the dead launch")
		await flushed()
		first.close()

		const second = new LogStore({ dir, capacity: 2, startedAt: new Date("2026-09-10T08:49:27.900Z") })
		const secondLoggers = new LoggerContext({ level: "info" })
		secondLoggers.addSink(second.sink())
		for (let i = 1; i <= 5; i++) secondLoggers.get("filler").info(`from this launch ${i}`)
		await flushed()

		expect(second.stats().path).not.toBe(first.stats().path)
		const history = await second.recent({})
		expect(history.map((r) => r.msg)).toEqual([
			"from this launch 1",
			"from this launch 2",
			"from this launch 3",
			"from this launch 4",
			"from this launch 5",
		])
		// seq is per launch, so a shared file would have produced duplicates here.
		expect(new Set(history.map((r) => r.seq)).size).toBe(history.length)
		second.close()
	})

	it("falls back to memory when the directory cannot be used", async () => {
		const dir = tempDir()
		const blocker = join(dir, "logs")
		writeFileSync(blocker, "not a directory")
		const { store, log } = storeAt("info", { dir: blocker })
		log.info("still captured")

		expect(store.stats().persisted).toBe(false)
		expect((await store.recent({})).map((r) => r.msg)).toEqual(["still captured"])
	})
})

describe("LogStore subscriptions", () => {
	it("delivers records captured after subscribing, until unsubscribed", () => {
		const { store, log } = storeAt("info")
		log.info("before")

		const seen: LogRecordDto[] = []
		const unsubscribe = store.subscribe((record) => seen.push(record))
		log.info("during")
		unsubscribe()
		log.info("after")

		expect(seen.map((r) => r.msg)).toEqual(["during"])
	})

	it("a throwing subscriber does not stop the others, or the capture", async () => {
		const { store, log } = storeAt("info")
		const seen: string[] = []
		store.subscribe(() => {
			throw new Error("dead tab")
		})
		store.subscribe((record) => seen.push(record.msg))

		expect(() => log.info("still logged")).not.toThrow()
		expect(seen).toEqual(["still logged"])
		expect(await store.recent({})).toHaveLength(1)
	})
})

describe("matchesLogQuery", () => {
	const record: LogRecordDto = {
		seq: 5,
		time: 1,
		level: "warn",
		module: "quorum",
		msg: "Provider fell behind",
		detail: '{"endpoint":"https://rpc.example"}',
	}

	it("an empty query matches everything", () => {
		expect(matchesLogQuery(record, {})).toBe(true)
	})

	it("rejects a record below the requested level", () => {
		expect(matchesLogQuery(record, { level: "error" })).toBe(false)
		expect(matchesLogQuery(record, { level: "warn" })).toBe(true)
	})

	it("rejects a record at or before `after`", () => {
		expect(matchesLogQuery(record, { after: 5 })).toBe(false)
		expect(matchesLogQuery(record, { after: 4 })).toBe(true)
	})
})
