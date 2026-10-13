import { Badge, TimelineItem, TimelineListGroup, TimelineRoot, TokenTransferPair } from "@hyperbridge/ui"
import { ArrowUpRight } from "@hyperbridge/ui/icons"
import { Fragment, type ReactNode, useEffect, useId, useState } from "react"
import { chainByChainId } from "@/cli/init/chains"
import { parseChainKey } from "@/config/interpolated-curve"
import externalLinks from "../../../src/config/external-links.json"
import hyperbridgeLogo from "../assets/networks/hyperbridge.png"
import { chainLogoSrc } from "../components/ChainLogo"
import { ChevronDownIcon } from "../components/InterfaceIcons"
import { tokenIconSrc } from "../components/TokenIcon"
import {
	formatDisplayAmount,
	formatElapsed,
	formatStepDuration,
	offerCount,
	selectedSolver,
	shortenAddress,
	type StepLink,
	type StepStatus,
	type TimelineStep,
	type TimelineTopUp,
	timelineMoments,
	timelineOffers,
	timelineSteps,
} from "./topUpTimelineModel"

const BSC_CHAIN_ID = 56
const BRIDGE_DECIMALS = 18
const HYPERBRIDGE_MESSAGES_URL = "https://explorer.hyperbridge.network/messages"

export type TimelineTopUpView = TimelineTopUp & { amountIn: string; bridgeOut: string; pending: boolean }

function cn(...classes: (string | false | null | undefined)[]): string {
	return classes.filter(Boolean).join(" ")
}

/** Counts down to a step's expected end, then disappears, as in the Hyperbridge app. */
function Countdown(props: { date: number }) {
	const [now, setNow] = useState(Date.now())
	useEffect(() => {
		const timer = setInterval(() => setNow(Date.now()), 1000)
		return () => clearInterval(timer)
	}, [])
	const left = Math.floor((props.date - now) / 1000)
	if (left <= 0) return null
	const pad = (value: number) => String(value).padStart(2, "0")
	const hours = Math.floor(left / 3600)
	return (
		<span className="tabular-nums">
			{hours > 0 ? `${pad(hours)}:` : ""}
			{pad(Math.floor((left % 3600) / 60))}:{pad(left % 60)}
		</span>
	)
}

function linkUrl(link: StepLink | undefined, sourceChainId: number | null): string | undefined {
	if (!link) return undefined
	if (link.chain === "hyperbridge") return `${externalLinks.hyperbridgeExplorers.mainnet}/#/extrinsics/${link.hash}`
	if (link.chain === "message") return `${HYPERBRIDGE_MESSAGES_URL}/${link.hash}`
	const chainId = link.chain === "bsc" ? BSC_CHAIN_ID : sourceChainId
	const explorer = chainId === null ? undefined : chainByChainId(chainId)?.explorerUrl
	return explorer ? `${explorer}/tx/${link.hash}` : undefined
}

function timeRange(start: number, end: number): string {
	const time = (ms: number) => new Date(ms).toLocaleTimeString(undefined, { hour: "numeric", minute: "2-digit" })
	const date = new Date(end).toLocaleDateString(undefined, { month: "short", day: "numeric", year: "numeric" })
	return `${time(start)} – ${time(end)} · ${date}`
}

function headline(topUp: TimelineTopUpView): string {
	const bridgeOut = Number(topUp.bridgeOut).toLocaleString(undefined, { maximumFractionDigits: 4 })
	switch (topUp.stage) {
		case "reclaimed":
			return `Refunded ${topUp.amountIn} ${topUp.token}`
		case "reclaiming":
			return `Refunding ${topUp.amountIn} ${topUp.token}`
		case "unfilled":
			return "Order Expired"
		case "failed":
			return "Top-up Failed"
		case "delivered":
		case "filled":
			return `Bought ${bridgeOut} BRIDGE`
		default:
			return `Buying ${bridgeOut} BRIDGE`
	}
}

/** The step's dot disc, for rows without a token badge. */
function TimelineVisual(props: { status: StepStatus }) {
	return (
		<span
			aria-hidden="true"
			className="bg-brand-black-300/50 flex size-8 shrink-0 items-center justify-center rounded-full shadow-[inset_0_4px_24px_0_rgba(255,255,255,0.05)]"
		>
			<span
				className={cn(
					"size-2 rounded-full",
					props.status === "completed"
						? "bg-brand-success-400"
						: props.status === "error"
							? "bg-brand-danger-500"
							: "bg-brand-white-500",
					props.status === "processing" && "animate-pulse",
				)}
			/>
		</span>
	)
}

function CountBadge(props: { count: number }) {
	if (props.count <= 0) return null
	return (
		<Badge
			variant="secondary"
			className="bg-brand-black-300 flex h-4 min-w-4 shrink-0 items-center justify-center rounded-full px-1 text-[10px] leading-none text-white"
		>
			{props.count}
		</Badge>
	)
}

type StepperEntry = { key: string; tone: "success" | "neutral"; title: ReactNode; meta?: ReactNode }

function TimelineStepper(props: { entries: StepperEntry[] }) {
	const { entries } = props
	return (
		<ol className="border-t border-[#222429] py-3 pr-3.5 pl-[3.75rem]">
			{entries.map((entry, index) => (
				<li key={entry.key} className="relative">
					<div className={cn(index < entries.length - 1 && "pb-3")}>
						{index < entries.length - 1 ? (
							<span
								aria-hidden="true"
								className="bg-brand-black-300 absolute top-3.5 -bottom-0.5 left-[-1.875rem] w-px -translate-x-1/2"
							/>
						) : null}
						<span
							aria-hidden="true"
							className={cn(
								"absolute left-[-1.875rem] size-2 -translate-x-1/2 rounded-full",
								entries.length === 1 ? "top-1/2 -translate-y-1/2" : "top-1.5",
								entry.tone === "success" ? "bg-brand-success-400" : "bg-brand-black-100",
							)}
						/>
						<p className="text-brand-black-100 text-xs leading-5">{entry.title}</p>
						{entry.meta ? (
							<div className="text-brand-black-100 mt-0.5 flex flex-wrap items-center gap-x-1.5 text-[11px] leading-4">
								{entry.meta}
							</div>
						) : null}
					</div>
				</li>
			))}
		</ol>
	)
}

/**
 * "Offers Received", as the Hyperbridge app's swap timeline shows it: the offer count beside
 * the caption, the latest solver under it, and the offers themselves on expanding the row.
 */
function OffersRow(props: {
	topUp: TimelineTopUpView
	step: TimelineStep
	meta: ReactNode
	selectedUrl?: string
}) {
	const { topUp, step, meta, selectedUrl } = props
	const [open, setOpen] = useState(false)
	const contentId = useId()
	// Offers, their count and their solver show once the step has begun.
	const begun = step.status !== "upcoming"
	const offers = begun ? timelineOffers(topUp) : []
	const count = begun ? offerCount(topUp) : 0
	const selected = selectedSolver(topUp)?.toLowerCase()
	const placedAt = timelineMoments(topUp).placedAt
	const latest = offers.at(-1)?.preview.solverAddress ?? selected
	const secondary = !begun
		? step.secondary
		: latest
			? `Received offer from ${shortenAddress(latest)}.`
		: count > 0
			? `${count} ${count === 1 ? "offer" : "offers"} received from solvers.`
			: step.secondary

	const entries: StepperEntry[] = offers.map((offer) => {
		const isSelected = offer.preview.solverAddress.toLowerCase() === selected
		const amount = offer.preview.outputs.reduce((sum, output) => sum + BigInt(output.amount), 0n)
		const elapsed = formatElapsed(placedAt, offer.timestamp)
		const metaItems = [
			isSelected ? (
				selectedUrl ? (
					<a
						key="selected"
						href={selectedUrl}
						target="_blank"
						rel="noopener noreferrer"
						aria-label="Selected, view in explorer"
						className="text-brand-success-400 inline-flex items-center gap-0.5 transition-opacity hover:opacity-80"
					>
						Selected
						<ArrowUpRight className="size-3" />
					</a>
				) : (
					<span key="selected" className="text-brand-success-400">
						Selected
					</span>
				)
			) : null,
			elapsed ? (
				<span key="elapsed">
					Landed in <span className="text-brand-white-500">{elapsed}</span>
				</span>
			) : null,
		].filter(Boolean)
		return {
			key: offer.key,
			tone: isSelected ? "success" : "neutral",
			title: (
				<>
					<span className="text-brand-white-500">{shortenAddress(offer.preview.solverAddress)}</span> offered{" "}
					<span className="text-brand-white-500">{formatDisplayAmount(amount.toString(), BRIDGE_DECIMALS)} BRIDGE</span>
				</>
			),
			meta: metaItems.length
				? metaItems.map((item, index) => (
						<Fragment key={index}>
							{index > 0 ? (
								<span aria-hidden="true" className="text-brand-black-200">
									·
								</span>
							) : null}
							{item}
						</Fragment>
					))
				: undefined,
		}
	})
	const expandable = entries.length > 0

	return (
		<li data-process-status={step.status} className="text-brand-white-500 block bg-brand-black-350 font-medium select-none">
			<div
				className={cn(
					"relative flex h-[3.5rem] items-center gap-[0.86rem] px-[0.75rem] py-[0.5625rem] transition-colors",
					expandable && "hover:bg-white/[0.02]",
					step.status === "upcoming" && "opacity-50",
				)}
			>
				{expandable ? (
					<button
						type="button"
						aria-expanded={open}
						aria-controls={contentId}
						aria-label={`${open ? "Hide" : "Show"} ${count} ${count === 1 ? "offer" : "offers"}`}
						onClick={() => setOpen((value) => !value)}
						className="absolute inset-0 cursor-pointer focus-visible:outline-2 focus-visible:-outline-offset-2 focus-visible:outline-white"
					/>
				) : null}
				<TimelineVisual status={step.status} />
				{/* The caption and secondary line use the library row's own classes, so every row lines up. */}
				<div className="flex flex-1 flex-col gap-[calc(4/16rem)]">
					<div className="flex min-w-0 items-center gap-2">
						<div className="text-brand-white-500 text-xs md:!text-base h-auto md:h-[1.25rem] truncate">
							{step.caption}
						</div>
						<CountBadge count={count} />
					</div>
					{secondary ? <div className="text-xs h-[1rem] text-brand-black-100 truncate">{secondary}</div> : null}
				</div>
				<div className="text-brand-black-100 flex shrink-0 items-center gap-2 text-[13px]">
					{meta}
					{expandable ? (
						<ChevronDownIcon
							aria-hidden="true"
							className={cn(
								"size-4 transition-transform duration-300 ease-[cubic-bezier(0.22,1,0.36,1)]",
								open && "rotate-180",
							)}
						/>
					) : step.status === "processing" ? (
						<span aria-hidden="true" className="border-brand-black-100 size-4 animate-spin rounded-full border-2 border-t-transparent" />
					) : null}
				</div>
			</div>
			{expandable ? (
				// Always mounted: the row eases its height and fade open and shut, as the Hyperbridge app's does.
				<div
					id={contentId}
					inert={!open}
					className={cn(
						"grid transition-[grid-template-rows,opacity] duration-[400ms] ease-[cubic-bezier(0.22,1,0.36,1)]",
						open ? "grid-rows-[1fr] opacity-100" : "grid-rows-[0fr] opacity-0",
					)}
				>
					<div className="min-h-0 overflow-hidden">
						<TimelineStepper entries={entries} />
					</div>
				</div>
			) : null}
		</li>
	)
}

/**
 * The top-up as the Hyperbridge app's transaction card: the token pair, a headline, then the
 * order (placed, offers, fulfilled on BNB Chain) and the BRIDGE's trip into Hyperbridge
 * (finalized, verified), each with its countdown while running and its duration once done.
 */
export function BridgeTopUpTimeline(props: { topUp: TimelineTopUpView }) {
	const { topUp } = props
	const sourceChainId = parseChainKey(topUp.chain)
	const networkLabel = (sourceChainId !== null && chainByChainId(sourceChainId)?.label) || topUp.chain
	const steps = timelineSteps(topUp, networkLabel)

	const paid = {
		src: tokenIconSrc(topUp.token),
		alt: topUp.token,
		badgeSrc: chainLogoSrc(networkLabel) ?? "",
		badgeAlt: networkLabel,
	}
	// The BRIDGE token on each network it passes through; the Hyperbridge logo is only the network badge.
	const bridgeToken = tokenIconSrc("BRIDGE")
	const onBsc = { src: bridgeToken, alt: "BRIDGE", badgeSrc: chainLogoSrc("BNB Chain") ?? "", badgeAlt: "BNB Chain" }
	const onHyperbridge = { src: bridgeToken, alt: "BRIDGE", badgeSrc: hyperbridgeLogo, badgeAlt: "Hyperbridge" }
	const badges: Partial<Record<TimelineStep["key"], typeof paid>> = {
		placed: paid,
		fulfilled: onBsc,
		claimed: paid,
		// Recovery: BSC's expiry finality with the BRIDGE the order wanted, the refund with what was paid.
		DESTINATION_FINALIZED: onBsc,
		FUNDS_CLAIMED: paid,
	}

	const timing = (step: TimelineStep): ReactNode => {
		if (!step.timed) return null
		if (step.status === "completed") return step.duration !== undefined ? formatStepDuration(step.duration) : null
		if (step.status === "processing") return step.etaAt ? <Countdown date={step.etaAt} /> : null
		return null
	}

	const renderStep = (step: TimelineStep) => {
		if (step.key === "offers") {
			const selectedUrl = linkUrl(topUp.fillTxHash ? { chain: "bsc", hash: topUp.fillTxHash } : undefined, sourceChainId)
			return <OffersRow key={step.key} topUp={topUp} step={step} meta={timing(step)} selectedUrl={selectedUrl} />
		}
		const badge = badges[step.key] ?? (step.key === "verified" ? onHyperbridge : paid)
		const level = step.level === 0 ? { level: 0 as const, badge } : { level: 1 as const }
		const common = {
			caption: step.caption,
			secondary: step.secondary,
			durationMode: step.timed ? ("visible" as const) : ("none" as const),
		}
		if (step.status === "completed") {
			return (
				<TimelineItem
					key={step.key}
					{...common}
					{...level}
					processingStatus="completed"
					data={{
						transactionUrl: linkUrl(step.link, sourceChainId) ?? "",
						completedIn: (timing(step) ?? "") as never,
					}}
				/>
			)
		}
		if (step.status === "processing") {
			return (
				<TimelineItem key={step.key} {...common} {...level} processingStatus="processing" data={{ eta: timing(step) }} />
			)
		}
		return <TimelineItem key={step.key} {...common} {...level} processingStatus={step.status} />
	}

	// Level 1 steps nest under the level 0 step before them.
	const groups: { head: TimelineStep; children: TimelineStep[] }[] = []
	for (const step of steps) {
		if (step.level === 1 && groups.length > 0) groups[groups.length - 1].children.push(step)
		else groups.push({ head: step, children: [] })
	}

	return (
		<div className="flex flex-col gap-[calc(24rem/16)]">
			<section className="flex flex-col items-center gap-[1rem]">
				<TokenTransferPair size="2.5rem" fromToken={paid} toToken={onHyperbridge} />
				<div className="flex flex-col items-center gap-1 text-center">
					<h2 className="text-brand-white-500 text-2xl font-medium">{headline(topUp)}</h2>
					<p className="text-brand-black-100 text-xs">
						{topUp.amountIn} {topUp.token} on {networkLabel}
					</p>
				</div>
			</section>

			<TimelineRoot>
				{groups.map(({ head, children }) => (
					<Fragment key={head.key}>
						{renderStep(head)}
						{children.length > 0 && <TimelineListGroup>{children.map(renderStep)}</TimelineListGroup>}
					</Fragment>
				))}
			</TimelineRoot>

			{!topUp.pending && (
				<p className="text-brand-black-100 text-center text-xs">{timeRange(topUp.createdAt, topUp.updatedAt)}</p>
			)}
		</div>
	)
}
