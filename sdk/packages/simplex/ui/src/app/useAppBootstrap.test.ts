import { describe, expect, it } from "vitest"
import type { Status } from "../types"
import { resolveBootstrapState } from "./useAppBootstrap"

describe("desktop UI version interlock", () => {
	it("blocks a newer desktop renderer from driving an older setup API", () => {
		expect(
			resolveBootstrapState({ mode: "init", starting: false, version: "0.16.2" }, undefined, undefined, "0.17.0"),
		).toEqual({ kind: "version-skew", desktopVersion: "0.17.0", solverVersion: "0.16.2" })
		expect(
			resolveBootstrapState(
				// Runtime compatibility: pre-update setup responses had no version field.
				{ mode: "init", starting: false } as unknown as Status,
				undefined,
				undefined,
				"0.17.0",
			),
		).toEqual({ kind: "version-skew", desktopVersion: "0.17.0", solverVersion: "unknown" })
	})
})
