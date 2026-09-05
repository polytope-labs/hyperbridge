import { ChevronRightIcon } from "./InterfaceIcons"

/** One row in a list of operator tools; each opens a focused side sheet. */
export function OperationLink(props: { title: string; description: string; meta: string; onClick: () => void }) {
	return (
		<button type="button" className="operator-tool-row" onClick={props.onClick}>
			<span>
				<strong>{props.title}</strong>
				<small>{props.description}</small>
			</span>
			<em>{props.meta}</em>
			<ChevronRightIcon aria-hidden="true" />
		</button>
	)
}
