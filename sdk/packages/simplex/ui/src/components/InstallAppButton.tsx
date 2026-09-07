import * as Dialog from "@radix-ui/react-dialog"
import {
	createContext,
	useCallback,
	useContext,
	useEffect,
	useMemo,
	useState,
	type ReactNode,
} from "react"
import { toast } from "sonner"
import { InstallGuidePanel } from "./InstallGuide"
import { CheckIcon, CloseIcon, DownloadIcon } from "./InterfaceIcons"

interface BeforeInstallPromptEvent extends Event {
	prompt(): Promise<void>
	readonly userChoice: Promise<{
		outcome: "accepted" | "dismissed"
		platform: string
	}>
}

function isStandalone(): boolean {
	const standaloneNavigator = navigator as Navigator & { standalone?: boolean }
	return window.matchMedia("(display-mode: standalone)").matches || standaloneNavigator.standalone === true
}

type InstallStatus = "installed" | "prompt-ready" | "manual"

interface InstallAppContextValue {
	status: InstallStatus
	openGuide(): void
}

const InstallAppContext = createContext<InstallAppContextValue | null>(null)

export function InstallAppProvider(props: { children: ReactNode }) {
	const [deferredPrompt, setDeferredPrompt] = useState<BeforeInstallPromptEvent>()
	const [installed, setInstalled] = useState(isStandalone)
	const [guideOpen, setGuideOpen] = useState(false)

	useEffect(() => {
		const handleBeforeInstallPrompt = (event: Event) => {
			event.preventDefault()
			setDeferredPrompt(event as BeforeInstallPromptEvent)
		}
		const handleAppInstalled = () => {
			setDeferredPrompt(undefined)
			setInstalled(true)
			setGuideOpen(false)
		}

		window.addEventListener("beforeinstallprompt", handleBeforeInstallPrompt)
		window.addEventListener("appinstalled", handleAppInstalled)
		return () => {
			window.removeEventListener("beforeinstallprompt", handleBeforeInstallPrompt)
			window.removeEventListener("appinstalled", handleAppInstalled)
		}
	}, [])

	const status: InstallStatus = installed ? "installed" : deferredPrompt ? "prompt-ready" : "manual"
	const openGuide = useCallback(() => {
		setGuideOpen(true)
	}, [])
	const install = useCallback(async () => {
		if (!deferredPrompt) {
			toast.info("Use the install icon", {
				description: "Select the icon shown in step 1, then choose Install.",
			})
			return
		}
		try {
			await deferredPrompt.prompt()
			const choice = await deferredPrompt.userChoice
			if (choice.outcome === "accepted") {
				setInstalled(true)
				setGuideOpen(false)
			} else {
				toast.info("Installation was cancelled", {
					description: "Open this guide again when you’re ready to retry.",
				})
			}
		} catch {
			toast.error("The install window could not open", {
				description: "Select the icon shown in step 1 instead.",
			})
		} finally {
			setDeferredPrompt(undefined)
		}
	}, [deferredPrompt])
	const contextValue = useMemo(() => ({ status, openGuide }), [status, openGuide])

	return (
		<InstallAppContext.Provider value={contextValue}>
			{props.children}
			<InstallAppDialog
				open={guideOpen}
				onOpenChange={setGuideOpen}
				status={status}
				onInstall={install}
			/>
		</InstallAppContext.Provider>
	)
}

export function InstallAppButton(props: { variant?: "header" | "nav" }) {
	const { status, openGuide } = useInstallApp()
	const installed = status === "installed"
	const Icon = installed ? CheckIcon : DownloadIcon
	if (props.variant === "nav") {
		return (
			<button
				type="button"
				className="operator-nav-item operator-install-nav"
				onClick={openGuide}
				aria-haspopup="dialog"
				title={installed ? "Simplex is installed" : "Install Simplex app"}
			>
				<Icon aria-hidden="true" />
				<span>
					<strong>{installed ? "App installed" : "Install app"}</strong>
					<small>{installed ? "Standalone mode" : "Desktop and offline"}</small>
				</span>
			</button>
		)
	}

	return (
		<button
			type="button"
			className="pwa-install-control"
			data-state={installed ? "installed" : undefined}
			onClick={openGuide}
			aria-haspopup="dialog"
			title={installed ? "Simplex is installed" : "Install Simplex app"}
		>
			<Icon aria-hidden="true" />
			<span className="pwa-install-label">{installed ? "Installed" : "Install app"}</span>
		</button>
	)
}

function useInstallApp(): InstallAppContextValue {
	const context = useContext(InstallAppContext)
	if (!context) throw new Error("InstallAppButton must be rendered inside InstallAppProvider")
	return context
}

function InstallAppDialog(props: {
	open: boolean
	onOpenChange(open: boolean): void
	status: InstallStatus
	onInstall(): Promise<void>
}) {
	const installed = props.status === "installed"
	return (
		<Dialog.Root open={props.open} onOpenChange={props.onOpenChange}>
			<Dialog.Portal>
				<Dialog.Overlay className="install-dialog-overlay" />
				<Dialog.Content className="install-dialog" aria-describedby="install-dialog-description">
					<header className="install-dialog-header">
						<img src="./icons/simplex-192.png" alt="" />
						<div>
							<span className="eyebrow">Simplex desktop app</span>
							<Dialog.Title>{installed ? "Simplex is installed" : "Install Simplex"}</Dialog.Title>
							<Dialog.Description id="install-dialog-description">
								{installed
									? "You’re running Simplex as a standalone app."
									: "Save Simplex to your desktop for quicker access and a dedicated app window."}
							</Dialog.Description>
						</div>
						<Dialog.Close asChild>
							<button type="button" className="install-dialog-close" aria-label="Close install guide">
								<CloseIcon aria-hidden="true" />
							</button>
						</Dialog.Close>
					</header>

					{installed ? (
						<div className="install-success">
							<span className="install-success-icon">
								<CheckIcon aria-hidden="true" />
							</span>
							<h2>You’re all set</h2>
							<p>Open Simplex from your desktop, Dock, taskbar, Start menu, or Applications folder.</p>
							<Dialog.Close asChild>
								<button type="button" className="primary">
									Done
								</button>
							</Dialog.Close>
						</div>
					) : (
						<>
							<div className="install-guide-content">
								<InstallGuidePanel />
							</div>

							<footer className="install-dialog-footer">
								<span>Usually takes less than a minute.</span>
								<div>
									<Dialog.Close asChild>
										<button type="button" className="secondary">
											Maybe later
										</button>
									</Dialog.Close>
									<button type="button" className="primary" onClick={() => void props.onInstall()}>
										<DownloadIcon aria-hidden="true" />
										Install Simplex
									</button>
								</div>
							</footer>
						</>
					)}
				</Dialog.Content>
			</Dialog.Portal>
		</Dialog.Root>
	)
}
