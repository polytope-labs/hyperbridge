import { describe, expect, it } from "vitest"
import { LoggerContext } from "@/services/Logger"
import { LogBuffer, matchesLogQuery } from "@/services/server/LogBuffer"
import type { LogRecordDto } from "@/services/server/dto"

/**
 * The dashboard's Logs page reads this buffer, so what it keeps and what it
 * filters is the page's behaviour. It is fed by registering its sink on a
 * `LoggerContext` exactly as the console sink is, so the tests drive it through
 * a real logger rather than hand-written NDJSON wherever they can.
 */

function bufferAt(level: "trace" | "debug" | "info" | "warn" | "error", capacity?: number) {
	const buffer = new LogBuffer(capacity)
	const loggers = new LoggerContext({ level })
	loggers.addSink(buffer.sink())
	return { buffer, loggers }
}

describe("LogBuffer capture", () => {
	it("turns pino records into display rows, splitting the message from its fields", () => {
		const { buffer, loggers } = bufferAt("info")
		loggers.get("filler").info({ orderId: "0x7f2c", pair: "USDC/CNGN" }, "Order detected")

		const [record] = buffer.recent({})
		expect(record.level).toBe("info")
		// The brackets belong to the console format, not the module name.
		expect(record.module).toBe("filler")
		expect(record.msg).toBe("Order detected")
		expect(JSON.parse(record.detail!)).toEqual({ orderId: "0x7f2c", pair: "USDC/CNGN" })
		expect(record.time).toBeGreaterThan(0)
		expect(record.seq).toBe(1)
	})

	it("omits the module for records logged without one, and detail for records with no fields", () => {
		const { buffer, loggers } = bufferAt("info")
		loggers.get().info("Booting")

		expect(buffer.recent({})[0]).toMatchObject({ msg: "Booting" })
		expect(buffer.recent({})[0].module).toBeUndefined()
		expect(buffer.recent({})[0].detail).toBeUndefined()
	})

	it("keeps fatal, which pino emits but no operator can select as a capture level", () => {
		const { buffer, loggers } = bufferAt("trace")
		loggers.get("boot").fatal("Unrecoverable")

		expect(buffer.recent({})[0].level).toBe("fatal")
		// It reads as the most severe thing there is, so an error filter must show it.
		expect(buffer.recent({ level: "error" })).toHaveLength(1)
	})

	it("captures nothing below the context's level — the page cannot recover what was never written", () => {
		const { buffer, loggers } = bufferAt("info")
		loggers.get("filler").debug("Curve quote resolved")
		loggers.get("filler").info("Order detected")

		expect(buffer.recent({}).map((r) => r.msg)).toEqual(["Order detected"])
	})

	it("follows a level change on the context it is attached to", () => {
		const { buffer, loggers } = bufferAt("info")
		loggers.get("filler").debug("before")
		loggers.setLevel("debug")
		loggers.get("filler").debug("after")

		expect(buffer.recent({}).map((r) => r.msg)).toEqual(["after"])
	})

	it("ignores lines that are not pino records", () => {
		const buffer = new LogBuffer()
		const sink = buffer.sink()
		sink.write("not json\n")
		sink.write(`${JSON.stringify({ time: 1, msg: "no level" })}\n`)
		sink.write(`${JSON.stringify({ level: 30, time: 1, msg: "kept" })}\n`)

		expect(buffer.recent({}).map((r) => r.msg)).toEqual(["kept"])
		// A dropped line must not consume a seq, or a resuming stream skips a record.
		expect(buffer.recent({})[0].seq).toBe(1)
	})

	it("splits a write that carries several lines", () => {
		const buffer = new LogBuffer()
		buffer.sink().write(
			`${JSON.stringify({ level: 30, time: 1, msg: "one" })}\n${JSON.stringify({ level: 40, time: 2, msg: "two" })}\n`,
		)

		expect(buffer.recent({}).map((r) => r.msg)).toEqual(["one", "two"])
	})

	it("caps a runaway detail rather than holding it in memory", () => {
		const buffer = new LogBuffer()
		buffer.sink().write(`${JSON.stringify({ level: 30, time: 1, msg: "big", blob: "x".repeat(10_000) })}\n`)

		const { detail } = buffer.recent({})[0]
		expect(detail!.length).toBeLessThan(4100)
		expect(detail!.endsWith("…")).toBe(true)
	})

	it("evicts the oldest records once it is full, and keeps counting seq", () => {
		const { buffer, loggers } = bufferAt("info", 3)
		for (let i = 1; i <= 5; i++) loggers.get("filler").info(`line ${i}`)

		expect(buffer.recent({}).map((r) => r.msg)).toEqual(["line 3", "line 4", "line 5"])
		expect(buffer.recent({}).map((r) => r.seq)).toEqual([3, 4, 5])
	})
})

describe("LogBuffer queries", () => {
	function populated() {
		const { buffer, loggers } = bufferAt("trace")
		loggers.get("scanner").info({ chain: "EVM-8453" }, "Scanned block")
		loggers.get("filler").debug({ notional: "1500" }, "Curve quote resolved")
		loggers.get("quorum").warn({ endpoint: "https://rpc.example" }, "Provider fell behind")
		loggers.get("filler").error({ reason: "insufficient allowance" }, "Fill reverted")
		return buffer
	}

	it("level means that level and above", () => {
		expect(populated()
			.recent({ level: "warn" })
			.map((r) => r.msg)).toEqual(["Provider fell behind", "Fill reverted"])
		expect(populated().recent({ level: "trace" })).toHaveLength(4)
	})

	it("search matches the message, the module and the serialized fields", () => {
		const buffer = populated()
		expect(buffer.recent({ q: "reverted" }).map((r) => r.msg)).toEqual(["Fill reverted"])
		expect(buffer.recent({ q: "quorum" }).map((r) => r.msg)).toEqual(["Provider fell behind"])
		expect(buffer.recent({ q: "rpc.example" }).map((r) => r.msg)).toEqual(["Provider fell behind"])
	})

	it("search is case-insensitive", () => {
		expect(populated().recent({ q: "SCANNED" })).toHaveLength(1)
	})

	it("combines level and search", () => {
		expect(populated().recent({ level: "error", q: "quorum" })).toHaveLength(0)
		expect(populated().recent({ level: "warn", q: "quorum" })).toHaveLength(1)
	})

	it("after resumes past a seq the caller already holds", () => {
		const buffer = populated()
		expect(buffer.recent({ after: 2 }).map((r) => r.seq)).toEqual([3, 4])
	})

	it("limit keeps the most recent matches, not the first", () => {
		expect(populated()
			.recent({ limit: 2 })
			.map((r) => r.msg)).toEqual(["Provider fell behind", "Fill reverted"])
	})
})

describe("LogBuffer subscriptions", () => {
	it("delivers records captured after subscribing, until unsubscribed", () => {
		const { buffer, loggers } = bufferAt("info")
		loggers.get("filler").info("before")

		const seen: LogRecordDto[] = []
		const unsubscribe = buffer.subscribe((record) => seen.push(record))
		loggers.get("filler").info("during")
		unsubscribe()
		loggers.get("filler").info("after")

		expect(seen.map((r) => r.msg)).toEqual(["during"])
	})

	it("a throwing subscriber does not stop the others, or the capture", () => {
		const { buffer, loggers } = bufferAt("info")
		const seen: string[] = []
		buffer.subscribe(() => {
			throw new Error("dead tab")
		})
		buffer.subscribe((record) => seen.push(record.msg))

		expect(() => loggers.get("filler").info("still logged")).not.toThrow()
		expect(seen).toEqual(["still logged"])
		expect(buffer.recent({})).toHaveLength(1)
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
