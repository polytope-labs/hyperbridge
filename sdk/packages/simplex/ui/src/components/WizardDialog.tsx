import type { ReactNode } from "react"
import { CloseIcon } from "./InterfaceIcons"
import {
	ResponsiveDialog,
	ResponsiveDialogClose,
	ResponsiveDialogContent,
	ResponsiveDialogDescription,
	ResponsiveDialogTitle,
} from "./ui/ResponsiveDialog"

/** Shared focused editor used by dense wizard sections. */
export function WizardDialog(props: {
	open: boolean
	onClose: () => void
	title: string
	description: string
	children: ReactNode
	/** Added to the dialog, e.g. to size it to its content. */
	className?: string
}) {
	const { open, onClose, title, description, children, className } = props
	return (
		<ResponsiveDialog
			open={open}
			onOpenChange={(nextOpen) => {
				if (!nextOpen) onClose()
			}}
		>
			<ResponsiveDialogContent
				className={className ? `market-dialog ${className}` : "market-dialog"}
				overlayClassName="dialog-overlay"
			>
					<header className="market-dialog-header">
						<div>
							<ResponsiveDialogTitle>{title}</ResponsiveDialogTitle>
							<ResponsiveDialogDescription>{description}</ResponsiveDialogDescription>
						</div>
						<ResponsiveDialogClose>
							<button type="button" className="icon-button market-dialog-close" aria-label="Close">
								<CloseIcon aria-hidden="true" />
							</button>
						</ResponsiveDialogClose>
					</header>
					<div className="market-dialog-body">{children}</div>
			</ResponsiveDialogContent>
		</ResponsiveDialog>
	)
}
