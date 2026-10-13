import { Button, Dialog, DialogContent, DialogTitle } from "@hyperbridge/ui"
import { useState } from "react"
import { api } from "../api"
import { MinusIcon } from "../components/InterfaceIcons"
import { BridgeTopUpTimeline, type TimelineTopUpView } from "./BridgeTopUpTimeline"
import { FILL_CHAIN } from "./topUpTimelineModel"

/**
 * The top-up's progress, in the transaction dialog the Hyperbridge app uses for a bridge:
 * minimised while it runs and reopened from the BRIDGE tile, it offers to reclaim an order
 * that went unfilled.
 */
export function BridgeTopUpProgressDialog(props: {
	open: boolean
	onOpenChange: (open: boolean) => void
	topUp: TimelineTopUpView | undefined
	/** Asks the parent to re-read the top-up after a reclaim starts. */
	onChanged: () => void
}) {
	const { open, onOpenChange, topUp, onChanged } = props
	// An order paid on BSC itself is cancelled and refunded in one step; any other needs a recovery.
	const sameChain = topUp?.chain === FILL_CHAIN
	const [reclaiming, setReclaiming] = useState(false)
	const [error, setError] = useState<string>()

	const reclaim = async () => {
		setReclaiming(true)
		setError(undefined)
		try {
			await api.post("/api/bridge-top-up/reclaim")
			onChanged()
		} catch (err) {
			setError(err instanceof Error ? err.message : String(err))
		} finally {
			setReclaiming(false)
		}
	}

	return (
		<Dialog open={open && !!topUp} onOpenChange={onOpenChange}>
			<DialogContent className="hb-ui gap-0" showCloseButton={false}>
				<DialogTitle>
					<span className="sr-only">BRIDGE top-up</span>
				</DialogTitle>

				<button
					type="button"
					onClick={() => onOpenChange(false)}
					className="group bg-brand-black-350 hover:bg-brand-black-500 text-brand-white-500 absolute end-4 top-4 inline-flex h-8 min-w-8 cursor-pointer items-center justify-center rounded-full p-2"
				>
					<MinusIcon width={16} height={16} aria-hidden="true" />
					<span className="inline-block w-0 overflow-hidden text-xs tracking-wider transition-[width] group-hover:w-[8ch] group-focus:w-[8ch]">
						Minimize
					</span>
				</button>

				{topUp && (
					<div className="flex flex-col gap-[calc(24rem/16)] py-4">
						<BridgeTopUpTimeline topUp={topUp} />

						{topUp.stage === "unfilled" && (
							<div className="border-brand-warning-500/30 bg-brand-warning-500/10 text-brand-warning-500 rounded-xl border px-4 py-3 text-center text-xs">
								{sameChain
									? "This swap timed out. Claim funds to refund your tokens."
									: "This cross-chain swap timed out. Start recovery to return funds to the source chain."}
							</div>
						)}

						{(topUp.error || error) && (
							<p className="text-brand-danger-500 text-center text-xs break-words">{error ?? topUp.error}</p>
						)}

						{topUp.stage === "unfilled" && (
							<Button variant="primary" onClick={() => void reclaim()} disabled={reclaiming}>
								{reclaiming
									? sameChain
										? "Claiming refund"
										: "Starting recovery"
									: sameChain
										? "Claim funds"
										: "Start recovery"}
							</Button>
						)}
					</div>
				)}
			</DialogContent>
		</Dialog>
	)
}
