import { Component, type ErrorInfo, type ReactNode } from "react"

interface ScreenErrorBoundaryProps {
	children: ReactNode
}

interface ScreenErrorBoundaryState {
	error?: Error
}

/** Last-resort render boundary for failures outside the API boot lifecycle. */
export class ScreenErrorBoundary extends Component<ScreenErrorBoundaryProps, ScreenErrorBoundaryState> {
	state: ScreenErrorBoundaryState = {}

	static getDerivedStateFromError(error: Error): ScreenErrorBoundaryState {
		return { error }
	}

	componentDidCatch(error: Error, info: ErrorInfo) {
		console.error("Simplex UI render failed", error, info)
	}

	render() {
		if (!this.state.error) return this.props.children
		return (
			<div className="screen-error" role="alert">
				<span className="eyebrow">Something went wrong</span>
				<h1>This view could not be rendered</h1>
				<p>{this.state.error.message}</p>
				<button type="button" onClick={() => window.location.reload()}>
					Reload dashboard
				</button>
			</div>
		)
	}
}
