import { describe, it, expect } from "vitest"
import { Command } from "commander"
import { DEFAULT_LOG_FORMAT, logFormatFromArgv } from "@/cli/log-format"
import { addRunOptions, type RunOptions } from "@/cli/run-options"

describe("logFormatFromArgv", () => {
	it("defaults to pretty, which is what a terminal has always got", () => {
		expect(logFormatFromArgv(["node", "simplex"])).toBe("pretty")
		expect(logFormatFromArgv(["node", "simplex", "run", "--no-ui"])).toBe("pretty")
		expect(DEFAULT_LOG_FORMAT).toBe("pretty")
	})

	it("reads both spellings, before or after the subcommand", () => {
		expect(logFormatFromArgv(["node", "simplex", "--log-format", "json"])).toBe("json")
		expect(logFormatFromArgv(["node", "simplex", "--log-format=json"])).toBe("json")
		expect(logFormatFromArgv(["node", "simplex", "run", "--log-format", "json"])).toBe("json")
		expect(logFormatFromArgv(["node", "simplex", "run", "--log-format=json", "--no-ui"])).toBe("json")
	})

	it("falls back to the default on an unknown value, leaving the complaint to commander", () => {
		expect(logFormatFromArgv(["node", "simplex", "--log-format", "ndjson"])).toBe("pretty")
		expect(logFormatFromArgv(["node", "simplex", "--log-format"])).toBe("pretty")
	})

	it("takes the last of a repeated flag, as commander does", () => {
		expect(logFormatFromArgv(["node", "simplex", "--log-format", "json", "--log-format", "pretty"])).toBe("pretty")
		expect(logFormatFromArgv(["node", "simplex", "--log-format", "pretty", "--log-format", "json"])).toBe("json")
	})

	it("stops at --, where commander stops parsing options", () => {
		expect(logFormatFromArgv(["node", "simplex", "run", "--", "--log-format", "json"])).toBe("pretty")
	})
})

/**
 * Parses `args` with the real `run` declarations — `addRunOptions` is the same
 * builder `bin/simplex.ts` calls, so this cannot drift from the shipped CLI the
 * way a hand-copied mirror would.
 */
function runOptionsFor(args: string[]): (RunOptions & { logFormat?: string }) | undefined {
	let seen: (RunOptions & { logFormat?: string }) | undefined
	const program = new Command()
	program
		.name("simplex")
		.exitOverride()
		.configureOutput({ writeErr: () => {} })
	program.command("init").action(() => {})
	addRunOptions(program.command("run", { isDefault: true })).action((opts) => {
		seen = opts
	})
	program.parse(["node", "simplex", ...args])
	return seen
}

/**
 * The flag is read twice: once from raw argv, because the console sink is built
 * before commander runs, and once by commander itself. Nothing forces the two to
 * agree, so pin it — a disagreement puts pretty-printed output in a stream a
 * supervisor is parsing as NDJSON.
 */
function commanderLogFormat(args: string[]): string | undefined {
	return runOptionsFor(args)?.logFormat
}

describe("the argv scan and commander agree", () => {
	const invocations = [
		[],
		["run"],
		["--log-format", "json"],
		["--log-format=json"],
		["run", "--log-format", "json"],
		["run", "--log-format=pretty"],
		["--log-format", "json", "--log-format", "pretty"],
		["--no-ui", "--no-open", "--log-format", "json"],
		["run", "-c", "filler-config.toml", "--log-format", "json"],
		// `--ui` takes an optional value, so it is the declaration most able to
		// swallow a following flag. It did, until #1249 dropped the angle brackets.
		["run", "--ui", "--log-format", "json"],
		["run", "--ui", "9000", "--log-format", "json"],
		["run", "--watch-only", "--log-format", "json"],
	]

	it.each(invocations)("simplex %s", (...args: string[]) => {
		expect(commanderLogFormat(args)).toBe(logFormatFromArgv(["node", "simplex", ...args]))
	})

	it("rejects an unknown format rather than guessing, so nothing is logged in the wrong one", () => {
		expect(() => commanderLogFormat(["--log-format", "ndjson"])).toThrow(/Allowed choices are pretty, json/)
	})
})

describe("--no-open", () => {
	it("is absent unless passed, so a plain run still opens a browser", () => {
		expect(runOptionsFor(["run"])?.open).toBe(true)
		expect(runOptionsFor([])?.open).toBe(true)
	})

	it("turns off only the browser launch", () => {
		expect(runOptionsFor(["run", "--no-open"])?.open).toBe(false)
		expect(runOptionsFor(["--no-open"])?.open).toBe(false)
		// --no-ui is a different switch and must not be implied either way.
		expect(runOptionsFor(["run", "--no-open"])?.ui).toBeUndefined()
	})

	it("survives a bare --ui in front of it", () => {
		// `--ui [<[host:]port>]` used to be required *and* optional, so commander
		// shifted the next token into it and `--no-open` was silently dropped (#1249).
		const opts = runOptionsFor(["run", "--ui", "--no-open"])
		expect(opts?.open).toBe(false)
		expect(opts?.ui).toBe(true)
	})

	it("is unaffected by a --ui that does take a value", () => {
		expect(runOptionsFor(["run", "--ui", "9000", "--no-open"])?.open).toBe(false)
		expect(runOptionsFor(["run", "--no-open", "--ui", "9000"])?.ui).toBe("9000")
	})
})
