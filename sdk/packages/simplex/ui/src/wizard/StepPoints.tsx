import type { ComponentType, SVGProps } from "react"

export interface StepPoint {
	icon: ComponentType<SVGProps<SVGSVGElement>>
	title: string
	detail: string
}

/** The few short points a wizard step opens with: what it asks for and why, said once. */
export function StepPoints(props: { label: string; points: ReadonlyArray<StepPoint> }) {
	return (
		<ul className="step-points" aria-label={props.label}>
			{props.points.map(({ icon: Icon, title, detail }) => (
				<li key={title}>
					<span className="step-point-icon" aria-hidden="true">
						<Icon />
					</span>
					<div>
						<strong>{title}</strong>
						<p>{detail}</p>
					</div>
				</li>
			))}
		</ul>
	)
}
