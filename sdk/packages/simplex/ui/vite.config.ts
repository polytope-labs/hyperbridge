import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"
import { resolve } from "path"

export default defineConfig({
	root: __dirname,
	base: "./",
	plugins: [react()],
	build: {
		outDir: resolve(__dirname, "../dist/ui"),
		emptyOutDir: true,
	},
	server: {
		port: 5173,
		proxy: {
			"/api": "http://127.0.0.1:8686",
			"/health": "http://127.0.0.1:8686",
		},
	},
})
