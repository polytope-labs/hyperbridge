import type { SVGProps } from "react"

type IconProps = SVGProps<SVGSVGElement>

export function ChartLineIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" {...props}>
			<path d="M4 4v16h16" strokeLinecap="round" strokeLinejoin="round" />
			<path d="m7 15 3.5-4 3 2.25L19 7" strokeLinecap="round" strokeLinejoin="round" />
		</svg>
	)
}

export function CheckIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="2" {...props}>
			<path d="m4.5 10 3.25 3.25L15.5 5.5" strokeLinecap="round" strokeLinejoin="round" />
		</svg>
	)
}

export function ChevronDownIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.8" {...props}>
			<path d="m5 7.5 5 5 5-5" strokeLinecap="round" strokeLinejoin="round" />
		</svg>
	)
}

export function CloseIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.7" {...props}>
			<path d="m5.5 5.5 9 9m0-9-9 9" strokeLinecap="round" />
		</svg>
	)
}

export function CopyIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" {...props}>
			<rect x="6.5" y="6.5" width="9" height="9" rx="1.5" />
			<path d="M13.5 6.5v-2A1.5 1.5 0 0 0 12 3H4.5A1.5 1.5 0 0 0 3 4.5V12A1.5 1.5 0 0 0 4.5 13.5h2" />
		</svg>
	)
}

export function ExternalLinkIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" {...props}>
			<path d="M8 5H5.5A1.5 1.5 0 0 0 4 6.5v8A1.5 1.5 0 0 0 5.5 16h8a1.5 1.5 0 0 0 1.5-1.5V12" />
			<path d="M11 4h5v5M16 4l-7 7" strokeLinecap="round" strokeLinejoin="round" />
		</svg>
	)
}

export function OverviewIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" {...props}>
			<rect x="3" y="3" width="5.5" height="5.5" rx="1.2" />
			<rect x="11.5" y="3" width="5.5" height="5.5" rx="1.2" />
			<rect x="3" y="11.5" width="5.5" height="5.5" rx="1.2" />
			<rect x="11.5" y="11.5" width="5.5" height="5.5" rx="1.2" />
		</svg>
	)
}

export function ActivityIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" {...props}>
			<path d="M2.5 10h3l2-5 4.2 10 2.1-5H17.5" strokeLinecap="round" strokeLinejoin="round" />
		</svg>
	)
}

export function WalletIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" {...props}>
			<path d="M3 5.5A2.5 2.5 0 0 1 5.5 3h9A1.5 1.5 0 0 1 16 4.5v11A1.5 1.5 0 0 1 14.5 17h-9A2.5 2.5 0 0 1 3 14.5v-9Z" />
			<path d="M13 8h4v4h-4a2 2 0 1 1 0-4Z" />
		</svg>
	)
}

export function OperationsIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" {...props}>
			<path d="M4 4v12M10 4v12M16 4v12" strokeLinecap="round" />
			<circle cx="4" cy="7" r="1.8" fill="currentColor" stroke="none" />
			<circle cx="10" cy="13" r="1.8" fill="currentColor" stroke="none" />
			<circle cx="16" cy="9" r="1.8" fill="currentColor" stroke="none" />
		</svg>
	)
}

export function SettingsIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.6" {...props}>
			<circle cx="10" cy="10" r="2.5" />
			<path
				d="M16.2 11.5a6.6 6.6 0 0 0 0-3l1.3-1-1.5-2.6-1.6.7A6.4 6.4 0 0 0 12 4.2L11.8 2H8.2L8 4.2a6.4 6.4 0 0 0-2.4 1.4L4 4.9 2.5 7.5l1.3 1a6.6 6.6 0 0 0 0 3l-1.3 1L4 15.1l1.6-.7A6.4 6.4 0 0 0 8 15.8l.2 2.2h3.6l.2-2.2a6.4 6.4 0 0 0 2.4-1.4l1.6.7 1.5-2.6-1.3-1Z"
				strokeLinejoin="round"
			/>
		</svg>
	)
}

export function ChevronRightIcon(props: IconProps) {
	return (
		<svg viewBox="0 0 20 20" fill="none" stroke="currentColor" strokeWidth="1.8" {...props}>
			<path d="m7.5 5 5 5-5 5" strokeLinecap="round" strokeLinejoin="round" />
		</svg>
	)
}
