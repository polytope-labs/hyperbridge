import type { SVGProps } from "react"

const INSTALL_STEPS = [
	{
		title: "Select the install icon",
		description: "Look for the small screen with a downward arrow at the top of this window.",
		visual: "toolbar",
	},
	{
		title: "Choose Install",
		description: "A confirmation window will appear. Select Install to continue.",
		visual: "confirm",
	},
	{
		title: "Open Simplex from your desktop",
		description: "Simplex will appear with your other apps and open in its own window.",
		visual: "desktop",
	},
] as const

export function InstallGuidePanel() {
	return (
		<section className="install-guide-panel" aria-labelledby="install-guide-title">
			<header>
				<span className="eyebrow">Three simple steps</span>
				<h2 id="install-guide-title">Save Simplex to your desktop</h2>
				<p>Install once, then launch Simplex like any other desktop app.</p>
			</header>
			<ol className="install-step-list">
				{INSTALL_STEPS.map((step, index) => (
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

function InstallStepVisual(props: { visual: (typeof INSTALL_STEPS)[number]["visual"] }) {
	if (props.visual === "toolbar") return <ToolbarVisual />
	if (props.visual === "confirm") return <ConfirmVisual />
	return <DesktopVisual />
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
			<img src="./icons/simplex-192.png" alt="" />
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
				<img src="./icons/simplex-192.png" alt="" />
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
