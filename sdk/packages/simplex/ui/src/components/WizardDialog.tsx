import * as Dialog from "@radix-ui/react-dialog"
import type { ReactNode } from "react"
import { CloseIcon } from "./InterfaceIcons"

/** Shared focused editor used by dense wizard sections. */
export function WizardDialog(props: {
	open: boolean
	onClose: () => void
	title: string
	description: string
	children: ReactNode
}) {
	const { open, onClose, title, description, children } = props
	return (
		<Dialog.Root
			open={open}
			onOpenChange={(nextOpen) => {
				if (!nextOpen) onClose()
			}}
		>
			<Dialog.Portal>
				<Dialog.Overlay className="dialog-overlay" />
				<Dialog.Content className="market-dialog">
					<header className="market-dialog-header">
						<div>
							<Dialog.Title asChild>
								<h2>{title}</h2>
							</Dialog.Title>
							<Dialog.Description asChild>
								<p>{description}</p>
							</Dialog.Description>
						</div>
						<Dialog.Close asChild>
							<button type="button" className="icon-button market-dialog-close" aria-label="Close">
								<CloseIcon aria-hidden="true" />
							</button>
						</Dialog.Close>
					</header>
					<div className="market-dialog-body">{children}</div>
				</Dialog.Content>
			</Dialog.Portal>
		</Dialog.Root>
	)
}
