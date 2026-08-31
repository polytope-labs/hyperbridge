import { spawn } from "child_process"
import { getLogger } from "@/services/Logger"

/** Best-effort browser launch; on failure the printed URL is the fallback. */
export function openBrowser(url: string): void {
	const [cmd, args] =
		process.platform === "darwin"
			? ["open", [url]]
			: process.platform === "win32"
				? ["cmd", ["/c", "start", "", url]]
				: ["xdg-open", [url]]

	try {
		const child = spawn(cmd, args as string[], { detached: true, stdio: "ignore" })
		child.on("error", () => {
			getLogger("cli").info(`Could not open a browser — open ${url} manually`)
		})
		child.unref()
	} catch {
		getLogger("cli").info(`Could not open a browser — open ${url} manually`)
	}
}
