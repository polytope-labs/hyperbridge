import type { SVGProps } from "react"

export type InstallPlatform = "mobile" | "desktop"

type InstallStep = {
	title: string
	description: string
	visual: "toolbar" | "confirm" | "desktop" | "browser-menu" | "home-screen"
}

type InstallGuide = { eyebrow: string; title: string; description: string; steps: InstallStep[] }

const GUIDES: Record<InstallPlatform, InstallGuide> = {
	desktop: {
		eyebrow: "Three simple steps",
		title: "Save Simplex to your desktop",
		description: "Install once, then launch Simplex like any other desktop app.",
		steps: [
			{ title: "Select the install icon", description: "Look for the small screen with a downward arrow at the top of this window.", visual: "toolbar" },
			{ title: "Choose Install", description: "A confirmation window will appear. Select Install to continue.", visual: "confirm" },
			{ title: "Open Simplex from your desktop", description: "Simplex will appear with your other apps and open in its own window.", visual: "desktop" },
		],
	},
	mobile: {
		eyebrow: "Install on mobile",
		title: "Save Simplex to your home screen",
		description: "Use your browser’s install or share menu to save Simplex for quicker access and offline startup.",
		steps: [
			{ title: "Open your browser menu", description: "Look for the menu or Share control in the browser you use.", visual: "browser-menu" },
			{ title: "Choose Add to Home Screen or Install", description: "Choose Add to Home Screen or Install app depending on the platform", visual: "confirm" },
			{ title: "Confirm, then open Simplex", description: "Simplex is added to your home screen and opens as its own app.", visual: "home-screen" },
		],
	},
}

export function installPlatform(): InstallPlatform {
	const userAgent = navigator.userAgent
	const isIpad = /iPad/.test(userAgent) || (navigator.platform === "MacIntel" && navigator.maxTouchPoints > 1)
	return /Android|iPhone|iPod|Mobile/.test(userAgent) || isIpad ? "mobile" : "desktop"
}

export function InstallGuidePanel(props: { platform: InstallPlatform }) {
	const guide = GUIDES[props.platform]
	return (
		<section className="install-guide-panel" aria-labelledby="install-guide-title">
			<header>
				<span className="eyebrow">{guide.eyebrow}</span>
				<h2 id="install-guide-title">{guide.title}</h2>
				<p>{guide.description}</p>
			</header>
			<ol className="install-step-list">
				{guide.steps.map((step, index) => (
					<li className="install-step" key={step.title}>
						<span className="install-step-number" aria-hidden="true">
							{index + 1}
						</span>
						<div className="install-step-copy">
							<strong>{step.title}</strong>
							<p>{step.description}</p>
						</div>
						<InstallStepVisual visual={step.visual} />
					</li>
				))}
			</ol>
		</section>
	)
}

function InstallStepVisual(props: { visual: InstallStep["visual"] }) {
	if (props.visual === "toolbar") return <ToolbarVisual />
	if (props.visual === "confirm") return <ConfirmVisual />
	if (props.visual === "browser-menu") return <BrowserMenuVisual />
	if (props.visual === "home-screen") return <HomeScreenVisual />
	return <DesktopVisual />
}

function BrowserMenuVisual() {
	return (
		<div className="install-visual install-mobile-action-visual" aria-hidden="true">
			<span>Browser</span>
			<b>⋮</b>
			<small>Install or add</small>
		</div>
	)
}

function HomeScreenVisual() {
	return (
		<div className="install-visual install-desktop-visual" aria-hidden="true">
			<div>
				<img src="./icons/mobile-logo.svg" alt="" />
				<span>Simplex</span>
			</div>
			<small>Home screen</small>
		</div>
	)
}

function ToolbarVisual() {
	return (
		<div className="install-visual install-toolbar-visual" aria-hidden="true">
			<div className="install-window-controls">
				<span />
				<span />
				<span />
			</div>
			<div className="install-window-address">Simplex</div>
			<span className="install-target-icon">
				<InstallDesktopIcon />
			</span>
			<small>Click this icon</small>
		</div>
	)
}

function ConfirmVisual() {
	return (
		<div className="install-visual install-confirm-visual" aria-hidden="true">
			<img src="./icons/mobile-logo.svg" alt="" />
			<span>
				<strong>Install Simplex?</strong>
				<small>Opens in its own window</small>
			</span>
			<b>Install</b>
		</div>
	)
}

function DesktopVisual() {
	return (
		<div className="install-visual install-desktop-visual" aria-hidden="true">
			<div>
				<img src="./icons/mobile-logo.svg" alt="" />
				<span>Simplex</span>
			</div>
			<small>Desktop · Dock · App list</small>
		</div>
	)
}

function InstallDesktopIcon(props: SVGProps<SVGSVGElement>) {
	return (
		<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.7" {...props}>
			<rect x="3" y="4" width="18" height="13" rx="2" />
			<path d="M12 7v6m0 0 2.5-2.5M12 13l-2.5-2.5M8 21h8M12 17v4" strokeLinecap="round" />
		</svg>
	)
}
