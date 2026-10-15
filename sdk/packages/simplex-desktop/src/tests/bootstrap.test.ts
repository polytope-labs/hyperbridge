import { execFileSync } from "node:child_process"
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs"
import { tmpdir } from "node:os"
import { basename, join } from "node:path"
import { expect, it } from "vitest"

it("maps main-process stacks without requiring a Node launch flag", () => {
	const directory = mkdtempSync(join(tmpdir(), "simplex-bootstrap-"))
	try {
		const { main } = JSON.parse(readFileSync(new URL("../../package.json", import.meta.url), "utf8"))
		const entry = basename(main)
		writeFileSync(join(directory, "package.json"), JSON.stringify({ type: "module" }))
		// The bootstrap contains only JavaScript, so exercise its exact source with Node.
		writeFileSync(join(directory, entry), readFileSync(new URL("../bootstrap.ts", import.meta.url)))
		writeFileSync(
			join(directory, "main.js"),
			"\n".repeat(100) + "console.log(new Error('mapped').stack)\n//# sourceMappingURL=main.js.map",
		)
		writeFileSync(
			join(directory, "main.js.map"),
			JSON.stringify({
				version: 3,
				file: "main.js",
				sources: ["original.ts"],
				sourcesContent: ["console.log(new Error('mapped').stack)"],
				names: [],
				mappings: ";".repeat(100) + "AAAA",
			}),
		)
		const stack = execFileSync(process.execPath, [join(directory, entry)], {
			encoding: "utf8",
			env: { ...process.env, NODE_OPTIONS: "" },
		})
		expect(stack).toContain("original.ts:1:")
		expect(stack).not.toContain("main.js:101:")
	} finally {
		rmSync(directory, { recursive: true, force: true })
	}
})
