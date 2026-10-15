import { describe, expect, it, vi } from "vitest"
import { RendererRecovery } from "../renderer-recovery"

function fixture() {
	let crashed = true
	let active = true
	const reload = vi.fn(async () => {
		crashed = false
	})
	const prompt = vi.fn(async () => false)
	const report = vi.fn()
	const recovery = new RendererRecovery({
		canRecover: () => active && crashed,
		reload,
		prompt,
		report,
	})
	return {
		recovery,
		reload,
		prompt,
		report,
		crash: () => {
			crashed = true
		},
		close: () => {
			active = false
		},
	}
}

describe("renderer recovery", () => {
	it("recovers once after teardown, then requires an operator retry instead of looping", async () => {
		const f = fixture()
		const first = f.recovery.crashed("crashed", 1)
		expect(f.reload).not.toHaveBeenCalled()
		await first
		expect(f.reload).toHaveBeenCalledTimes(1)
		f.crash()
		await f.recovery.crashed("oom", 2)
		expect(f.prompt).toHaveBeenCalledOnce()
		expect(f.reload).toHaveBeenCalledTimes(1)
		await f.recovery.show()
		expect(f.reload).toHaveBeenCalledTimes(2)
	})

	it("coalesces show and crash recovery and skips destroyed or quitting windows", async () => {
		const f = fixture()
		await Promise.all([f.recovery.crashed("killed", 2), f.recovery.show()])
		expect(f.reload).toHaveBeenCalledOnce()
		f.crash()
		const pending = f.recovery.show()
		f.close()
		await pending
		expect(f.reload).toHaveBeenCalledOnce()
	})

	it("ignores clean exits and contains reload failures", async () => {
		const f = fixture()
		await f.recovery.crashed("clean-exit", 0)
		expect(f.reload).not.toHaveBeenCalled()
		f.reload.mockRejectedValueOnce(new Error("load failed"))
		await expect(f.recovery.crashed("crashed", 1)).resolves.toBeUndefined()
		expect(f.report).toHaveBeenCalledWith(expect.objectContaining({ message: "load failed" }))
	})
})
