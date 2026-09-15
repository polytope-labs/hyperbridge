import { join } from "node:path"
import type { SolverStatus } from "./solver-supervisor"

export const TRAY_ICON_STATES = [
	"starting",
	"setup",
	"running",
	"paused",
	"stopping",
	"stopped",
	"unreachable",
] as const

/** Selects a rasterized PWA-logo variant that Electron can use as a native tray image. */
export function trayIconPath(
	directory: string,
	status: Pick<SolverStatus, "state">,
	platform: NodeJS.Platform = process.platform,
): string {
	const suffix = platform === "darwin" ? "Template" : ""
	return join(directory, `${status.state}${suffix}.png`)
}

export function trayIconRetinaPath(directory: string, status: Pick<SolverStatus, "state">): string {
	return join(directory, `${status.state}Template@2x.png`)
}
