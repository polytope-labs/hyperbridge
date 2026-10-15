import { afterEach, describe, expect, it, vi } from "vitest"
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
	afterEach(() => vi.useRealTimers())

	it("lets crash recovery own an initial navigation rejection that arrives before the crash event", async () => {
		const f = fixture()
		const startup = f.recovery.loadInitial(async () => {
			throw new Error("ERR_FAILED")
		})
		// Linux can reject loadURL before reporting the renderer's termination.
		await new Promise((resolve) => setTimeout(resolve, 10))
		await f.recovery.crashed("crashed", 133)
		await expect(startup).resolves.toBeUndefined()
		expect(f.reload).toHaveBeenCalledOnce()
	})

	it("also accepts a navigation rejection after recovery has already reloaded the renderer", async () => {
		const f = fixture()
		await f.recovery.crashed("crashed", 1)
		await expect(f.recovery.loadInitial(async () => {
			throw new Error("ERR_ABORTED")
		})).resolves.toBeUndefined()
		expect(f.reload).toHaveBeenCalledOnce()
	})

	it("bounds the wait and preserves startup errors when no recoverable crash is reported", async () => {
		vi.useFakeTimers()
		const f = fixture()
		const error = new Error("navigation failed")
		const startup = f.recovery.loadInitial(async () => { throw error })
		const rejected = expect(startup).rejects.toBe(error)
		await f.recovery.crashed("clean-exit", 0)
		await vi.advanceTimersByTimeAsync(1000)
		await rejected
		expect(f.reload).not.toHaveBeenCalled()
		expect(vi.getTimerCount()).toBe(0)
	})

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
