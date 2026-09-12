import type { ReactNode } from "react"
import { CloseIcon } from "../InterfaceIcons"
import {
	ResponsiveDialog,
	ResponsiveDialogClose,
	ResponsiveDialogContent,
	ResponsiveDialogDescription,
	ResponsiveDialogTitle,
} from "./ResponsiveDialog"

interface SheetProps {
	open: boolean
	onOpenChange: (open: boolean) => void
	title: string
	description?: string
	children: ReactNode
	wide?: boolean
}

/** Accessible right-side sheet following the shadcn/Radix composition model. */
export function Sheet(props: SheetProps) {
	const { open, onOpenChange, title, description, children, wide = false } = props
	return (
		<ResponsiveDialog open={open} onOpenChange={onOpenChange}>
			<ResponsiveDialogContent className="sheet-content" overlayClassName="sheet-overlay" wide={wide}>
					<header className="sheet-header">
						<div>
							<span className="eyebrow">Simplex operator</span>
							<ResponsiveDialogTitle>{title}</ResponsiveDialogTitle>
							{description ? (
								<ResponsiveDialogDescription>{description}</ResponsiveDialogDescription>
							) : null}
						</div>
						<ResponsiveDialogClose>
							<button type="button" className="icon-button" aria-label="Close panel">
								<CloseIcon aria-hidden="true" />
							</button>
						</ResponsiveDialogClose>
					</header>
					<div className="sheet-body">{children}</div>
			</ResponsiveDialogContent>
		</ResponsiveDialog>
	)
}
