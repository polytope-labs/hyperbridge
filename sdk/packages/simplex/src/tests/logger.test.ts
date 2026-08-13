import { afterEach, describe, expect, it } from "vitest"
import { addLogSink, configureLogger, getLogger, type LogSink } from "@/services/Logger"

/**
 * A library must not write to its host's stdout uninvited, so logging is silent
 * until a sink is registered. These tests pin that, plus the two behaviours the
 * previous module-global logger got wrong: level changes never reached loggers
 * that had already been handed out, and there was no way to redirect output at
 * all.
 */

function collector(): LogSink & { lines: string[]; records: Record<string, unknown>[] } {
	const lines: string[] = []
	return {
		lines,
		get records() {
			return lines.map((line) => JSON.parse(line))
		},
		write(line: string) {
			lines.push(line)
		},
	}
}

const detachers: Array<() => void> = []

function attach(sink: LogSink): void {
	detachers.push(addLogSink(sink))
}

afterEach(() => {
	for (const detach of detachers.splice(0)) detach()
	configureLogger("info")
})

describe("logging", () => {
	it("writes nothing until a sink is registered", () => {
		const sink = collector()
		getLogger("silent-module").info("before any sink")
		expect(sink.lines).toHaveLength(0)

		attach(sink)
		getLogger("silent-module").info("after the sink")
		expect(sink.records.map((r) => r.msg)).toEqual(["after the sink"])
	})

	it("stops writing once the sink detaches", () => {
		const sink = collector()
		const detach = addLogSink(sink)
		getLogger("x").info("captured")
		detach()
		getLogger("x").info("dropped")
		expect(sink.records.map((r) => r.msg)).toEqual(["captured"])
	})

	it("fans out to every registered sink", () => {
		const a = collector()
		const b = collector()
		attach(a)
		attach(b)
		getLogger("x").info("both")
		expect(a.lines).toHaveLength(1)
		expect(b.lines).toHaveLength(1)
	})

	it("applies a level change to loggers handed out beforehand", () => {
		const sink = collector()
		attach(sink)
		// Captured before the level moves — the previous implementation replaced the
		// base logger, leaving this one at the old level forever.
		const logger = getLogger("early")

		logger.debug("hidden at info")
		expect(sink.lines).toHaveLength(0)

		configureLogger("debug")
		logger.debug("visible at debug")
		expect(sink.records.map((r) => r.msg)).toEqual(["visible at debug"])
	})

	it("tags records with their module", () => {
		const sink = collector()
		attach(sink)
		getLogger("intent-filler").warn({ orderId: "0xabc" }, "skipped")

		const [record] = sink.records
		expect(record.moduleTag).toBe("[intent-filler]")
		expect(record.orderId).toBe("0xabc")
		expect(record.msg).toBe("skipped")
	})

	it("survives a sink that throws", () => {
		const good = collector()
		attach({
			write() {
				throw new Error("downstream is on fire")
			},
		})
		attach(good)
		// A broken sink is the host's problem; it must not propagate into a fill.
		expect(() => getLogger("x").info("still delivered")).not.toThrow()
		expect(good.lines).toHaveLength(1)
	})
})
