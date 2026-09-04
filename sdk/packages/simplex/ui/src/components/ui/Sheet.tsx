import * as Dialog from "@radix-ui/react-dialog"
import type { ReactNode } from "react"
import { CloseIcon } from "../InterfaceIcons"

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
		<Dialog.Root open={open} onOpenChange={onOpenChange}>
			<Dialog.Portal>
				<Dialog.Overlay className="sheet-overlay" />
				<Dialog.Content className="sheet-content" data-wide={wide || undefined}>
					<header className="sheet-header">
						<div>
							<span className="eyebrow">Simplex operator</span>
							<Dialog.Title>{title}</Dialog.Title>
							{description ? <Dialog.Description>{description}</Dialog.Description> : null}
						</div>
						<Dialog.Close asChild>
							<button type="button" className="icon-button" aria-label="Close panel">
								<CloseIcon aria-hidden="true" />
							</button>
						</Dialog.Close>
					</header>
					<div className="sheet-body">{children}</div>
				</Dialog.Content>
			</Dialog.Portal>
		</Dialog.Root>
	)
}
