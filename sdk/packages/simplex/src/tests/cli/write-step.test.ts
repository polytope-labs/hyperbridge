import { mkdtempSync, existsSync, readFileSync, rmSync } from "fs"
import { tmpdir } from "os"
import { join } from "path"
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest"

const prompts = vi.hoisted(() => ({
	confirm: vi.fn(async () => false),
	note: vi.fn(),
	outro: vi.fn(),
	cancel: vi.fn(),
	isCancel: vi.fn(() => false),
	log: { info: vi.fn(), warn: vi.fn(), success: vi.fn(), error: vi.fn(), message: vi.fn() },
}))

vi.mock("@clack/prompts", () => prompts)

import { stepWrite } from "@/cli/init/steps/write"
import { newWizardState } from "@/cli/init/state"
import { INIT_CHAINS } from "@/cli/init/chains"
import type { FillerConfigFile } from "@/config/filler-toml"
import { SignerType } from "@/services/wallet"

const existing: FillerConfigFile = {
	orderbook: { url: "https://orderbook.example/graphql" },
	simplex: {
		signer: {
			type: SignerType.PrivateKey,
			key: "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
		},
		maxConcurrentOrders: 7,
		substratePrivateKey: "seed",
		hyperbridgeWsUrl: "wss://nexus.rpc.polytope.technology",
	},
	chains: [{ rpcUrls: ["https://eth.example/rpc"] }],
}

function updateRunState(config: FillerConfigFile) {
	const state = newWizardState()
	state.prefillConfig = JSON.parse(JSON.stringify(config))
	state.chains = [{ meta: INIT_CHAINS.find((c) => c.chainId === 1)!, rpcUrls: ["https://eth.example/rpc"] }]
	state.signer = config.simplex.signer
	state.substratePrivateKey = config.simplex.substratePrivateKey
	state.hyperbridgeWsUrl = config.simplex.hyperbridgeWsUrl
	state.maxConcurrentOrders = config.simplex.maxConcurrentOrders ?? state.maxConcurrentOrders
	return state
}

const DROPPED = /Dropped \[simplex\.overfillProtection\]/

describe("simplex init: the removed [simplex.overfillProtection] section", () => {
	let dir: string
	beforeEach(() => {
		dir = mkdtempSync(join(tmpdir(), "simplex-write-step-"))
		vi.clearAllMocks()
	})
	afterEach(() => rmSync(dir, { recursive: true, force: true }))

	it("says it dropped the section only once the new file is written without it", async () => {
		const path = join(dir, "filler-config.toml")
		const withSection = { ...existing, simplex: { ...existing.simplex, overfillProtection: { maxOverfillBps: 500 } } }
		// Recorded at the moment of the log line, to show the file is already written by then.
		let writtenWhenLogged: boolean | undefined
		prompts.log.info.mockImplementation((message: string) => {
			if (DROPPED.test(message)) writtenWhenLogged = existsSync(path)
		})

		await stepWrite(updateRunState(withSection), path)

		expect(writtenWhenLogged).toBe(true)
		expect(prompts.log.info).toHaveBeenCalledWith(expect.stringMatching(DROPPED))
		expect(readFileSync(path, "utf8")).not.toContain("overfillProtection")
	})

	it("says nothing when the previous config had no such section", async () => {
		await stepWrite(updateRunState(existing), join(dir, "filler-config.toml"))

		expect(prompts.log.info).not.toHaveBeenCalledWith(expect.stringMatching(DROPPED))
	})
})
