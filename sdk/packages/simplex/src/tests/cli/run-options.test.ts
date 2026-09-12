import { describe, it, expect } from "vitest"
import { Command } from "commander"
import { addRunOptions, type RunOptions } from "@/cli/run-options"

/**
 * A `run`-shaped command with no action, so parsing stops once the flags are
 * read. The bin itself cannot be imported here: it calls
 * `program.parse(process.argv)` at module load.
 */
function parseRun(argv: string[]): RunOptions {
	const command = addRunOptions(new Command().exitOverride())
	command.parse(argv, { from: "user" })
	return command.opts<RunOptions>()
}

describe("simplex run flags", () => {
	/**
	 * Regression: the placeholder used to be `[<[host:]port>]`, and commander
	 * derives `required` from a `<` *anywhere* in the flags string. The option
	 * came out both required and optional, required won, and a valueless `--ui`
	 * died with "option '--ui [<[host:]port>]' argument missing".
	 */
	it("accepts --ui with no value", () => {
		const ui = addRunOptions(new Command()).options.find((o) => o.long === "--ui" && !o.negate)
		expect(ui?.required).toBe(false)
		expect(ui?.optional).toBe(true)
		expect(parseRun(["--ui"]).ui).toBe(true)
	})

	it("keeps the bind address, --no-ui and the default untouched", () => {
		expect(parseRun(["--ui", "9000"]).ui).toBe("9000")
		expect(parseRun(["--ui", "192.168.1.50:8686"]).ui).toBe("192.168.1.50:8686")
		expect(parseRun(["--ui", "0.0.0.0:8686"]).ui).toBe("0.0.0.0:8686")
		expect(parseRun(["--no-ui"]).ui).toBe(false)
		// Absent, not `true`: the handler reads `options.ui !== false`, so an
		// unpassed flag and a bare `--ui` both mean "UI on, default bind".
		expect(parseRun([]).ui).toBeUndefined()
	})

	it("takes a socket path from --ui-socket, independently of --ui", () => {
		expect(parseRun(["--ui-socket", "/run/user/1000/simplex.sock"]).uiSocket).toBe("/run/user/1000/simplex.sock")
		expect(parseRun([]).uiSocket).toBeUndefined()
		// The two are separate flags. The handler rejects the combinations that name
		// two listen addresses (`--ui <addr>` with a socket, or `--no-ui` with one);
		// parsing keeps both so it can tell them apart and say which was meant.
		expect(parseRun(["--ui", "9000", "--ui-socket", "/tmp/s.sock"])).toMatchObject({
			ui: "9000",
			uiSocket: "/tmp/s.sock",
		})
		expect(parseRun(["--no-ui", "--ui-socket", "/tmp/s.sock"])).toMatchObject({
			ui: false,
			uiSocket: "/tmp/s.sock",
		})
		// A bare --ui names no address, so it is not a competing target.
		expect(parseRun(["--ui", "--ui-socket", "/tmp/s.sock"])).toMatchObject({ ui: true, uiSocket: "/tmp/s.sock" })
	})

	it("carries the other run flags", () => {
		const options = parseRun(["-c", "filler-config.toml", "-d", "/data", "--watch-only"])
		expect(options).toMatchObject({ config: "filler-config.toml", dataDir: "/data", watchOnly: true })
		expect(parseRun([]).watchOnly).toBe(false)
	})

	/**
	 * `run` is the default command, so the invocation that reaches an operator is
	 * `simplex --ui`, with no subcommand named. That routes through the parent
	 * program, which is where an argument-missing error surfaced.
	 */
	it("reaches the default command's handler from a bare simplex --ui", () => {
		const seen: RunOptions[] = []
		const program = new Command().name("simplex").version("0.0.0").exitOverride()
		program.command("init").action(() => {})
		addRunOptions(program.command("run", { isDefault: true })).action((options: RunOptions) => {
			seen.push(options)
		})

		program.parse(["--ui"], { from: "user" })
		expect(seen).toEqual([expect.objectContaining({ ui: true })])
	})
})
