import type { ReactNode } from "react"
import { Sheet } from "./ui/Sheet"

/** Contextual operator workspace sheet. Keeps dense actions out of the dashboard canvas. */
export function OperatorSheet(props: {
	open: boolean
	onClose: () => void
	title: string
	description?: string
	children: ReactNode
	wide?: boolean
}) {
	const { open, onClose, title, description, children, wide = false } = props
	return (
		<Sheet
			open={open}
			onOpenChange={(nextOpen) => {
				if (!nextOpen) onClose()
			}}
			title={title}
			description={description}
			wide={wide}
		>
			{children}
		</Sheet>
	)
}
