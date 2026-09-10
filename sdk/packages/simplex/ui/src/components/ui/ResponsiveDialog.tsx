import * as Dialog from "@radix-ui/react-dialog"
import {
	createContext,
	useContext,
	type ComponentPropsWithoutRef,
	type ReactElement,
	type ReactNode,
} from "react"
import { useIsMobile } from "../../lib/hooks"
import { Drawer, DrawerClose, DrawerContent, DrawerDescription, DrawerTitle } from "./Drawer"

const ResponsiveDialogContext = createContext(false)

export function ResponsiveDialog(props: {
	open: boolean
	onOpenChange?: (open: boolean) => void
	children: ReactNode
	dismissible?: boolean
}) {
	const mobile = useIsMobile()
	const { open, onOpenChange, children, dismissible = true } = props
	return (
		<ResponsiveDialogContext.Provider value={mobile}>
			{mobile ? (
				<Drawer
					open={open}
					onOpenChange={onOpenChange}
					showSwipeHandle
					disablePointerDismissal={!dismissible}
				>
					{children}
				</Drawer>
			) : (
				<Dialog.Root open={open} onOpenChange={onOpenChange}>
					{children}
				</Dialog.Root>
			)}
		</ResponsiveDialogContext.Provider>
	)
}

export function ResponsiveDialogContent(props: {
	className: string
	overlayClassName: string
	children: ReactNode
	ariaDescribedBy?: string
	preventEscapeKeyDown?: boolean
	wide?: boolean
}) {
	const mobile = useContext(ResponsiveDialogContext)
	const { className, overlayClassName, children, ariaDescribedBy, preventEscapeKeyDown = false, wide = false } = props
	if (mobile) {
		return (
			<DrawerContent
				className={className}
				overlayClassName={overlayClassName}
				aria-describedby={ariaDescribedBy}
				data-wide={wide || undefined}
			>
				{children}
			</DrawerContent>
		)
	}
	return (
		<Dialog.Portal>
			<Dialog.Overlay className={overlayClassName} />
			<Dialog.Content
				className={className}
				aria-describedby={ariaDescribedBy}
				data-wide={wide || undefined}
				onEscapeKeyDown={preventEscapeKeyDown ? (event) => event.preventDefault() : undefined}
			>
				{children}
			</Dialog.Content>
		</Dialog.Portal>
	)
}

export function ResponsiveDialogTitle(props: ComponentPropsWithoutRef<"h2">) {
	const mobile = useContext(ResponsiveDialogContext)
	return mobile ? <DrawerTitle {...props} /> : <Dialog.Title {...props} />
}

export function ResponsiveDialogDescription(props: ComponentPropsWithoutRef<"p">) {
	const mobile = useContext(ResponsiveDialogContext)
	return mobile ? <DrawerDescription {...props} /> : <Dialog.Description {...props} />
}

export function ResponsiveDialogClose(props: { children: ReactElement<{ children?: ReactNode }> }) {
	const mobile = useContext(ResponsiveDialogContext)
	return mobile ? (
		<DrawerClose render={props.children}>{props.children.props.children}</DrawerClose>
	) : (
		<Dialog.Close asChild>{props.children}</Dialog.Close>
	)
}
