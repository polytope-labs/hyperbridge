import { useCallback, useEffect, useState } from "react"

export type OperatorTab = "overview" | "orders" | "wallet" | "logs" | "operations"

/**
 * One path per sidebar page. Single segments only: index.html loads its
 * assets relatively (`./assets/…`), so a nested path would resolve them under
 * the wrong directory. The server serves index.html for any unknown path, so
 * a reload on any of these lands back on the same page.
 */
export const TAB_PATHS: Record<OperatorTab, string> = {
	overview: "/",
	orders: "/orders",
	wallet: "/wallet",
	logs: "/logs",
	operations: "/operations",
}

export function tabFromPath(pathname: string): OperatorTab {
	const trimmed = pathname.replace(/\/+$/, "") || "/"
	const match = (Object.entries(TAB_PATHS) as Array<[OperatorTab, string]>).find(([, path]) => path === trimmed)
	return match?.[0] ?? "overview"
}

/**
 * The active sidebar page, backed by the URL: reads the path on load, pushes a
 * history entry on navigation, and follows the browser's back and forward.
 */
export function useTabRoute(): [OperatorTab, (tab: OperatorTab, options?: { replace?: boolean }) => void] {
	const [tab, setTab] = useState<OperatorTab>(() => tabFromPath(window.location.pathname))

	useEffect(() => {
		const onPopState = () => setTab(tabFromPath(window.location.pathname))
		window.addEventListener("popstate", onPopState)
		return () => window.removeEventListener("popstate", onPopState)
	}, [])

	// `replace` is for a redirect rather than a choice: pushing an entry for a
	// page the app just sent you away from makes Back bounce off it forever.
	const navigate = useCallback((next: OperatorTab, options?: { replace?: boolean }) => {
		if (window.location.pathname !== TAB_PATHS[next]) {
			const url = TAB_PATHS[next] + window.location.search
			if (options?.replace) window.history.replaceState(null, "", url)
			else window.history.pushState(null, "", url)
		}
		setTab(next)
	}, [])

	return [tab, navigate]
}
