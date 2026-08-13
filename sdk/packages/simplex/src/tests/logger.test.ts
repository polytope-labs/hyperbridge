import { afterEach, describe, expect, it } from "vitest"
import { addLogSink, configureLogger, getLogger, LoggerContext, type LogSink } from "@/services/Logger"

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

describe("LoggerContext isolation", () => {
	it("keeps two contexts' sinks apart", () => {
		const a = collector()
		const b = collector()
		const ctxA = new LoggerContext({ sink: a })
		const ctxB = new LoggerContext({ sink: b })

		ctxA.get("filler").info("from A")
		ctxB.get("filler").info("from B")

		// The whole point: one filler's records never reach another's sink.
		expect(a.records.map((r) => r.msg)).toEqual(["from A"])
		expect(b.records.map((r) => r.msg)).toEqual(["from B"])
	})

	it("keeps levels independent", () => {
		const a = collector()
		const b = collector()
		const ctxA = new LoggerContext({ sink: a })
		const ctxB = new LoggerContext({ sink: b })

		ctxA.setLevel("debug")
		ctxA.get("x").debug("A debug")
		ctxB.get("x").debug("B debug")

		expect(a.records.map((r) => r.msg)).toEqual(["A debug"])
		expect(b.lines).toHaveLength(0)
	})

	it("does not write to the process-wide context", () => {
		const instance = collector()
		const process = collector()
		attach(process)

		new LoggerContext({ sink: instance }).get("filler").info("instance only")

		expect(instance.lines).toHaveLength(1)
		expect(process.lines).toHaveLength(0)
	})

	it("takes an initial level and accepts more sinks later", () => {
		const first = collector()
		const second = collector()
		const context = new LoggerContext({ level: "debug", sink: first })

		const detach = context.addSink(second)
		context.get("x").debug("both")
		detach()
		context.get("x").debug("first only")

		expect(first.records.map((r) => r.msg)).toEqual(["both", "first only"])
		expect(second.records.map((r) => r.msg)).toEqual(["both"])
	})
})

describe("service wiring", () => {
	it("routes services through their filler's context, not the process one", async () => {
		const { FillerConfigService } = await import("@/services/FillerConfigService")
		const { ChainClientManager } = await import("@/services/ChainClientManager")
		const { CacheService } = await import("@/services/CacheService")

		const instance = collector()
		const process = collector()
		attach(process)

		const loggers = new LoggerContext({ level: "debug", sink: instance })
		const configService = new FillerConfigService(
			[{ chainId: 1, rpcUrls: ["https://rpc.example"] }],
			undefined,
			loggers,
		)

		// The context reaches services through the config service, and through the
		// client manager for everything constructed from one.
		expect(configService.loggers).toBe(loggers)
		expect(new ChainClientManager(configService).loggers).toBe(loggers)

		new CacheService(loggers)

		// Whatever any of them logged went to the filler's sink; the process-wide
		// context — which a host would never have configured — stayed empty.
		expect(process.lines).toHaveLength(0)
	})

	it("gives two config services independent contexts", async () => {
		const { FillerConfigService } = await import("@/services/FillerConfigService")
		const a = new LoggerContext({ sink: collector() })
		const b = new LoggerContext({ sink: collector() })
		const chains = [{ chainId: 1, rpcUrls: ["https://rpc.example"] }]

		expect(new FillerConfigService(chains, undefined, a).loggers).not.toBe(
			new FillerConfigService(chains, undefined, b).loggers,
		)
	})
})
