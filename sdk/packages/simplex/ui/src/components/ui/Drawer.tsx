import { Drawer as DrawerPrimitive } from "@base-ui/react/drawer"
import { createContext, useContext, useMemo, type ReactNode } from "react"

interface DrawerContextValue {
	modal: DrawerPrimitive.Root.Props["modal"]
	showSwipeHandle: boolean
}

const DrawerContext = createContext<DrawerContextValue | null>(null)

export function Drawer({
	modal = true,
	showSwipeHandle = false,
	...props
}: DrawerPrimitive.Root.Props & { showSwipeHandle?: boolean }) {
	const contextValue = useMemo(() => ({ modal, showSwipeHandle }), [modal, showSwipeHandle])
	return (
		<DrawerContext.Provider value={contextValue}>
			<DrawerPrimitive.Root modal={modal} swipeDirection="down" {...props} />
		</DrawerContext.Provider>
	)
}

export const DrawerClose = DrawerPrimitive.Close
export const DrawerTitle = DrawerPrimitive.Title
export const DrawerDescription = DrawerPrimitive.Description

export function DrawerContent({
	className,
	children,
	overlayClassName,
	...props
}: Omit<DrawerPrimitive.Popup.Props, "className"> & {
	className?: string
	overlayClassName?: string
	children: ReactNode
}) {
	const context = useContext(DrawerContext)
	if (!context) throw new Error("DrawerContent must be rendered inside Drawer")

	return (
		<DrawerPrimitive.Portal>
			{context.modal === true ? (
				<DrawerPrimitive.Backdrop className={`drawer-overlay ${overlayClassName ?? ""}`} />
			) : null}
			<DrawerPrimitive.Viewport className="drawer-viewport" data-modal={context.modal}>
				<DrawerPrimitive.Popup className={`drawer-popup ${className ?? ""}`} {...props}>
					{context.showSwipeHandle ? <div className="drawer-swipe-handle" aria-hidden="true" /> : null}
					<DrawerPrimitive.Content className="drawer-content">{children}</DrawerPrimitive.Content>
				</DrawerPrimitive.Popup>
			</DrawerPrimitive.Viewport>
		</DrawerPrimitive.Portal>
	)
}
