import { defineConfig } from "tsup"

export default defineConfig({
	entry: ["src/index.ts", "src/bin/filler.ts"],
	format: ["esm", "cjs"],
	dts: true,
	splitting: false,
	sourcemap: true,
	clean: true,
	shims: true,
	onSuccess: async () => {
		// Add shebang only to the CLI file after build
		const fs = await import("fs/promises")
		const cliPath = "./dist/bin/filler.js"
		const cliCjsPath = "./dist/bin/filler.cjs"

		try {
			// Add shebang to ESM version
			const content = await fs.readFile(cliPath, "utf-8")
			if (!content.startsWith("#!/usr/bin/env node")) {
				await fs.writeFile(cliPath, `#!/usr/bin/env node\n${content}`)
			}

			// Add shebang to CJS version
			const cjsContent = await fs.readFile(cliCjsPath, "utf-8")
			if (!cjsContent.startsWith("#!/usr/bin/env node")) {
				await fs.writeFile(cliCjsPath, `#!/usr/bin/env node\n${cjsContent}`)
			}
		} catch (error) {
			console.error("Failed to add shebang to CLI files:", error)
		}
	},
})
